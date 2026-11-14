//! E2E — le groupe (creux, relecteur, OCR) activé à la demande, par le
//! catalogue (décision de c7, 14 novembre 2026 : une activation nommée, citée
//! par chaque déclaration). Sur la fixture CPU
//! (`tests/fixtures/residence/fournisseur.py`, aucun modèle) : une recherche
//! sans relecture n'active rien ; une recherche qui relit son pool active le
//! fournisseur du groupe, qui relit.
//!
//! ./run_e2e.sh --test e2e_groupe_a_la_demande
#![cfg(all(feature = "rag3db-native", feature = "daemon", unix))]

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use rag3weaver::config::FieldType;
use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::model_source::ModelSource;
use rag3weaver::search::{BM25Mode, Consistency, RerankOptions, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

fn evenements(journal: &std::path::Path, nom: &str) -> usize {
    std::fs::read_to_string(journal).unwrap_or_default().lines()
        .filter(|l| l.contains(&format!("\"event\": \"{nom}\""))).count()
}

fn note(name: &str, body: &str) -> BTreeMap<String, CypherValue> {
    let mut d = BTreeMap::new();
    d.insert("name".into(), CypherValue::String(name.into()));
    d.insert("body".into(), CypherValue::String(body.into()));
    d
}

fn opts(rerank: Option<RerankOptions>) -> SearchOptions {
    SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        limit: 10,
        rerank,
        ..Default::default()
    }
}

#[test]
#[ignore]
fn une_recherche_sans_relecture_n_active_rien_une_recherche_relue_active_le_groupe() {
    let dossier = tempfile::tempdir().unwrap();
    let journal = dossier.path().join("events.jsonl");
    let source: ModelSource = serde_json::from_value(serde_json::json!({
        "provider": "service", "model": "fixture-relecteur", "address": "127.0.0.1:0", "timeout_secs": 5,
        "activation": {
            "nom": "groupe", "modele": "fixture-groupe",
            "programme": concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/residence/fournisseur.py"),
            "args": ["--journal", journal.display().to_string(), "--generation", "groupe-e2e", "--modele", "fixture-groupe", "--sparse"],
            "env": {"RAG3WEAVER_RERANK_MODEL": "fixture-relecteur", "RAG3WEAVER_OCR_MODEL": "fixture-ocr"},
            "profil": "fixture-cpu", "peripherique": "cpu",
            "coordination": dossier.path().join("coordination").display().to_string(),
            "journal": dossier.path().join("journaux").display().to_string(),
            "budget_activation_secs": 10, "budget_attente_secs": 10,
            "residence": {"a_demande": {"inactivite_secs": 2, "rendre_le_bail_secs": 1}},
        },
    })).expect("la déclaration du relecteur à la demande");
    let (relecteur, _) = rag3weaver::model_source::connect_reranker(&source).expect("le relecteur à la demande");
    assert_eq!(evenements(&journal, "constructeur_demarre"), 0, "ouvrir ne lance rien");

    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let boxed: Box<dyn DbConnection> = Box::new(conn);
    boxed.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).expect("extension vecteur");
    let config = CatalogConfig { name: Some("groupe-a-la-demande".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().expect("initialize");
    let mut fields = HashMap::new();
    fields.insert("name".into(), SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() });
    fields.insert("body".into(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() });
    catalog.register_entity("Note", EntityConfig { fields, signals: SearchSignals::BM25, ..Default::default() }).unwrap();
    catalog.set_reranker(relecteur);
    catalog.ingest_entities("Note", vec![
        note("A", "scheduler scheduler scheduler notes"),
        note("B", "the scheduler handles preemption"),
        note("C", "scheduler overview"),
    ]).unwrap();
    let partage = Arc::new(Mutex::new(catalog));

    // Une recherche par les mots, sans relecture : le relecteur n'est pas appelé.
    let r = Catalog::rechercher(&partage, "Note", "scheduler preemption", opts(None)).expect("recherche sans relecture");
    assert_eq!(r.results.len(), 3);
    assert_eq!(r.meta.reranked_count, 0);
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert_eq!(evenements(&journal, "constructeur_demarre"), 0, "une recherche sans relecture n'active rien");

    // Une recherche qui relit son pool : le groupe se lance et relit.
    let r = Catalog::rechercher(&partage, "Note", "scheduler preemption", opts(Some(RerankOptions::default()))).expect("recherche relue");
    assert!(r.meta.reranked_count > 0, "le pool est relu : {:?}", r.meta.warnings);
    assert_eq!(evenements(&journal, "constructeur_demarre"), 1, "la relecture a lancé le groupe");

    // Le fournisseur relancé est tué ici, pas laissé au poste.
    if let Ok(texte) = std::fs::read_to_string(&journal) {
        let lignes: Vec<serde_json::Value> = texte.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
        for pid in lignes.iter().filter_map(|v| v["pid"].as_i64()).collect::<std::collections::BTreeSet<_>>() {
            let arrete = lignes.iter().any(|v| v["pid"].as_i64() == Some(pid) && v["event"] == "arrete");
            let notre = std::fs::read(format!("/proc/{pid}/cmdline")).map(|c| String::from_utf8_lossy(&c).contains("fournisseur.py")).unwrap_or(false);
            if !arrete && notre {
                unsafe { libc::kill(pid as i32, libc::SIGTERM) };
            }
        }
    }
}
