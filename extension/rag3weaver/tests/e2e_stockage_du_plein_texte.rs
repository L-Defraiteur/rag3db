//! **Où vit le plein texte d'une base** (5 octobre 2026, défaut basculé en
//! préparation, décision de Lucie).
//!
//! Une base neuve sur disque range son plein texte dans des fichiers, à côté
//! d'elle (`<base>.fts/`). Une base existante dont le plein texte est déjà
//! dans la base (`_index_blobs`) le garde : pas de migration silencieuse.
//! Une base en mémoire le garde dans la base. `RAG3WEAVER_FTS=blobs` ou
//! `fichiers` force le choix. Le lecteur et l'écrivain d'une même base font
//! le même choix.
//!
//! Chaque cas dans un processus fils (la variable d'environnement y reste).
//!
//! ```bash
//! ./run_e2e.sh --test e2e_stockage_du_plein_texte
//! ```
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rag3weaver::code::{default_scope_chunking, register_code_schema, SCOPE};
use rag3weaver::code_sync::{sync_source, SourceSyncOptions};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::search::{Consistency, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

const ROLE: &str = "STOCKAGE_FTS_ROLE";
const BASE: &str = "STOCKAGE_FTS_BASE";

fn corpus() -> Vec<(String, String)> {
    (0..40).map(|i| (format!("src/f{i:03}.rs"), format!("pub fn fonction_{i}() -> u32 {{ {i} }}\n"))).collect()
}

fn racine_moteur() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap().display().to_string()
    })
}

fn catalogue(conn: Rag3dbConnection, lecture: bool, blobs: bool) -> Catalog {
    use rag3weaver::connection::DbConnection;
    // L'extension d'abord : sans elle, enregistrer le schéma créerait l'index
    // vectoriel, une écriture, refusée à un lecteur.
    conn.execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", racine_moteur()))
        .expect("extension vecteur");
    let config = CatalogConfig { name: Some("stockage-fts".into()), embedding_dim: 16, ..Default::default() };
    let mut catalog = if lecture {
        Catalog::ouvrir_en_lecture(Box::new(conn), Box::new(HashEmbedder::new(16)), config)
    } else {
        Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(16)), config)
    };
    if blobs {
        catalog.set_fts_storage(rag3weaver::fts_handle::FtsStorage::BlobBacked { lazy: false });
    }
    catalog.initialize().expect("initialize");
    register_code_schema(&mut catalog, default_scope_chunking()).expect("schéma du code");
    catalog
}

/// La fonction 17 est-elle trouvée par ses mots ?
fn trouvee(catalog: Catalog) -> bool {
    let partage = Arc::new(Mutex::new(catalog));
    let r = Catalog::rechercher(&partage, SCOPE, "fonction_17", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        ..Default::default()
    })
    .expect("recherche");
    r.results.iter().any(|x| format!("{x:?}").contains("fonction_17"))
}

fn blobs_en_base(catalog: &Catalog) -> i64 {
    catalog.execute_raw("MATCH (b:_index_blobs) RETURN count(b)").map(|r| r.rows[0][0].as_i64().unwrap_or(-1)).unwrap_or(-1)
}

#[test]
#[ignore]
fn role_enfant() {
    let (Ok(role), Ok(base)) = (std::env::var(ROLE), std::env::var(BASE)) else { return };
    let synchroniser = |catalog: &mut Catalog| {
        let options = SourceSyncOptions { exige: Disponibilites::RECHERCHE_TEXTE, ..Default::default() };
        let r = sync_source(catalog, &Snapshot::new("depot", corpus()), &options, &mut |_| {}).expect("synchroniser");
        assert_eq!(r.failed, 0, "{r:?}");
    };
    match role.as_str() {
        // Écrire une base : au défaut, ou en blobs posés (une base « d'avant »).
        "ecrire" | "ecrire_en_blobs" => {
            let mut c = catalogue(Rag3dbConnection::new(&base).unwrap(), false, role == "ecrire_en_blobs");
            synchroniser(&mut c);
            println!("BLOBS {}", blobs_en_base(&c));
        }
        // Rouvrir au défaut, écrire encore, puis chercher.
        "rouvrir" => {
            let mut c = catalogue(Rag3dbConnection::new(&base).unwrap(), false, false);
            synchroniser(&mut c);
            println!("TROUVEE {}", trouvee(c));
        }
        "lire" => {
            // Un catalogue en mode lecture (comme `e2e_tx_par_paquet_arret`) :
            // il choisit son stockage comme l'écrivain, sans rien écrire.
            let c = catalogue(Rag3dbConnection::new(&base).unwrap(), true, false);
            println!("TROUVEE {}", trouvee(c));
        }
        "memoire" => {
            let mut c = catalogue(Rag3dbConnection::in_memory().unwrap(), false, false);
            synchroniser(&mut c);
            println!("BLOBS {}", blobs_en_base(&c));
            println!("TROUVEE {}", trouvee(c));
        }
        _ => {}
    }
}

