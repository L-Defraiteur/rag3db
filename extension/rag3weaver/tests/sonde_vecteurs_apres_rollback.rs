//! **Après un paquet défait, l'index vectoriel retrouve-t-il les morceaux
//! repris ?** (5 octobre 2026). Hors de la batterie : rouge tant que le moteur
//! n'est pas corrigé (session cœur C++).
//!
//! Le défaut : sur une table qui porte un index vectoriel, un COPY annulé
//! (BEGIN ; COPY ; ROLLBACK) laisse avancé le compteur des lignes indexées ; la
//! recherche dans l'index échoue ensuite (« vector::reserve »), et un COPY
//! rejoué et validé n'indexe pas ses lignes — dans la table, absentes de
//! l'index, en silence. La transaction par paquet (`RAG3WEAVER_TX_PAR_PAQUET`)
//! y passe sur le chemin d'échec d'un paquet : les morceaux partent par COPY
//! dans des tables `*_Chunk` déjà indexées.
//!
//! Trois processus, l'embarqueur factice (`HashEmbedder`, pas de service) :
//! - l'**échoueur** fait échouer le paquet 6 (quatre paquets par validation),
//!   ROLLBACK, puis ferme ;
//! - le **repreneur** rouvre et reprend l'index ;
//! - le **vérificateur** prend, par table de morceaux, un échantillon borné et
//!   déterministe (une quarantaine, par uuid) et cherche chacun par son PROPRE
//!   vecteur dans l'index : il doit figurer parmi les dix premiers, ou un
//!   vecteur identique au sien en tête.
//! Le même vérificateur sur une base sans échec (le témoin) prouve que la
//! sonde elle-même voit juste.
//!
//! ```bash
//! ./run_e2e.sh --test sonde_vecteurs_apres_rollback
//! ```
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::path::{Path, PathBuf};

use rag3weaver::code::{default_scope_chunking, register_code_schema};
use rag3weaver::code_sync::{sync_source, RelationsMode, SourceSyncOptions};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::connection::{CypherValue, DbConnection, QueryParam};
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

const ROLE: &str = "SONDE_VECTEURS_ROLE";
const BASE: &str = "SONDE_VECTEURS_BASE";
const TABLES: [&str; 2] = ["Scope_Chunk", "File_Chunk"];

fn racine_moteur() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap().display().to_string()
    })
}

