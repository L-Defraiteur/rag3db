//! **Sonde : ce que le relecteur change sur de vraies requêtes** (pièce 2 de la
//! recherche de l'IDE, docs/12-octobre-2026-recherche/06).
//!
//! Pas un test : une sonde, qui ne juge rien et imprime ce qu'elle trouve. Elle
//! existe pour que Lucie tranche le défaut du reclassement sur des chiffres :
//! sur chaque question, l'ordre des cinq premiers résultats de `search_code`
//! sans relecture et avec, et la latence des deux (médiane de trois appels).
//!
//! ```sh
//! cp -a --reflink=always ~/.local/share/rag3weaver/code-lourd.rag3db* <copie>/
//! SONDE_CONFIG=~/.config/rag3weaver/code-lourd SONDE_BASE=<copie>/code-lourd.rag3db \
//!   scripts/cargo_magasin.sh test --features rag3db-native,code,daemon \
//!   --test sonde_reclassement -- --include-ignored --nocapture
//! ```
//!
//! **Sur une copie, jamais sur l'originale** : l'ouverture écrit (rejeu,
//! estampille). Le manifeste est celui de cette branche ; de la config de
//! l'hôte, il ne reprend que ce qui désigne sa base. Il y gagne
//! `models.rerank`, le relecteur du démon d'embarquement (`SONDE_RELECTEUR`,
//! 127.0.0.1:7879 par défaut), et pointe la copie de la base.

#![cfg(all(feature = "rag3db-native", feature = "code", feature = "daemon"))]

use std::path::{Path, PathBuf};
use std::time::Instant;

use rag3weaver::backend::PreparedBackend;
use rag3weaver::Rag3dbConnection;
use serde_json::{json, Value};

/// Les questions : celles qu'un agent pose sur ce dépôt, en langue naturelle
/// (le relecteur ne sert à rien sur un identifiant).
const QUESTIONS: &[&str] = &[
    "où un run par niveaux vérifie-t-il qu'il a été annulé",
    "comment la recherche se replie quand l'index vectoriel est retiré",
    "qui borne le nombre de runs lancés en fond en même temps",
    "comment le point de reprise écrit les pages modifiées sur le disque",
    "où le binder refuse un index qui n'existe pas",
    "comment la montre réindexe un fichier modifié",
    "how does the hybrid search fuse bm25 and vector scores",
    "where is the read-only snapshot of a tool call opened",
];

fn copier(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for e in std::fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        let cible = dst.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copier(&e.path(), &cible);
        } else {
            std::fs::copy(e.path(), &cible).unwrap();
        }
    }
}

fn mediane(mut v: Vec<u128>) -> u128 {
    v.sort();
    v[v.len() / 2]
}

/// Les cinq premiers, en `fichier:nom`.
fn tete(r: &Value) -> Vec<String> {
    r["result"]
        .as_array()
        .map(|v| {
            v.iter()
                .take(5)
                .map(|x| format!("{}:{}", x["data"]["file_path"].as_str().unwrap_or("?"), x["data"]["name"].as_str().unwrap_or("?")))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
#[ignore = "sonde : demande SONDE_CONFIG et SONDE_BASE (une copie), et le relecteur du démon"]
fn ce_que_le_relecteur_change_sur_de_vraies_requetes() {
    let config = PathBuf::from(std::env::var("SONDE_CONFIG").expect("SONDE_CONFIG : le dossier de config du backend"));
    let base = PathBuf::from(std::env::var("SONDE_BASE").expect("SONDE_BASE : une copie de sa base"));
    let relecteur = std::env::var("SONDE_RELECTEUR").unwrap_or_else(|_| "127.0.0.1:7879".to_string());

    // **Les gabarits de cette branche**, pas ceux de la config de l'hôte (une
    // copie d'avant : sans relecteur dans le graphe ni port `metadata`). De
    // l'hôte, seulement ce qui désigne sa base : nom, espace de travail,
    // embarquement, extension.
    let tmp = tempfile::tempdir().unwrap();
    let gabarits = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    copier(&gabarits.join("tools"), &tmp.path().join("templates/tools"));
    copier(&gabarits.join("backends/code"), &tmp.path().join("templates/backends/code"));
    let hote: Value = serde_json::from_slice(&std::fs::read(config.join("templates/backends/code/backend.json")).unwrap()).unwrap();
    let manifeste = tmp.path().join("templates/backends/code/backend.json");
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    for cle in ["name", "workspace", "embeddings", "vector_extension"] {
        if !hote[cle].is_null() {
            m[cle] = hote[cle].clone();
        }
    }
    m["workspace"]["commands"] = json!("off");
    // La sonde ne fait que chercher : les outils qui écrivent partent (un
    // espace de travail en lecture seule les refuse au chargement).
    if let Some(outils) = m["tools"].as_object_mut() {
        outils.retain(|_, t| t["policy"]["write_files"] != json!(true) && t["policy"]["run_commands"] != json!(true));
    }
    m["database"] = json!(base.to_string_lossy());
    m["models"]["rerank"] = json!({"provider": "service", "model": "bge-reranker-v2-m3", "address": relecteur});
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();

    let prepared = PreparedBackend::load(&manifeste).expect("le manifeste charge");
    let (embedder, _) = prepared.connect_embedder().expect("le service d'embarquement du manifeste");
    let backend = prepared
        .open(Box::new(Rag3dbConnection::new(&base).expect("la copie de la base")), Some(embedder))
        .expect("l'ouverture");

    let appel = |q: &str, options: Value| -> (Value, u128) {
        let mut temps = Vec::new();
        let mut dernier = Value::Null;
        for _ in 0..3 {
            let t = Instant::now();
            dernier = backend.call("search_code", json!({"query": q, "options": options, "mode": "indexed"})).expect("la recherche");
            temps.push(t.elapsed().as_millis());
        }
        (dernier, mediane(temps))
    };

    let mut changees = 0;
    for q in QUESTIONS {
        let (sans, ms_sans) = appel(q, json!({}));
        let (avec, ms_avec) = appel(q, json!({"rerank": {"candidates": 20}}));
        let (a, b) = (tete(&sans), tete(&avec));
        if a != b {
            changees += 1;
        }
        println!("\n## {q}");
        println!("latence : fusion {ms_sans} ms, avec relecture {ms_avec} ms (+{} ms)", ms_avec as i128 - ms_sans as i128);
        println!("relus : {}", avec["metadata"]["render.meta"]["rerankedCount"]);
        let avertis = avec["metadata"]["render.meta"]["warnings"].to_string();
        if avertis != "null" {
            println!("avertissements : {avertis}");
        }
        for i in 0..5 {
            println!(
                "  {}. {:<60} | {}",
                i + 1,
                a.get(i).map(String::as_str).unwrap_or("-"),
                b.get(i).map(String::as_str).unwrap_or("-")
            );
        }
    }
    println!("\nRÉSUMÉ : {changees} questions sur {} ont un autre ordre des cinq premiers avec la relecture", QUESTIONS.len());
}
