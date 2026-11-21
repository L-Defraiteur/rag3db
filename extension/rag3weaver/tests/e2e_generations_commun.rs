//! **Les générations du plein texte, ce que partagent leurs témoins** (lucivy 4.4 ; décision de Lucie du 21 novembre
//! 2026 : (A), la génération du paquet). Un processus enfant, en mode fichiers et en transaction par paquet, deux
//! paquets par validation : une mort au paquet 3 laisse le dossier au moins deux générations au-dessus de la base.
//!
//! ```bash
//! ./run_e2e.sh --test e2e e2e_generations
//! ```
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::path::{Path, PathBuf};

use crate::e2e_tx_par_paquet_arret as tx;

const ROLE: &str = "GEN_ROLE";
const BASE: &str = "GEN_BASE";
/// Ce que la garde d'ouverture dit quand elle jette le dossier.
pub const REBATI: &str = "dossier jeté, rebâti depuis les lignes";
/// Les variables des pièges du produit, retirées de tout enfant qui ne les demande pas.
const PIEGES: [&str; 6] = [
    "RAG3WEAVER_TEST_KILL_IN_BATCH",
    "RAG3WEAVER_TEST_FAIL_IN_BATCH",
    "RAG3WEAVER_TEST_KILL_BEFORE_BLOB_PUSH",
    "RAG3WEAVER_TX_POUSSEE_A_LA_FIN",
    "RAG3WEAVER_TEST_KILL_IN_COMMIT",
    "RAG3WEAVER_TEST_KILL_BEFORE_FTS_SYNC",
];

pub fn dossier(cas: &str) -> PathBuf {
    crate::common::dossier_de_suite("generations-du-plein-texte", cas)
}

#[test]
#[ignore]
fn role_enfant() {
    let (Ok(role), Ok(base)) = (std::env::var(ROLE), std::env::var(BASE)) else { return crate::common::enfant("e2e_generations_commun") };
    let base = PathBuf::from(base);
    if role == "generations" {
        // La génération que la base a validée, et celle du dossier, par index ; sans ouvrir de catalogue.
        let conn = rag3weaver::Rag3dbConnection::new(&base).expect("rouvrir la base");
        let r = rag3weaver::connection::DbConnection::execute(
            &conn,
            "MATCH (m:_catalog_meta) WHERE m._key STARTS WITH 'fts_generation:' RETURN m._key, m._value",
        )
        .expect("les marques");
        for l in &r.rows {
            let index = l[0].as_str().unwrap_or("").trim_start_matches("fts_generation:").to_string();
            let en_base: u64 = l[1].as_str().and_then(|v| v.parse().ok()).unwrap_or(0);
            let disque = rag3weaver::fts_directory::generation_on_disk(&Path::new(&format!("{}.fts", base.display())).join(&index));
            println!("GENERATION {index} base={en_base} disque={}", disque.unwrap_or(0));
        }
        return;
    }
    let t = std::time::Instant::now();
    let mut catalog = tx::catalogue_en(&base, false);
    if role == "mainteneur" {
        // La maintenance de la nuit, avec compaction, sur une base déjà indexée : la taille du plein texte avant et
        // après, et la génération que la base nomme.
        catalog.open_fts_files();
        let fts = std::path::PathBuf::from(format!("{}.fts", base.display()));
        let avant = taille(&fts);
        let maintenant = jiff::civil::date(2026, 11, 21).at(3, 30, 0, 0);
        let note = catalog.fts_maintenance_nocturne(maintenant, 0.5, true).expect("une maintenance du plein texte");
        assert!(!note.to_string().contains("\"error\""), "la maintenance passe : {note}");
        // Le ramasse-miettes passe au commit suivant d'un index : un commit sous la génération courante, qui n'en
        // étiquette aucune (une synchronisation en étiquetterait une, et rendrait la place pour une autre raison).
        for (table, h) in catalog.fts_handles() {
            h.commit_keeping_generation().unwrap_or_else(|e| panic!("commit de {table} : {e}"));
        }
        println!("TAILLE avant={avant} apres={}", taille(&fts));
        return;
    }
    if role == "echoueur" {
        let options = rag3weaver::code_sync::SourceSyncOptions {
            batch_files: tx::paquet(),
            relations: Some(rag3weaver::code_sync::RelationsMode::Bulk),
            exige: rag3weaver::disponibilite::Disponibilites::RECHERCHE_TEXTE,
            force: true,
            ..Default::default()
        };
        let erreur = rag3weaver::code_sync::sync_source(&mut catalog, &rag3weaver::code_tools::Snapshot::new("depot", tx::corpus()), &options, &mut |_| {})
            .expect_err("le paquet piégé échoue");
        println!("ECHEC {erreur}");
        assert!(catalog.must_reopen().is_some(), "le catalogue est empoisonné après le ROLLBACK : {erreur}");
        return;
    }
    if role == "repreneur" {
        // La remise en état du plein texte seule : l'ouverture des index, et le rebâti en ligne s'il y en a un.
        catalog.verify_fts_files();
        println!("REMISE_MS {role} {}", t.elapsed().as_millis());
    }
    tx::synchroniser(&mut catalog, role == "repreneur");
    // La reprise : rouvrir, remettre le plein texte d'accord avec la base, finir la synchronisation.
    println!("REPRISE_MS {role} {}", t.elapsed().as_millis());
    println!("COMPTES {role} {}", serde_json::to_string(&tx::comptes(&catalog)).unwrap());
}

