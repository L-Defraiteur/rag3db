//! **Une base fermée sans point de reprise, journal présent, s'ouvre en
//! lecture seule sans planter** (5 octobre 2026).
//!
//! C'est l'état de toute base après un arrêt brutal. C'est aussi celui que
//! fabriquent les filets du 5 octobre : fermeture sans point de reprise
//! après un paquet défait ou un COPY refusé. Le cœur C++ a vu un core dump à
//! l'ouverture en lecture seule d'une telle base. Les lecteurs de rag3weaver
//! ouvrent en lecture seule (`acces::ouvrir_lecteur`, `Acces::Direct`).
//!
//! Une ouverture en lecture seule rejoue le journal en mémoire sans toucher
//! aux fichiers. Attendu : des comptes justes, ou un refus nommé, jamais un
//! signal. Deux journaux :
//! - `ordinaire` : un point de reprise, puis 2 000 insertions ordinaires
//!   (autocommit), puis la fermeture sans point de reprise ;
//! - `copy_refuse` : la même chose, puis un COPY de liens refusé par le
//!   moteur (une preuve d'existence périmée : la base est empoisonnée et se
//!   ferme sans point de reprise, filet `f0f7f0951`) ;
//! - `vecteurs` : une table à index vectoriel, des vecteurs posés par SET
//!   après le point de reprise. Au rejeu, même en lecture seule, le moteur
//!   charge l'extension notée à côté de la base (cas que les témoins C++ du
//!   journal ne couvrent pas, d'après le cœur C++).
//!
//! Chaque rôle tourne dans un processus fils : un signal y reste.
//!
//! ```bash
//! ./run_e2e.sh --test e2e_lecture_seule_apres_fermeture_sans_point_de_reprise
//! ```
// `rag3weaver::acces` n'existe qu'avec le démon : sans la feature, ce test ne
// compilait pas sous `cargo check --tests --features rag3db-native,code`.
#![cfg(all(feature = "rag3db-native", feature = "daemon"))]

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use rag3weaver::acces::{ouvrir_lecteur, Acces};
use rag3weaver::config::FieldType;
use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::records::RefOrUuid;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

const ROLE: &str = "LECTURE_SEULE_ROLE";
const BASE: &str = "LECTURE_SEULE_BASE";

fn bout() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert(
        "nom".to_string(),
        SimpleFieldDef { field_type: FieldType::String, is_title: true, is_content: true, ..Default::default() },
    );
    EntityConfig { fields, signals: SearchSignals::BM25, hashsafe: Some(vec!["nom".into()]), ..Default::default() }
}

fn ligne(nom: &str) -> BTreeMap<String, CypherValue> {
    BTreeMap::from([("nom".to_string(), CypherValue::String(nom.into()))])
}

fn ecrire(base: &str, copy_refuse: bool) {
    let conn = Rag3dbConnection::new(base).expect("ouvrir la base");
    let config = CatalogConfig { name: Some("lecture-seule".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    catalog.register_entity("Gauche", bout()).unwrap();
    catalog.register_entity("Droite", bout()).unwrap();
    catalog.register_relation_with("Lien", "Gauche", "Droite", HashMap::new()).unwrap();
    catalog.execute_raw("CREATE NODE TABLE IF NOT EXISTS Brut(id INT64, PRIMARY KEY(id))").unwrap();
    catalog.execute_raw("CHECKPOINT").unwrap();
    // Le journal : des insertions ordinaires, chacune sa transaction.
    for i in 0..2_000 {
        catalog.execute_raw(&format!("CREATE (:Brut {{id: {i}}})")).unwrap();
    }
    if copy_refuse {
        catalog.begin_proving_presence();
        catalog.ingest_entities("Gauche", (0..30).map(|i| ligne(&format!("g{i}"))).collect()).unwrap();
        catalog.ingest_entities("Droite", (0..30).map(|i| ligne(&format!("d{i}"))).collect()).unwrap();
        catalog.execute_raw("MATCH (d:Droite {nom: 'd0'}) DETACH DELETE d").unwrap();
        for g in 0..30 {
            let ug = catalog.entity_uuid("Gauche", &ligne(&format!("g{g}"))).unwrap();
            for d in 0..30 {
                let ud = catalog.entity_uuid("Droite", &ligne(&format!("d{d}"))).unwrap();
                catalog
                    .link_jusqu_a("Lien", RefOrUuid::Uuid(ug.clone()), RefOrUuid::Uuid(ud), BTreeMap::new(), Disponibilites::AUCUNE)
                    .unwrap();
            }
        }
        let _ = catalog.drain();
        println!("EMPOISONNEE {}", catalog.must_reopen().is_some());
    } else {
        catalog.close_without_checkpoint();
    }
    drop(catalog);
    println!("ECRIT");
}

fn racine_moteur() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap().display().to_string()
    })
}

