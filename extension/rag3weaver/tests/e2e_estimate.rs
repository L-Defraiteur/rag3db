//! **Estimer ce dépôt avant de l'indexer** — le verbe `estimate`, sur le vrai
//! dépôt et un vrai embarqueur.
//!
//! Compte le dépôt entier par la politique d'ingestion, sonde le débit de
//! l'embarqueur attaché (le service de `RAG3WEAVER_EMBED_SERVICE` s'il est
//! posé, la carte d'ici sinon), et imprime ce qu'un agent lirait. N'écrit
//! rien, n'ouvre aucune base.
//!
//! Run with: ./run_e2e.sh --test e2e_estimate
#![cfg(all(feature = "rag3db-native", feature = "burn-embedder", feature = "code"))]

mod common;

use std::sync::Arc;

use rag3weaver::code_tools::WorkingTree;
use rag3weaver::embedder::Embedder;
use rag3weaver::estimate::{code_policy, estimate_here, probe_rate, working_tree_files};

fn repository_root() -> std::path::PathBuf {
    std::env::var("RAG3DB_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

/// Une soixantaine de morceaux de 1 200 caractères pris dans la crate : la
/// matière d'une ingestion.
fn samples() -> Vec<String> {
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    for name in ["catalog.rs", "code.rs", "search.rs"] {
        let text = std::fs::read_to_string(src.join(name)).expect("source de la crate");
        let chars: Vec<char> = text.chars().collect();
        out.extend(chars.chunks(1_200).take(22).map(|c| c.iter().collect::<String>()));
    }
    out
}

#[test]
#[ignore]
fn ce_depot_s_estime_avant_de_s_indexer() {
    let tree = WorkingTree::new(repository_root());
    let t = std::time::Instant::now();
    let (files, excluded) = working_tree_files(&tree).expect("liste du dépôt");
    eprintln!("[estimate] {} fichiers listés en {:?}, sans en lire un seul ; {} écartés par la source", files.len(), t.elapsed(), excluded.len());
    // Le dossier pris tel quel, ignorés compris, n'est pas le dépôt : 307 541
    // fichiers le 3 octobre 2026, dont 278 759 retenus (2,46 Go).
    assert!(files.len() < 50_000, "la source respecte les règles d'exclusion du dossier : {}", files.len());
    let samples = samples();

    let embedders: [(&str, Arc<dyn Embedder>); 2] =
        [("granite-278m", common::burn::GRANITE_278M.clone()), ("granite-107m", common::burn::GRANITE_107M.clone())];
    for (name, embedder) in embedders {
        let rate = probe_rate(embedder.as_ref(), &samples).expect("sonde");
        let estimate = estimate_here(&files, &excluded, code_policy, rate, embedder.distant());
        eprintln!(
            "[estimate] sonde {name} ({}) : {:.0} caractères/s",
            if embedder.distant() { "service" } else { "carte d'ici" },
            rate.map(|r| r.chars_per_second).unwrap_or(0.0)
        );
        eprintln!("[estimate] {}", estimate.text().replace('\n', "\n[estimate] "));

        assert!(estimate.survey.files > 1_000, "le dépôt a plus de mille fichiers retenus : {:?}", estimate.survey);
        assert!(estimate.survey.skipped_files() > 0, "et des données qu'on écarte");
        assert!(estimate.vectors_seconds.is_some(), "une sonde qui répond donne une durée");
        assert!(!estimate.model_reason.is_empty());
    }
}
