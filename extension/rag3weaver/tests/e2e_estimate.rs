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

/// Les fichiers **suivis par git**, avec leur taille. Le dossier d'un dépôt
/// en travail contient bien plus que le dépôt — des builds, des données, des
/// `target/` : mesuré le 3 octobre 2026 sur celui-ci, 307 541 fichiers sur le
/// disque contre 6 110 suivis. Ce que l'estimation compte doit être ce que
/// l'indexation prendra ; tant que la source « dépôt git » n'existe pas, ce
/// test fait la différence lui-même pour montrer les deux.
fn tracked_files(root: &std::path::Path) -> Vec<(String, u64)> {
    let out = std::process::Command::new("git").arg("-C").arg(root).args(["ls-files", "-z"]).output().expect("git ls-files");
    String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        .filter_map(|p| std::fs::symlink_metadata(root.join(p)).ok().filter(|m| m.is_file()).map(|m| (p.to_string(), m.len())))
        .collect()
}

#[test]
#[ignore]
fn ce_depot_s_estime_avant_de_s_indexer() {
    let root = repository_root();
    let t = std::time::Instant::now();
    let on_disk = working_tree_files(&WorkingTree::new(&root)).expect("liste du dossier");
    let whole = estimate_here(&on_disk, code_policy, None, true);
    eprintln!("[estimate] le dossier tel quel : {} fichiers listés en {:?}, sans en lire un seul", on_disk.len(), t.elapsed());
    eprintln!("[estimate]   → {} retenus, {:.0} Mo", whole.survey.files, whole.survey.bytes as f64 / 1e6);
    let files = tracked_files(&root);
    eprintln!("[estimate] les fichiers suivis par git : {}", files.len());
    assert!(files.len() < on_disk.len(), "un dossier de travail contient plus que le dépôt");
    let samples = samples();

    let embedders: [(&str, Arc<dyn Embedder>); 2] =
        [("granite-278m", common::burn::GRANITE_278M.clone()), ("granite-107m", common::burn::GRANITE_107M.clone())];
    for (name, embedder) in embedders {
        let rate = probe_rate(embedder.as_ref(), &samples).expect("sonde");
        let estimate = estimate_here(&files, code_policy, rate, embedder.distant());
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