fn ecrire_vecteurs(base: &str) {
    let conn = Rag3dbConnection::new(base).expect("ouvrir la base");
    let x = |q: &str| conn.execute(q).unwrap_or_else(|e| panic!("{q} : {e}"));
    x(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", racine_moteur()));
    x("CREATE NODE TABLE Vec(id INT64, v FLOAT[8], PRIMARY KEY(id))");
    x("CALL CREATE_VECTOR_INDEX('Vec', 'vec_idx', 'v', metric := 'cosine')");
    x("CHECKPOINT");
    for i in 0..500 {
        let v: Vec<String> = (0..8).map(|j| format!("{}", ((i * 7 + j * 3) % 11) as f64 + 0.5)).collect();
        x(&format!("CREATE (:Vec {{id: {i}, v: [{}]}})", v.join(",")));
    }
    conn.close_without_checkpoint();
    drop(conn);
    println!("ECRIT");
}

fn lire(base: &str) {
    match ouvrir_lecteur(Path::new(base), Acces::Direct, None) {
        Ok(lecteur) => {
            let compte = |q: &str| lecteur.conn.execute(q).map(|r| format!("{:?}", r.rows.first())).unwrap_or_else(|e| format!("refusée : {e}"));
            println!(
                "LU brut={} gauche={} droite={} vec={} recherche={}",
                compte("MATCH (n:Brut) RETURN count(n)"),
                compte("MATCH (n:Gauche) RETURN count(n)"),
                compte("MATCH (n:Droite) RETURN count(n)"),
                compte("MATCH (n:Vec) RETURN count(n)"),
                compte("CALL QUERY_VECTOR_INDEX('Vec', 'vec_idx', [0.5,3.5,6.5,9.5,1.5,4.5,7.5,10.5], 3) RETURN count(*)")
            );
        }
        Err(e) => println!("REFUS {}", e.replace('\n', " ")),
    }
}

#[test]
#[ignore]
fn role_enfant() {
    let (Ok(role), Ok(base)) = (std::env::var(ROLE), std::env::var(BASE)) else { return };
    match role.as_str() {
        "ordinaire" => ecrire(&base, false),
        "copy_refuse" => ecrire(&base, true),
        "vecteurs" => ecrire_vecteurs(&base),
        _ => lire(&base),
    }
}

fn lancer(role: &str, base: &Path) -> (std::process::ExitStatus, String) {
    let s = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "role_enfant", "--nocapture", "--ignored"])
        .env(ROLE, role)
        .env(BASE, base)
        .output()
        .expect("lancer le fils");
    (s.status, format!("{}{}", String::from_utf8_lossy(&s.stdout), String::from_utf8_lossy(&s.stderr)))
}

fn journal(dossier: &Path) -> u64 {
    std::fs::read_dir(dossier)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".wal"))
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

fn cas(ecrivain: &str) {
    let dossier = PathBuf::from(std::env::var("HOME").unwrap()).join(format!(
        ".cache/rag3weaver-build/lecture-seule-{ecrivain}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&dossier).unwrap();
    let base = dossier.join("base.rag3db");
    let (statut, sortie) = lancer(ecrivain, &base);
    assert!(statut.success() && sortie.contains("ECRIT"), "l'écrivain {ecrivain} a échoué ({statut:?}) :\n{}", sortie.chars().take(3000).collect::<String>());
    if ecrivain == "copy_refuse" {
        assert!(sortie.contains("EMPOISONNEE true"), "le COPY refusé a empoisonné la base :\n{}", sortie.chars().take(3000).collect::<String>());
    }
    let wal = journal(&dossier);
    println!("▸ {ecrivain} : journal de {wal} octets après la fermeture");
    assert!(wal > 0, "le journal est vide : la base a fait son point de reprise, le test ne prouverait rien");

    let (statut, sortie) = lancer("lecteur", &base);
    use std::os::unix::process::ExitStatusExt;
    let dit = sortie.lines().find(|l| l.starts_with("LU ") || l.starts_with("REFUS ")).unwrap_or("rien dit").to_string();
    println!("▸ {ecrivain} : lecteur {statut:?} — {}", dit.chars().take(400).collect::<String>());
    assert!(statut.signal().is_none(), "le lecteur en lecture seule est mort d'un signal (base gardée : {}) :\n{}", dossier.display(), sortie.chars().take(3000).collect::<String>());
    assert!(statut.success(), "le lecteur a échoué sans signal :\n{}", sortie.chars().take(3000).collect::<String>());
    if let Some(lu) = dit.strip_prefix("LU ") {
        if ecrivain == "vecteurs" {
            assert!(lu.contains("vec=Some([Int(500)])") && lu.contains("recherche=Some([Int(3)])"), "les vecteurs du journal relus, l'index répond : {lu}");
            let _ = std::fs::remove_dir_all(&dossier);
            return;
        }
        assert!(lu.contains("brut=Some([Int(2000)])"), "les insertions du journal sont relues : {lu}");
        if ecrivain == "copy_refuse" {
            assert!(lu.contains("gauche=Some([Int(30)])") && lu.contains("droite=Some([Int(29)])"), "l'état d'avant le COPY refusé : {lu}");
        }
    }
    let _ = std::fs::remove_dir_all(&dossier);
}

#[test]
#[ignore]
fn apres_des_ecritures_ordinaires_la_lecture_seule_relit_le_journal() {
    cas("ordinaire");
}

#[test]
#[ignore]
fn apres_un_copy_refuse_la_lecture_seule_ne_plante_pas() {
    cas("copy_refuse");
}

#[test]
#[ignore]
fn avec_un_index_vectoriel_la_lecture_seule_rejoue_et_l_index_repond() {
    cas("vecteurs");
}
