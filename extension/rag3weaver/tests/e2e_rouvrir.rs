//! E2E : **la base doit être rouverte** après un point de reprise échoué
//! (`REOPEN_AFTER_FAILED_CHECKPOINT`, 3 octobre 2026).
//!
//! Le moteur, après un point de reprise qui échoue, refuse tout jusqu'à la
//! réouverture, sous un nom stable. rag3weaver le reconnaît en un seul point
//! (`Rag3dbConnection`), empoisonne la base pour toutes ses connexions, et le
//! catalogue refuse alors chaque verbe en disant pourquoi. Son propriétaire le
//! lâche et en rouvre un neuf : ce qui était validé est là, le reste se repose.
//!
//! La panne est injectée par le crochet de test de la connexion, qui répond
//! exactement comme le moteur ; la vraie panne est éprouvée côté C++.
//!
//! Run with: ./run_e2e.sh --test e2e_rouvrir
#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use rag3weaver::config::FieldType;
use rag3weaver::connection::{CypherValue, DbConnection, REOPEN_AFTER_FAILED_CHECKPOINT};
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, CatalogError, EntityConfig, Rag3dbConnection, SimpleFieldDef};

fn note() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert(
        "texte".to_string(),
        SimpleFieldDef { field_type: FieldType::Text, is_title: true, is_content: true, ..Default::default() },
    );
    EntityConfig { fields, signals: SearchSignals::BM25, ..Default::default() }
}

fn ligne(texte: &str) -> BTreeMap<String, CypherValue> {
    BTreeMap::from([("texte".to_string(), CypherValue::String(texte.into()))])
}

fn ouvrir(dossier: &std::path::Path) -> (Catalog, rag3weaver::FailedCheckpointHook) {
    let conn = Rag3dbConnection::new(dossier).expect("ouvrir la base");
    let hook = conn.failed_checkpoint_hook();
    let config = CatalogConfig { name: Some("rouvrir".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    catalog.register_entity("Note", note()).unwrap();
    (catalog, hook)
}

fn textes(catalog: &Catalog) -> Vec<String> {
    let mut v: Vec<String> = catalog
        .execute_raw("MATCH (n:Note) RETURN n.texte")
        .unwrap()
        .rows
        .into_iter()
        .filter_map(|r| r.first().and_then(|v| v.as_str()).map(str::to_string))
        .collect();
    v.sort();
    v
}

#[test]
#[ignore]
fn apres_un_point_de_reprise_echoue_le_catalogue_refuse_tout_et_se_rouvre() {
    let dossier = std::env::temp_dir().join(format!("rag3weaver-rouvrir-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dossier);
    {
        let (mut catalog, hook) = ouvrir(&dossier);
        catalog.ingest_entities("Note", vec![ligne("validée avant la panne")]).unwrap();
        assert!(catalog.must_reopen().is_none());

        // Une opération en file, pas encore drainée : elle sera perdue avec
        // le catalogue, et l'erreur le dit.
        catalog.create_jusqu_a("Note", ligne("en file"), Disponibilites::AUCUNE).unwrap();

        // La panne : la prochaine instruction rencontre le refus du moteur.
        hook.after(0);
        let err = catalog.ingest_entities("Note", vec![ligne("pendant la panne")]).unwrap_err().to_string();
        assert!(err.contains(REOPEN_AFTER_FAILED_CHECKPOINT), "l'erreur porte le nom du moteur : {err}");

        // Le catalogue est empoisonné : chaque verbe refuse, en le disant,
        // sans plus rien envoyer au moteur.
        let raison = catalog.must_reopen().expect("le catalogue sait qu'il doit être rouvert");
        assert!(raison.contains(REOPEN_AFTER_FAILED_CHECKPOINT), "{raison}");
        assert!(raison.contains("1 opération(s) en file"), "la file perdue est comptée : {raison}");
        match catalog.ingest_entities("Note", vec![ligne("après la panne")]) {
            Err(CatalogError::MustReopen(m)) => assert!(m.contains(REOPEN_AFTER_FAILED_CHECKPOINT), "{m}"),
            autre => panic!("un verbe après la panne doit refuser par MustReopen : {autre:?}"),
        }
        assert!(matches!(catalog.get_many("Note", &["x".into()]), Err(CatalogError::MustReopen(_))));
        let brut = catalog.execute_raw("MATCH (n:Note) RETURN count(n)").unwrap_err().to_string();
        assert!(brut.contains(REOPEN_AFTER_FAILED_CHECKPOINT), "la connexion elle-même refuse : {brut}");
        // Le catalogue est lâché ici, et la base fermée avec lui.
    }

    // Rouvert : ce qui était validé est là, ce qui ne l'était pas non.
    let (mut catalog, _) = ouvrir(&dossier);
    assert!(catalog.must_reopen().is_none(), "une base rouverte repart saine");
    assert_eq!(textes(&catalog), ["validée avant la panne"]);
    // Ce qui a échoué se repose, par son appelant.
    catalog.ingest_entities("Note", vec![ligne("pendant la panne")]).unwrap();
    assert_eq!(textes(&catalog), ["pendant la panne", "validée avant la panne"]);
    drop(catalog);
    let _ = std::fs::remove_dir_all(&dossier);
}

/// **La connexion sœur** (celle du magasin de blobs) partage l'état : la base
/// est empoisonnée, pas une connexion.
#[test]
#[ignore]
fn la_connexion_soeur_partage_l_empoisonnement() {
    let conn = Rag3dbConnection::in_memory().unwrap();
    let soeur = conn.create_sync_connection().unwrap();
    conn.failed_checkpoint_hook().after(0);
    assert!(conn.execute("RETURN 1").is_err());
    assert!(conn.must_reopen().is_some());
    assert!(soeur.must_reopen().is_some(), "la sœur sait aussi");
    let err = soeur.execute("RETURN 1").unwrap_err().to_string();
    assert!(err.contains(REOPEN_AFTER_FAILED_CHECKPOINT), "{err}");
}
