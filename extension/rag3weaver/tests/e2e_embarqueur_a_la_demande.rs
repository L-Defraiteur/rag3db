//! **L'embarqueur activé à la demande, par le catalogue** (ticket
//! `docs/embarquement/tickets/2026-11-13-un-fournisseur-a-la-demande-ouvert-par-l-hote-ne-s-evince-jamais.md`).
//!
//! Une déclaration `Residence::ADemande` sur le fournisseur factice des témoins
//! (`tests/fixtures/residence/fournisseur.py`, CPU, aucun modèle) : un index le
//! lance ; sans usage il s'évince ; une recherche par les mots seuls ne le
//! relance pas ; une recherche qui embarque le relance.
//!
//! ```bash
//! ./run_e2e.sh --test e2e_embarqueur_a_la_demande
//! ```
#![cfg(all(feature = "rag3db-native", feature = "code", feature = "daemon", unix))]

use std::sync::{Arc, Mutex};

use rag3weaver::code::{default_scope_chunking, register_code_schema, SCOPE};
use rag3weaver::code_sync::{sync_source, SourceSyncOptions};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::connection::DbConnection;
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::model_source::activation::{Activation, Residence};
use rag3weaver::model_source::{connect_embedder, Addresses, ModelSource};
use rag3weaver::search::{SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn evenements(journal: &std::path::Path, nom: &str) -> usize {
    std::fs::read_to_string(journal).unwrap_or_default().lines().filter(|l| l.contains(&format!("\"event\": \"{nom}\""))).count()
}

fn attendre(journal: &std::path::Path, nom: &str, combien: usize, secondes: u64) -> bool {
    let debut = std::time::Instant::now();
    while debut.elapsed() < std::time::Duration::from_secs(secondes) {
        if evenements(journal, nom) >= combien {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    false
}

#[test]
#[ignore]
fn une_lecture_par_les_mots_n_active_rien_une_lecture_dense_active() {
    let dossier = tempfile::tempdir().unwrap();
    let journal = dossier.path().join("events.jsonl");
    let mut source = ModelSource::service("fixture-residence-384");
    source.address = Addresses::parse("127.0.0.1:0");
    source.dimensions = Some(384);
    source.timeout_secs = Some(5);
    source.activation = Some(Activation {
        programme: concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/residence/fournisseur.py").into(),
        args: vec!["--journal".into(), journal.display().to_string(), "--generation".into(), "a-la-demande".into()],
        env: Default::default(),
        profil: "fixture-cpu".into(),
        peripherique: "cpu".into(),
        coordination: dossier.path().join("coordination"),
        journal: dossier.path().join("journaux"),
        budget_activation_secs: 10,
        budget_attente_secs: 10,
        residence: Residence::ADemande { inactivite_secs: 2, rendre_le_bail_secs: 1 },
        hote: None,
        adresse_distante: None,
        ssh: None,
    });
    let (embarqueur, _) = connect_embedder(&source).expect("embarqueur à la demande");
    assert_eq!(evenements(&journal, "constructeur_demarre"), 0, "ouvrir ne lance rien");

    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).expect("extension vecteur");
    let config = CatalogConfig { name: Some("a-la-demande".into()), embedding_dim: 384, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), embarqueur, config);
    catalog.initialize().expect("initialize");
    register_code_schema(&mut catalog, default_scope_chunking()).expect("schéma du code");
    assert_eq!(evenements(&journal, "constructeur_demarre"), 0, "ni l'initialisation ni le schéma ne lancent le fournisseur");

    // Un index qui exige les vecteurs : il lance le fournisseur.
    let corpus: Vec<(String, String)> = (0..12).map(|i| (format!("src/f{i:02}.rs"), format!("pub fn calcule_{i}() -> u32 {{ {i} }}\n"))).collect();
    let options = SourceSyncOptions { batch_files: 6, exige: Disponibilites::TOUT, ..Default::default() };
    let r = sync_source(&mut catalog, &Snapshot::new("depot", corpus), &options, &mut |_| {}).expect("synchroniser");
    assert_eq!(r.failed, 0, "{r:?}");
    assert_eq!(evenements(&journal, "constructeur_demarre"), 1, "l'index a lancé le fournisseur");

    // Sans usage, il s'évince.
    assert!(attendre(&journal, "evince", 1, 15), "évincé après l'index et son délai");
    assert!(attendre(&journal, "arrete", 1, 5));
    let partage = Arc::new(Mutex::new(catalog));

    // Une lecture par les mots seuls n'a pas besoin d'embarquer : rien ne se relance.
    let mots = Catalog::rechercher(&partage, SCOPE, "calcule_7", SearchOptions { signals: Some(SearchSignals::BM25), ..Default::default() })
        .expect("recherche par les mots");
    assert!(mots.results.iter().any(|x| format!("{x:?}").contains("f07.rs")), "les mots trouvent calcule_7 : {:?}", mots.meta.warnings);
    std::thread::sleep(std::time::Duration::from_millis(500));
    assert_eq!(evenements(&journal, "constructeur_demarre"), 1, "une lecture par les mots ne relance pas le fournisseur");

    // Une lecture dense embarque la requête : le fournisseur se relance.
    let dense = Catalog::rechercher(&partage, SCOPE, "calcule_7", SearchOptions { signals: Some(SearchSignals::VECTOR), ..Default::default() })
        .expect("recherche dense");
    assert_eq!(evenements(&journal, "constructeur_demarre"), 2, "une lecture dense relance le fournisseur");
    assert!(dense.meta.warnings.iter().all(|w| !w.contains("not available")), "la branche dense répond : {:?}", dense.meta.warnings);
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
