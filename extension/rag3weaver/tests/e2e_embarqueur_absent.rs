//! **Sans service d'embarquement, le produit tourne en plein texte seul**
//! (10 octobre 2026).
//!
//! Un backend sans service montait un `MockEmbedder` : des vecteurs nuls
//! écrits comme de vrais vecteurs, une recherche dense qui répondait sur
//! eux, et un débit sondé sur du vide. L'[`AbsentEmbedder`] porte le nom et
//! la dimension du modèle attendu, et n'embarque jamais :
//! - une synchronisation qui exige tout écrit le plein texte, pas un vecteur,
//!   et laisse la dette, lisible par l'état de l'index ;
//! - le mot du repli est dit une fois, et l'avertissement nommé au démarrage ;
//! - le rattrapage rend 0 sans toucher à l'index ;
//! - aucun débit n'est sondé ni noté ;
//! - la recherche hybride répond par les mots, et dit la branche dense
//!   « not available ».
//!
//! ```bash
//! ./run_e2e.sh --test e2e_embarqueur_absent
//! ```
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::sync::{Arc, Mutex};

use rag3weaver::code::{default_scope_chunking, register_code_schema, SCOPE};
use rag3weaver::code_sync::{sync_source, SourceSyncOptions};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::connection::DbConnection;
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::events::CatalogEvent;
use rag3weaver::search::{SearchOptions, SearchSignals};
use rag3weaver::{AbsentEmbedder, Catalog, CatalogConfig, Rag3dbConnection, AVERTISSEMENT_EMBARQUEUR_ABSENT};

const MODELE: &str = "granite-278m";
const DIM: usize = 32;

fn corpus() -> Vec<(String, String)> {
    (0..40)
        .map(|i| (format!("src/f{i:03}.rs"), format!("pub fn calcule_{i}() -> u32 {{ {i} }}\n")))
        .collect()
}

fn avertissements(rx: &mut async_broadcast::Receiver<CatalogEvent>) -> Vec<String> {
    let mut dits = Vec::new();
    while let Ok(e) = rx.try_recv() {
        if let CatalogEvent::Warning { message, .. } = e {
            dits.push(message);
        }
    }
    dits
}

#[test]
#[ignore]
fn sans_service_l_index_est_en_plein_texte_et_la_dette_se_lit() {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension()))
        .expect("extension vecteur");
    let config = CatalogConfig { name: Some("embarqueur-absent".into()), embedding_dim: DIM, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(AbsentEmbedder::new(MODELE, DIM)), config);
    let mut rx = catalog.subscribe();
    catalog.initialize().expect("initialize");
    let dits = avertissements(&mut rx);
    assert!(
        dits.iter().any(|m| m.starts_with(AVERTISSEMENT_EMBARQUEUR_ABSENT) && m.contains(MODELE)),
        "l'avertissement nommé du démarrage : {dits:?}"
    );
    register_code_schema(&mut catalog, default_scope_chunking()).expect("schéma du code");

    // 1. Une synchronisation qui exige tout : le plein texte, pas un vecteur.
    let options = SourceSyncOptions { batch_files: 16, exige: Disponibilites::TOUT, ..Default::default() };
    let r = sync_source(&mut catalog, &Snapshot::new("depot", corpus()), &options, &mut |_| {}).expect("synchroniser");
    assert_eq!(r.failed, 0, "{r:?}");
    let dits = avertissements(&mut rx);
    let repli: Vec<&String> = dits.iter().filter(|m| m.contains("se replie sur le plein texte")).collect();
    assert_eq!(repli.len(), 1, "le mot du repli, dit une fois : {dits:?}");

    // 2. La dette se lit, contre le modèle attendu.
    let progres = catalog.index_progress_for(SCOPE).expect("avancement");
    println!("▸ modèle {} : {} morceaux, {} sans vecteur", progres.model, progres.chunks(), progres.dense_missing());
    assert!(progres.chunks() > 0, "des morceaux ont été écrits");
    assert_eq!(progres.dense_missing(), progres.chunks(), "aucun vecteur écrit : toute la dette reste");

    // 3. Le rattrapage ne rattrape rien, et aucun débit n'est sondé.
    assert_eq!(catalog.embarquer_le_retard(Disponibilites::TOUT, 512, None).expect("rattrapage"), 0);
    assert_eq!(catalog.index_progress_for(SCOPE).unwrap().dense_missing(), progres.chunks(), "la dette est intacte");
    assert!(catalog.probe_embedding_rate(&["fn x() {}".to_string()]).expect("sonde").is_none(), "pas de débit");
    assert!(catalog.known_embedding_rate().expect("débit").is_none(), "aucun débit noté");

    // 4. La recherche hybride répond par les mots, la branche dense se dit.
    let partage = Arc::new(Mutex::new(catalog));
    let r = Catalog::rechercher(&partage, SCOPE, "calcule_7", SearchOptions {
        signals: Some(SearchSignals::BM25 | SearchSignals::VECTOR),
        ..Default::default()
    })
    .expect("recherche hybride");
    println!("▸ {} résultats ; avertissements : {:?}", r.results.len(), r.meta.warnings);
    assert!(r.results.iter().any(|x| format!("{x:?}").contains("f007.rs")), "les mots trouvent calcule_7");
    assert!(
        r.meta.warnings.iter().any(|w| w.contains("signal is not available") && w.contains(AVERTISSEMENT_EMBARQUEUR_ABSENT)),
        "la branche dense dit « not available » : {:?}",
        r.meta.warnings
    );
}

