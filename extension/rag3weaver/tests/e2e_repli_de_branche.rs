//! **Le repli par branche de la recherche hybride** (décision de Lucie,
//! 5 octobre 2026 — ticket `2026-10-05-erreur-de-la-branche-vecteur-fait-
//! tomber-la-recherche-hybride.md`) : une branche de signal qui manque ou
//! échoue ne fait pas tomber la fusion — les branches restantes répondent
//! et le repli SE DIT (« dense signal is not available » / « … failed: … »).
//! Toutes les branches tombées : une erreur, pas un résultat vide. Le mode
//! strict reste au choix de l'appelant (`SearchOptions.strict_signals`).
//!
//! Le montage du modèle absent est celui d'e2e_prise_atomique : une base
//! écrite avec `modele-a`, rouverte en lecture avec `modele-b` — la branche
//! dense n'a pas sa colonne, la branche texte a ses mots.
//!
//! Run with: ./run_e2e.sh --test e2e_repli_de_branche

#![cfg(feature = "rag3db-native")]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::search::{Consistency, SearchOptions, SearchSignals};
use rag3weaver::Rag3dbConnection;

struct EmbarqueurNomme(&'static str, usize);

impl rag3weaver::embedder::Embedder for EmbarqueurNomme {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, rag3weaver::embedder::EmbedError> {
        Ok(texts.iter().map(|_| vec![0.5; self.1]).collect())
    }
    fn dim(&self) -> usize {
        self.1
    }
    fn is_mock(&self) -> bool {
        false
    }
    fn name(&self) -> &str {
        self.0
    }
}

fn racine() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::Path::new(&manifest).parent().unwrap().parent().unwrap().to_string_lossy().to_string()
    })
}

fn catalogue_sur(dossier: &std::path::Path, embarqueur: EmbarqueurNomme) -> rag3weaver::Catalog {
    let conn = Rag3dbConnection::new(dossier).expect("ouvrir la base");
    let boxed: Box<dyn DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", racine());
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let mut config = rag3weaver::CatalogConfig::default();
    config.embedding_dim = embarqueur.1;
    let mut catalog = rag3weaver::Catalog::new(boxed, Box::new(embarqueur), config);
    catalog.initialize().expect("initialize");
    catalog
}

fn produit_entite() -> rag3weaver::config::EntityConfig {
    use rag3weaver::config::{EntityConfig, FieldType, SimpleFieldDef};
    let mut fields = HashMap::new();
    fields.insert(
        "texte".to_string(),
        SimpleFieldDef { field_type: FieldType::Text, is_title: true, is_content: true, ..Default::default() },
    );
    EntityConfig { fields, ..Default::default() }
}

fn produit_ligne(texte: &str) -> Vec<std::collections::BTreeMap<String, CypherValue>> {
    let mut d = std::collections::BTreeMap::new();
    d.insert("texte".to_string(), CypherValue::String(texte.into()));
    vec![d]
}

fn dossier_temporaire(nom: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("rag3weaver-{nom}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// Une base écrite avec `modele-a`, rouverte en lecture avec `modele-b` :
/// la branche dense est indisponible, la branche texte sait répondre.
fn lecteur_au_mauvais_modele(nom: &str) -> (std::path::PathBuf, Arc<Mutex<rag3weaver::Catalog>>) {
    let dossier = dossier_temporaire(nom);
    {
        let mut a = catalogue_sur(&dossier, EmbarqueurNomme("modele-a", 4));
        a.register_entity("Produit", produit_entite()).unwrap();
        a.ingest_entities("Produit", produit_ligne("un clavecin bien tempéré")).unwrap();
        let _ = a.execute_raw("CHECKPOINT");
        a.shutdown().unwrap();
    }
    let conn = Rag3dbConnection::read_only(&dossier).expect("ouvrir en lecture");
    let mut config = rag3weaver::CatalogConfig::default();
    config.embedding_dim = 4;
    let lecteur = Arc::new(Mutex::new(rag3weaver::Catalog::ouvrir_en_lecture(
        Box::new(conn),
        Box::new(EmbarqueurNomme("modele-b", 4)),
        config,
    )));
    lecteur.lock().unwrap().initialize().unwrap();
    (dossier, lecteur)
}

fn options(signals: SearchSignals) -> SearchOptions {
    SearchOptions {
        limit: 10,
        consistency: Consistency::Immediate,
        signals: Some(signals),
        ..Default::default()
    }
}

/// **Le constat du ticket, éprouvé** : en hybride, la branche dense
/// indisponible ne doit pas jeter les résultats des mots — ils reviennent,
/// et le repli se dit dans les warnings.
#[test]
#[ignore]
fn l_hybride_sans_modele_rend_les_mots_et_le_dit() {
    let (dossier, lecteur) = lecteur_au_mauvais_modele("repli-hybride");
    let r = rag3weaver::Catalog::rechercher(&lecteur, "Produit", "clavecin", options(SearchSignals::HYBRID))
        .expect("le repli par branche est le défaut : l'hybride répond par les mots");
    assert!(
        !r.results.is_empty(),
        "les résultats des mots ne sont pas jetés : {:?}",
        r.meta.warnings
    );
    assert!(
        r.meta.warnings.iter().any(|w| w.contains("dense signal is not available")),
        "le repli se dit — warnings : {:?}",
        r.meta.warnings
    );
    drop(lecteur);
    let _ = std::fs::remove_dir_all(&dossier);
}

/// **Toutes les branches tombées = une erreur, pas un vide** : en vecteur
/// seul, le modèle absent reste un refus qui se nomme — le contrat
/// d'e2e_prise_atomique, préservé par le repli.
#[test]
#[ignore]
fn le_vecteur_seul_sans_modele_reste_une_erreur() {
    let (dossier, lecteur) = lecteur_au_mauvais_modele("repli-vecteur-seul");
    match rag3weaver::Catalog::rechercher(&lecteur, "Produit", "clavecin", options(SearchSignals::VECTOR)) {
        Err(rag3weaver::CatalogError::EmbeddingModelUnavailable(message)) => {
            assert!(message.contains("modele_b") || message.contains("modele-b"), "{message}");
        }
        autre => panic!("toutes les branches tombées devait rester une erreur nommée — {autre:?}"),
    }
    drop(lecteur);
    let _ = std::fs::remove_dir_all(&dossier);
}

/// **Le strict au choix de l'appelant** : avec `strict_signals`, l'hybride
/// rend l'erreur d'avant — le comportement d'hier, sur demande.
#[test]
#[ignore]
fn le_strict_rend_l_erreur_comme_avant() {
    let (dossier, lecteur) = lecteur_au_mauvais_modele("repli-strict");
    let mut o = options(SearchSignals::HYBRID);
    o.strict_signals = true;
    match rag3weaver::Catalog::rechercher(&lecteur, "Produit", "clavecin", o) {
        Err(_) => {}
        Ok(r) => panic!(
            "en strict, une branche tombée est une erreur — reçu {} résultats",
            r.results.len()
        ),
    }
    drop(lecteur);
    let _ = std::fs::remove_dir_all(&dossier);
}
