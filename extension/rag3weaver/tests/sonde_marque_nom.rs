//! **Sonde : les arêtes d'usage d'un arbre, par marque, et chacune en TSV** (15 novembre 2026),
//! pour mesurer avant et après une règle de résolution (ticket « la marque nom relie un appel de
//! méthode standard à un homonyme »).
//!
//! `SONDE_DIRS=<d1>,<d2>` : chaque dossier est indexé seul, en mémoire (aucune base de service
//! touchée), avec la porte de l'index. Rend, par dossier, les CONSUMES par marque et le compte des
//! appels sur un receveur non typé (rendez-vous visibles, jamais reliés). `SONDE_TSV=<préfixe>`
//! écrit en plus `<préfixe>-<n>.tsv`, une ligne par CONSUMES : fichier et ligne du mentionneur, son
//! nom, la cible, son fichier, la marque — de quoi faire un `comm` entre deux pointeurs.
//!
//! Run with: SONDE_DIRS=… cargo test --features rag3db-native,code --test sonde_marque_nom -- --ignored --nocapture

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use rag3weaver::code::{admission, default_scope_chunking, register_code_schema};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn setup() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = rag3weaver::backend::vector_extension_of_this_process().expect("l'extension vector de ce processus").display().to_string();
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("sonde-marque-nom".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    catalog
}

fn fichiers(d: &std::path::Path, v: &mut Vec<std::path::PathBuf>) {
    let Ok(lu) = std::fs::read_dir(d) else { return };
    for e in lu.flatten() {
        let p = e.path();
        let nom = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if nom.starts_with('.') || nom == "target" || nom == "node_modules" || nom == "build" {
            continue;
        }
        if p.is_dir() {
            fichiers(&p, v)
        } else if let Some(x) = p.extension().and_then(|x| x.to_str()) {
            if ["rs", "c", "h", "cpp", "cc", "hpp", "py", "ts", "js", "go", "cs"].contains(&x) {
                v.push(p)
            }
        }
    }
}

/// Les manifestes sous `d`, sans suivre les liens ni entrer dans les dossiers de bâti.
fn des_manifestes(d: &std::path::Path, v: &mut Vec<String>) {
    let Ok(lu) = std::fs::read_dir(d) else { return };
    for e in lu.flatten() {
        let p = e.path();
        let nom = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if nom.starts_with('.') || nom == "target" || nom == "node_modules" || nom == "build" || e.file_type().map(|t| t.is_symlink()).unwrap_or(true) {
            continue;
        }
        if p.is_dir() {
            des_manifestes(&p, v)
        } else if rag3weaver::manifestes::est_un_manifeste(&p.to_string_lossy()) {
            v.push(p.to_string_lossy().to_string())
        }
    }
}

fn texte(v: Option<&rag3weaver::connection::CypherValue>) -> String {
    v.map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.as_i64().map(|n| n.to_string()).unwrap_or_default())).unwrap_or_default()
}

#[test]
#[ignore]
fn aretes_par_marque_en_tsv() {
    let dirs = std::env::var("SONDE_DIRS").expect("SONDE_DIRS");
    let tsv = std::env::var("SONDE_TSV").ok();
    for (n, racine) in dirs.split(',').enumerate() {
        let mut v = Vec::new();
        fichiers(std::path::Path::new(racine), &mut v);
        v.sort();
        let porte = rag3weaver::generated::GeneratedPolicy::default();
        let sources: Vec<(String, String)> = v
            .iter()
            .filter_map(|p| {
                let c = std::fs::read_to_string(p).ok()?;
                let rel = p.strip_prefix(racine).ok()?.display().to_string();
                (c.len() <= 1_000_000 && admission(&rel, Some(&c), &porte).is_ok()).then_some((rel, c))
            })
            .collect();
        let admis = sources.len();
        // Les manifestes du corpus, lus comme l'index les lit (`Cargo.toml`, `package.json`, `pyproject.toml`…) :
        // la provenance des imports et les paquets du dépôt en dépendent (20 novembre 2026).
        let mut manifestes = Vec::new();
        des_manifestes(std::path::Path::new(racine), &mut manifestes);
        manifestes.sort();
        let m = rag3weaver::manifestes::Manifestes::lire(&manifestes, |c| std::fs::read_to_string(c).ok());
        let mut cat = setup();
        cat.ingest_code(&rag3weaver::code::analyze_in_project_avec(racine, sources, "", None, Some(&m))).unwrap();
        let r = cat.execute_raw("MATCH (:Scope)-[r:CONSUMES]->(:Scope) RETURN r.resolution, count(*)").unwrap();
        let mut marques: Vec<(String, i64)> = r.rows.iter().map(|x| (texte(x.first()), x.get(1).and_then(|v| v.as_i64()).unwrap_or(0))).collect();
        marques.sort();
        let non_types = cat.execute_raw("MATCH (:Scope)-[m:MENTIONS]->(:Symbol) WHERE m.untyped_receiver = 'oui' RETURN count(*)").unwrap();
        let scopes = cat.execute_raw("MATCH (s:Scope) RETURN count(*)").unwrap();
        let orphans = cat.execute_raw("MATCH (s:Scope) WHERE NOT EXISTS { MATCH (s)-[:DEFINED_IN]->(:File) } RETURN count(*)").unwrap();
        println!(
            "SONDE {racine} : fichiers {} ; admis {admis} ; scopes {} (sans DEFINED_IN {}) ; CONSUMES {:?} ; rendez-vous non typés {}",
            v.len(),
            texte(scopes.rows[0].first()),
            texte(orphans.rows[0].first()),
            marques,
            texte(non_types.rows[0].first())
        );
        if let Some(prefixe) = &tsv {
            let r = cat
                .execute_raw("MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) RETURN a.file_path, r.line, a.name, b.name, b.file_path, r.resolution")
                .unwrap();
            let mut lignes: Vec<String> = r.rows.iter().map(|x| (0..6).map(|i| texte(x.get(i))).collect::<Vec<_>>().join("\t")).collect();
            lignes.sort();
            std::fs::write(format!("{prefixe}-{n}.tsv"), lignes.join("\n") + "\n").unwrap();
        }
    }
}