fn dossier(cas: &str) -> PathBuf {
    let d = PathBuf::from(std::env::var("HOME").unwrap()).join(".cache/rag3weaver-build/sonde-vecteurs").join(format!(
        "{cas}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d.join("base.rag3db")
}

fn corpus() -> Vec<(String, String)> {
    (0..300)
        .map(|i| {
            let appel = if i > 0 { format!("f{}() + ", i - 1) } else { String::new() };
            (format!("src/f{i:03}.rs"), format!("pub fn f{i}() -> u32 {{ {appel}{i} }}\n"))
        })
        .collect()
}

fn catalogue(base: &Path) -> Catalog {
    let conn = Rag3dbConnection::new(base).expect("ouvrir la base");
    conn.execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", racine_moteur()))
        .expect("extension vecteur");
    let config = CatalogConfig { name: Some("sonde-vecteurs".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().expect("initialize");
    register_code_schema(&mut catalog, default_scope_chunking()).expect("schéma du code");
    catalog
}

fn options(reprendre: bool) -> SourceSyncOptions {
    SourceSyncOptions {
        batch_files: 32,
        relations: Some(RelationsMode::Bulk),
        // Les vecteurs posés avec les lignes : c'est le COPY des morceaux
        // vectorisés qui est en jeu.
        exige: Disponibilites::TOUT,
        force: true,
        takeover: reprendre,
        ..Default::default()
    }
}

/// Le vérificateur : par table, l'échantillon, et pour chacun sa recherche.
fn verifier(catalog: &Catalog) {
    for table in TABLES {
        let st = catalog.vector_storage(table).expect("stockage vectoriel");
        let lignes = catalog
            .execute_raw(&format!(
                "MATCH (c:{table}) WHERE c.{col} IS NOT NULL RETURN c._uuid, c.{col} ORDER BY c._uuid",
                col = st.column
            ))
            .expect("lire les morceaux")
            .rows;
        let pas = (lignes.len() / 40).max(1);
        let (mut vus, mut manques, mut erreurs) = (0usize, 0usize, 0usize);
        let mut exemples: Vec<String> = Vec::new();
        for l in lignes.iter().step_by(pas) {
            vus += 1;
            let uuid = l[0].as_str().unwrap_or("").to_string();
            let q = format!("CALL QUERY_VECTOR_INDEX('{table}', '{}', $v, 10) RETURN node._uuid, distance", st.index);
            match catalog.conn().execute_with_params(&q, &[QueryParam::new("v", l[1].clone())]) {
                Ok(r) => {
                    // Retrouvé : son uuid parmi les dix premiers, ou un vecteur
                    // identique au sien (distance nulle) en tête.
                    let le_sien = r.rows.iter().any(|x| x.first().and_then(CypherValue::as_str) == Some(uuid.as_str()));
                    let premier = r.rows.first().and_then(|x| x.get(1)).and_then(CypherValue::as_f64);
                    if !le_sien && !premier.is_some_and(|d| d.abs() < 1e-6) {
                        manques += 1;
                        if exemples.len() < 3 {
                            exemples.push(format!("{uuid} (premier à {premier:?}, {} résultats)", r.rows.len()));
                        }
                    }
                }
                Err(e) => {
                    erreurs += 1;
                    if exemples.len() < 3 {
                        exemples.push(format!("{uuid} : {}", e.to_string().chars().take(120).collect::<String>()));
                    }
                }
            }
        }
        println!("VECTEURS {table} lignes={} echantillon={vus} manques={manques} erreurs={erreurs} exemples={exemples:?}", lignes.len());
    }
}

#[test]
#[ignore]
fn role_enfant() {
    let (Ok(role), Ok(base)) = (std::env::var(ROLE), std::env::var(BASE)) else { return };
    let base = PathBuf::from(base);
    let mut catalog = catalogue(&base);
    match role.as_str() {
        "echoueur" => {
            let e = sync_source(&mut catalog, &Snapshot::new("depot", corpus()), &options(false), &mut |_| {})
                .expect_err("le paquet piégé échoue");
            println!("ECHEC {e}");
            drop(catalog);
        }
        "repreneur" | "temoin" => {
            let r = sync_source(&mut catalog, &Snapshot::new("depot", corpus()), &options(role == "repreneur"), &mut |_| {})
                .expect("synchroniser");
            println!("SYNC failed={}", r.failed);
        }
        _ => verifier(&catalog),
    }
}

fn lancer(role: &str, base: &Path, piege: bool) -> String {
    let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", "role_enfant", "--nocapture", "--ignored"])
        .env(ROLE, role)
        .env(BASE, base)
        .env("RAG3WEAVER_TX_PAR_PAQUET", "1")
        .env("RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION", "4")
        .env_remove("RAG3WEAVER_TEST_FAIL_IN_BATCH");
    if piege {
        cmd.env("RAG3WEAVER_TEST_FAIL_IN_BATCH", "6");
    }
    let s = cmd.output().expect("lancer le fils");
    let texte = format!("{}{}", String::from_utf8_lossy(&s.stdout), String::from_utf8_lossy(&s.stderr));
    assert!(s.status.success(), "le rôle {role} a échoué ({:?}) :\n{texte}", s.status);
    texte
}

/// Les lignes « VECTEURS … » d'une sortie, et leur verdict.
fn bilan(sortie: &str) -> (Vec<String>, bool) {
    let lignes: Vec<String> = sortie.lines().filter(|l| l.starts_with("VECTEURS")).map(str::to_string).collect();
    let sain = lignes.len() == TABLES.len() && lignes.iter().all(|l| l.contains(" manques=0 ") && l.contains(" erreurs=0 "));
    (lignes, sain)
}

/// **Une base défaite par l'ancien moteur, reprise par le nouveau** : le
/// repreneur sur une copie de `SONDE_VECTEURS_ANCIENNE` (un dossier gardé
/// par une passe d'avant le correctif). Attendu : pas de plantage ; un refus
/// nommé (« is behind its table ») ou une reprise saine.
#[test]
#[ignore]
fn une_base_defaite_par_l_ancien_moteur_se_reprend_ou_se_refuse_par_son_nom() {
    let Ok(ancienne) = std::env::var("SONDE_VECTEURS_ANCIENNE") else {
        println!("SONDE_VECTEURS_ANCIENNE absente : rien à reprendre");
        return;
    };
    let base = dossier("ancienne");
    for f in std::fs::read_dir(&ancienne).unwrap().flatten() {
        std::fs::copy(f.path(), base.parent().unwrap().join(f.file_name())).unwrap();
    }
    let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", "role_enfant", "--nocapture", "--ignored"])
        .env(ROLE, "repreneur")
        .env(BASE, &base)
        .env("RAG3WEAVER_TX_PAR_PAQUET", "1")
        .env("RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION", "4")
        .env_remove("RAG3WEAVER_TEST_FAIL_IN_BATCH");
    let s = cmd.output().expect("lancer le repreneur");
    let texte = format!("{}{}", String::from_utf8_lossy(&s.stdout), String::from_utf8_lossy(&s.stderr));
    let nomme = texte.contains("is behind its table");
    let fin: String = texte.lines().rev().take(15).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
    println!("▸ repreneur sur l'ancienne base : {:?}, refus nommé : {nomme}\n{fin}", s.status);
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert!(s.status.signal().is_none(), "le repreneur est mort d'un signal (base gardée : {}) :\n{fin}", base.display());
    }
    if s.status.success() {
        let (lignes, sain) = bilan(&lancer("verificateur", &base, false));
        println!("▸ après reprise : {lignes:?}");
        assert!(sain, "reprise acceptée mais index faux (base gardée : {}) : {lignes:?}", base.display());
    } else {
        assert!(nomme, "le repreneur a échoué sans le refus nommé (base gardée : {}) :\n{fin}", base.display());
    }
    let _ = std::fs::remove_dir_all(base.parent().unwrap());
}

#[test]
#[ignore]
fn apres_un_paquet_defait_l_index_vectoriel_retrouve_chaque_morceau_repris() {
    // Le témoin d'abord : sans échec, la sonde doit tout retrouver.
    let temoin = dossier("temoin");
    lancer("temoin", &temoin, false);
    let (lignes_t, sain_t) = bilan(&lancer("verificateur", &temoin, false));
    println!("▸ témoin : {lignes_t:?}");
    assert!(sain_t, "la sonde elle-même ne retrouve pas les morceaux d'une base saine : {lignes_t:?}");

    let base = dossier("rollback");
    let sortie = lancer("echoueur", &base, true);
    assert!(sortie.contains("échec au paquet 6"), "le paquet piégé a échoué :\n{sortie}");
    lancer("repreneur", &base, false);
    let (lignes, sain) = bilan(&lancer("verificateur", &base, false));
    println!("▸ après ROLLBACK et reprise : {lignes:?}");
    assert!(sain, "l'index vectoriel ne retrouve pas tous les morceaux repris (base gardée : {}) : {lignes:?}", base.display());
    let _ = std::fs::remove_dir_all(base.parent().unwrap());
    let _ = std::fs::remove_dir_all(temoin.parent().unwrap());
}
