//! **Sonde : la recherche contre lucivy seul, sur les requêtes de Lucie** (25 octobre 2026, le
//! lot de la requête entière, docs/12-octobre-2026-recherche/06).
//!
//! Pas un test : une sonde, qui ne juge rien et imprime une table. Pour chaque requête, l'oracle
//! est lucivy interrogé directement, comme la démo WASM : un seul `contains` relâché de la
//! requête entière, sans découpage. La table dit, pour les premiers morceaux que l'oracle trouve,
//! leur rang dans `search_code` (après fusion) et leur rang dans son signal plein texte.
//!
//! ```sh
//! cp -a --reflink=always ~/.cache/rag3db-locks/maintenant-21h26 <copie>
//! SONDE_CONFIG=~/.config/rag3weaver/code-rag3db SONDE_BASE=<copie>/code-rag3db-depot.rag3db \
//!   scripts/cargo_magasin.sh test --features rag3db-native,code,daemon \
//!   --test sonde_requete_entiere -- --include-ignored --nocapture
//! ```
//!
//! **Sur une copie, jamais sur l'originale** : l'ouverture écrit. Le manifeste est celui de cette
//! branche ; de la config de l'hôte, il ne reprend que ce qui désigne sa base.

#![cfg(all(feature = "rag3db-native", feature = "code", feature = "daemon"))]

use std::path::{Path, PathBuf};

use rag3weaver::backend::PreparedBackend;
use rag3weaver::search::{build_bm25_query, BM25Mode};
use rag3weaver::Rag3dbConnection;
use serde_json::{json, Value};

/// Les requêtes de Lucie du 25 octobre au soir (ses runs journalisés), plus la forme corrigée de
/// sa faute de frappe.
const REQUETES: &[&str] = &[
    "rag3weaver",
    "work in progress",
    "refreh index states",
    "refresh index states",
    "l'indexation",
    "Vue d'ensemble",
    "cpu_count",
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

/// L'identité d'un résultat : fichier, nom, ligne.
fn cle(x: &Value) -> (String, String, i64) {
    let d = &x["data"];
    (
        d["file_path"].as_str().unwrap_or("?").to_string(),
        d["name"].as_str().unwrap_or("?").to_string(),
        d["start_line"].as_i64().unwrap_or(-1),
    )
}

#[test]
#[ignore = "sonde : demande SONDE_CONFIG et SONDE_BASE (une copie)"]
fn la_recherche_contre_lucivy_seul_sur_les_requetes_de_lucie() {
    let config = PathBuf::from(std::env::var("SONDE_CONFIG").expect("SONDE_CONFIG : le dossier de config du backend"));
    let base = PathBuf::from(std::env::var("SONDE_BASE").expect("SONDE_BASE : une copie de sa base"));

    // Les gabarits de cette branche ; de l'hôte, ce qui désigne sa base.
    let tmp = tempfile::tempdir().unwrap();
    let gabarits = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    copier(&gabarits.join("tools"), &tmp.path().join("templates/tools"));
    copier(&gabarits.join("backends/code"), &tmp.path().join("templates/backends/code"));
    let dossier_hote = config.join("templates/backends/code");
    let fichier_hote = ["rag3db.json", "backend.json"]
        .iter()
        .map(|n| dossier_hote.join(n))
        .find(|p| p.is_file())
        .expect("le manifeste de l'hôte (rag3db.json ou backend.json)");
    let hote: Value = serde_json::from_slice(&std::fs::read(&fichier_hote).unwrap()).unwrap();
    let manifeste = tmp.path().join("templates/backends/code/backend.json");
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    for cle in ["name", "workspace", "embeddings", "vector_extension"] {
        if !hote[cle].is_null() {
            m[cle] = hote[cle].clone();
        }
    }
    m["workspace"]["commands"] = json!("off");
    m["database"] = json!(base.to_string_lossy());
    if let Some(outils) = m["tools"].as_object_mut() {
        outils.retain(|_, t| t["policy"]["write_files"] != json!(true) && t["policy"]["run_commands"] != json!(true));
    }
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();

    let prepared = PreparedBackend::load(&manifeste).expect("le manifeste charge");
    let (embedder, _) = prepared.connect_embedder().expect("le service d'embarquement du manifeste");
    let backend = prepared
        .open(Box::new(Rag3dbConnection::new(&base).expect("la copie de la base")), Some(embedder))
        .expect("l'ouverture");

    println!("| requête | lucivy seul : trouvés | morceau de l'oracle (rang lucivy) | son rang dans search_code | son rang plein texte | premier de search_code |");
    println!("|---|---|---|---|---|---|");
    for q in REQUETES {
        let reponse = backend
            .call("search_code", json!({"query": q, "options": {"limit": 30}, "mode": "indexed"}))
            .unwrap_or_else(|e| panic!("search_code « {q} » : {e}"));
        let rendus: Vec<Value> = reponse["result"].as_array().cloned().unwrap_or_default();
        let premier = rendus
            .first()
            .map(|x| {
                let (f, n, _) = cle(x);
                format!("{f} : {n} ({})", x["signal"].as_str().unwrap_or("?"))
            })
            .unwrap_or_else(|| "—".into());

        // L'oracle : lucivy seul, la requête entière en un seul contains relâché.
        let (oracle, total) = {
            let cat = backend.catalog.lock().unwrap();
            let cible = cat.resolve_search_target("Scope").expect("la cible Scope");
            let handle = cat.fts_handle(&cible.parent_table).expect("l'index plein texte de Scope, ouvert par la recherche");
            let requete = build_bm25_query(q, &cible.bm25_fields, BM25Mode::Contains, 0);
            let config: lucivy_core::query::QueryConfig = serde_json::from_str(&requete).expect("la requête lucivy");
            let hits = rag3weaver::fts_handle::search_hits(&handle, &config, 30, None).expect("lucivy seul");
            let total = hits.len();
            let mut lignes = Vec::new();
            for (offset, _score, _hl) in hits.into_iter().take(3) {
                let r = cat
                    .execute_read_with_params(
                        "MATCH (s:Scope) WHERE offset(id(s)) = $o RETURN s.file_path, s.name, s.start_line",
                        &[rag3weaver::connection::QueryParam::new("o", offset as i64)],
                    )
                    .expect("le morceau de l'oracle");
                if let Some(l) = r.rows.first() {
                    lignes.push((
                        l[0].as_str().unwrap_or("?").to_string(),
                        l[1].as_str().unwrap_or("?").to_string(),
                        l[2].as_i64().unwrap_or(-1),
                    ));
                }
            }
            (lignes, total)
        };
        if oracle.is_empty() {
            println!("| {q} | 0 | — | — | — | {premier} |");
            continue;
        }
        for (rang_oracle, o) in oracle.iter().enumerate() {
            let place = rendus.iter().position(|x| cle(x) == *o);
            let rang = place.map_or(format!("absent des {}", rendus.len()), |p| (p + 1).to_string());
            let plein_texte = place
                .and_then(|p| rendus[p]["signals"].as_array().cloned())
                .and_then(|s| s.into_iter().find(|x| x["label"] == "bm25"))
                .map_or("—".to_string(), |s| s["rank"].to_string());
            println!("| {q} | {total} | {} : {} ({}) | {rang} | {plein_texte} | {premier} |", o.0, o.1, rang_oracle + 1);
        }
    }
}