/// Un embarqueur de témoin au nom et à la dimension du modèle attendu.
struct Regle;

impl rag3weaver::Embedder for Regle {
    fn embed(&self, textes: &[String]) -> Result<Vec<Vec<f32>>, rag3weaver::EmbedError> {
        Ok(textes.iter().map(|t| {
            let mut v = vec![0.0f32; DIM];
            v[t.len() % DIM] = 1.0;
            v
        }).collect())
    }
    fn dim(&self) -> usize {
        DIM
    }
    fn name(&self) -> &str {
        MODELE
    }
}

/// **Un index vectoriel retiré le temps d'un rattrapage revient dès la dette
/// soldée, par la montre** (hôte de Lucie, 25 octobre 2026). Au-delà du seuil,
/// la première passe retire l'index ; la montre fait des passes de 512 (une
/// par lot) ; celle qui solde la dette doit le rebâtir. Avant, seule une passe
/// qui *commençait* sans dette le rebâtissait : la montre ne le faisait
/// jamais, et la recherche dense ne répondait plus jusqu'à un `index` manuel.
#[test]
#[ignore]
fn un_index_vectoriel_retire_revient_des_la_dette_soldee_par_la_montre() {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension()))
        .expect("extension vecteur");
    let config = CatalogConfig { name: Some("index-retire".into()), embedding_dim: DIM, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(AbsentEmbedder::new(MODELE, DIM)), config);
    catalog.initialize().expect("initialize");
    register_code_schema(&mut catalog, default_scope_chunking()).expect("schéma du code");

    // Une dette au-delà du seuil (1500 morceaux en dimension 32), sans service.
    let corpus: Vec<(String, String)> = (0..1700)
        .map(|i| (format!("src/m{i:04}.rs"), format!("pub fn mesure_{i}() -> u32 {{ {i} }}\n")))
        .collect();
    let options = SourceSyncOptions { batch_files: 256, exige: Disponibilites::TOUT, ..Default::default() };
    let r = sync_source(&mut catalog, &Snapshot::new("depot", corpus), &options, &mut |_| {}).expect("synchroniser");
    assert_eq!(r.failed, 0, "{r:?}");
    let dette = catalog.index_progress_for(SCOPE).unwrap().dense_missing();
    println!("▸ dette : {dette} morceaux");
    assert!(dette > 1500, "la dette doit dépasser le seuil pour que l'index tombe : {dette}");

    // Le service est là ; la montre rattrape, une passe de 512 par lot.
    catalog.set_embedder(Arc::new(Regle));
    let partage = Arc::new(Mutex::new(catalog));
    let mut retire_vu = false;
    for passe in 1..=20 {
        Catalog::embarquer_le_retard_hors_verrou(&partage, Disponibilites::TOUT, 512, None).expect("passe de la montre");
        let cat = partage.lock().unwrap();
        let reste = cat.index_progress_for(SCOPE).unwrap().dense_missing();
        let retires = cat.index_vectoriels_retires().expect("drapeaux");
        println!("▸ passe {passe} : {reste} sans vecteur, index retirés : {retires:?}");
        retire_vu |= !retires.is_empty();
        if reste == 0 {
            assert!(retire_vu, "l'index est bien tombé pendant le rattrapage");
            assert!(retires.is_empty(), "la passe qui solde la dette rebâtit l'index : {retires:?}");
            break;
        }
        assert!(passe < 20, "la dette ne se solde pas : {reste}");
    }

    // La recherche dense répond de nouveau.
    let r = Catalog::rechercher(&partage, SCOPE, "mesure_7", SearchOptions {
        signals: Some(SearchSignals::VECTOR),
        ..Default::default()
    })
    .expect("recherche dense");
    println!("▸ {} résultats ; avertissements : {:?}", r.results.len(), r.meta.warnings);
    assert!(!r.results.is_empty(), "la recherche dense répond : {:?}", r.meta.warnings);
}