/// Le mode mesure (`GEN_GROS=1`) : le corpus de 100 000 scopes n'a que deux paquets de 200 fichiers ; on y meurt au
/// paquet 1, un paquet par validation (une génération d'avance), pour comparer le temps de reprise.
pub fn mesure() -> bool {
    std::env::var_os("GEN_GROS").is_some()
}

fn par_validation() -> &'static str {
    if mesure() { "1" } else { "2" }
}

/// Le paquet où l'écrivain meurt.
pub fn paquet_tue() -> &'static str {
    if mesure() { "1" } else { "3" }
}

/// La taille d'un dossier, sous-dossiers compris.
pub fn taille(p: &Path) -> u64 {
    let mut t = 0;
    for e in std::fs::read_dir(p).into_iter().flatten().flatten() {
        let Ok(m) = e.metadata() else { continue };
        t += if m.is_dir() { taille(&e.path()) } else { m.len() };
    }
    t
}

/// Lance un enfant `role` sur `base`, avec les pièges `pieges` ; rend son statut et sa sortie.
pub fn lancer(role: &str, base: &Path, pieges: &[(&str, &str)]) -> (std::process::ExitStatus, String) {
    let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", &crate::common::test_de_cette_suite!("role_enfant"), "--nocapture", "--ignored"])
        .env(ROLE, role)
        .env(BASE, base)
        .env("TX_ARRET_FTS", "fichiers")
        .env("RAG3WEAVER_TX_PAR_PAQUET", "1")
        .env("RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION", par_validation());
    for p in PIEGES {
        cmd.env_remove(p);
    }
    for (k, v) in pieges {
        cmd.env(k, v);
    }
    // Le corpus gros (100 000 scopes) pour la mesure du temps de reprise : `GEN_GROS=1`.
    if std::env::var_os("GEN_GROS").is_some() {
        cmd.env("TX_ARRET_GROS", "1");
    } else {
        cmd.env_remove("TX_ARRET_GROS");
    }
    let sortie = cmd.output().expect("lancer le fils");
    let texte = format!("{}{}", String::from_utf8_lossy(&sortie.stdout), String::from_utf8_lossy(&sortie.stderr));
    (sortie.status, texte)
}

pub fn comptes(role: &str, sortie: &str) -> std::collections::BTreeMap<String, i64> {
    let ligne = sortie
        .lines()
        .find_map(|l| l.strip_prefix(&format!("COMPTES {role} ")))
        .unwrap_or_else(|| panic!("le {role} n'a pas rendu ses comptes :\n{sortie}"));
    serde_json::from_str(ligne).unwrap()
}

pub fn reprise_ms(role: &str, sortie: &str) -> u128 {
    sortie
        .lines()
        .find_map(|l| l.strip_prefix(&format!("REPRISE_MS {role} ")))
        .and_then(|n| n.trim().parse().ok())
        .unwrap_or_else(|| panic!("le {role} n'a pas dit sa durée :\n{sortie}"))
}

/// `(index, génération en base, génération du dossier)`, lus sans ouvrir de catalogue.
pub fn generations(base: &Path) -> Vec<(String, u64, u64)> {
    let (statut, sortie) = lancer("generations", base, &[]);
    assert!(statut.success(), "{sortie}");
    sortie
        .lines()
        .filter_map(|l| l.strip_prefix("GENERATION "))
        .map(|l| {
            let mut it = l.split_whitespace();
            let index = it.next().unwrap().to_string();
            let n = |s: Option<&str>, k: &str| s.and_then(|v| v.strip_prefix(k)).and_then(|v| v.parse().ok()).unwrap_or(0);
            (index, n(it.next(), "base="), n(it.next(), "disque="))
        })
        .collect()
}

/// Un témoin sans arrêt : ses comptes, sur une base à lui.
pub fn temoin(cas: &str) -> std::collections::BTreeMap<String, i64> {
    let d = dossier(cas);
    let (statut, sortie) = lancer("temoin", &d.join("base.rag3db"), &[]);
    assert!(statut.success(), "le témoin va au bout :\n{sortie}");
    let c = comptes("temoin", &sortie);
    println!("▸ passe complète du témoin en {} ms", reprise_ms("temoin", &sortie));
    let _ = std::fs::remove_dir_all(&d);
    c
}