fn lancer(role: &str, base: &Path, fts: Option<&str>) -> String {
    let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", "role_enfant", "--nocapture", "--ignored"]).env(ROLE, role).env(BASE, base);
    match fts {
        Some(v) => cmd.env("RAG3WEAVER_FTS", v),
        None => cmd.env_remove("RAG3WEAVER_FTS"),
    };
    let s = cmd.output().expect("lancer le fils");
    let texte = format!("{}{}", String::from_utf8_lossy(&s.stdout), String::from_utf8_lossy(&s.stderr));
    assert!(s.status.success(), "le fils « {role} » a échoué ({:?}) :\n{}", s.status, texte.chars().take(3000).collect::<String>());
    texte
}

fn ligne<'a>(t: &'a str, p: &str) -> &'a str {
    t.lines().find_map(|l| l.strip_prefix(p)).unwrap_or_else(|| panic!("pas de « {p} » :\n{t}"))
}

fn dossier(cas: &str) -> PathBuf {
    let d = PathBuf::from(std::env::var("HOME").unwrap()).join(format!(
        ".cache/rag3weaver-build/stockage-fts-{cas}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
#[ignore]
fn une_base_neuve_sur_disque_range_son_plein_texte_dans_des_fichiers() {
    let d = dossier("neuve");
    let base = d.join("base.rag3db");
    let ecrit = lancer("ecrire", &base, None);
    assert!(PathBuf::from(format!("{}.fts", base.display())).is_dir(), "le dossier <base>.fts existe");
    println!("▸ base neuve : blobs en base {}", ligne(&ecrit, "BLOBS "));
    assert_eq!(ligne(&lancer("rouvrir", &base, None), "TROUVEE "), "true", "rouverte, elle cherche dans ses fichiers");
    assert_eq!(ligne(&lancer("lire", &base, None), "TROUVEE "), "true", "un lecteur fait le même choix");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
#[ignore]
fn une_base_existante_en_blobs_les_garde() {
    let d = dossier("existante");
    let base = d.join("base.rag3db");
    let ecrit = lancer("ecrire_en_blobs", &base, None);
    let blobs: i64 = ligne(&ecrit, "BLOBS ").trim().parse().unwrap();
    assert!(blobs > 0, "la base d'avant a son plein texte en blobs ({blobs})");
    assert_eq!(ligne(&lancer("rouvrir", &base, None), "TROUVEE "), "true", "rouverte au défaut, elle cherche dans ses blobs");
    assert!(!PathBuf::from(format!("{}.fts", base.display())).exists(), "pas de migration silencieuse : aucun dossier .fts créé");
    assert_eq!(ligne(&lancer("lire", &base, None), "TROUVEE "), "true", "un lecteur fait le même choix");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
#[ignore]
fn la_variable_force_les_blobs_et_une_base_en_memoire_les_garde() {
    let d = dossier("force");
    let base = d.join("base.rag3db");
    let ecrit = lancer("ecrire", &base, Some("blobs"));
    assert!(!PathBuf::from(format!("{}.fts", base.display())).exists(), "RAG3WEAVER_FTS=blobs : pas de dossier .fts");
    assert!(ligne(&ecrit, "BLOBS ").trim().parse::<i64>().unwrap() > 0, "le plein texte en blobs");
    let memoire = lancer("memoire", &base, None);
    assert!(ligne(&memoire, "BLOBS ").trim().parse::<i64>().unwrap() > 0, "une base en mémoire garde ses blobs");
    assert_eq!(ligne(&memoire, "TROUVEE "), "true");
    let _ = std::fs::remove_dir_all(&d);
}
