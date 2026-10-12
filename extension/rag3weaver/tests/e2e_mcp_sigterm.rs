//! E2E : **SIGTERM vaut la fermeture du tube** pour le serveur MCP (11 octobre
//! 2026, marche à part de la page C, §4).
//!
//! Le cycle de vie MCP sur stdio prévoit que le client ferme l'entrée, attend,
//! **puis envoie SIGTERM** s'il juge l'attente trop longue. Un SIGTERM par
//! défaut tuait le serveur au milieu de son arrêt : base fermée sans point de
//! reprise, session de synchronisation laissée ouverte si une indexation
//! tournait. Ce témoin éprouve les deux moitiés :
//!
//! 1. un serveur qui reçoit SIGTERM **sort de lui-même, avec succès**, et le dit
//!    sur stderr — il n'est pas tué ;
//! 2. la base qu'il tenait **se rouvre** par un second serveur, qui la sert.
//!
//! Il tourne sur le gabarit `proto/boutique`, qui s'ouvre sans service
//! d'embarquement : ce qui est éprouvé est l'arrêt, pas la recherche.
//!
//! Run with: ./run_e2e.sh --test e2e_mcp_sigterm

#![cfg(all(feature = "rag3db-native", unix))]

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

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

/// La boutique copiée dans `dir`, l'extension vectorielle prise à la racine du
/// moteur (`RAG3DB_ROOT`) — la forme d'`e2e_proto_boutique`.
fn boutique(dir: &Path) -> PathBuf {
    copier(&Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/proto/boutique"), dir);
    let chemin = dir.join("backend.json");
    let mut m: Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
    m["vector_extension"] = json!(crate::common::vector_extension());
    std::fs::write(&chemin, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    chemin
}

fn lancer(manifeste: &Path) -> Child {
    Command::new(env!("CARGO_BIN_EXE_rag3weaver-backend"))
        .args(["mcp", "--standalone", "--manifest", &manifeste.to_string_lossy()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("lancer le serveur MCP")
}

/// Une requête, une réponse.
fn demander(enfant: &mut Child, sortie: &mut BufReader<std::process::ChildStdout>, requete: Value) -> Value {
    let entree = enfant.stdin.as_mut().expect("stdin");
    writeln!(entree, "{requete}").expect("écrire la requête");
    entree.flush().expect("vider");
    let mut ligne = String::new();
    sortie.read_line(&mut ligne).expect("lire la réponse");
    assert!(!ligne.is_empty(), "le serveur n'a rien répondu à {requete}");
    serde_json::from_str(&ligne).unwrap_or_else(|e| panic!("réponse illisible ({e}) : {ligne}"))
}

fn ouvrir_et_lister(enfant: &mut Child) -> (BufReader<std::process::ChildStdout>, usize) {
    let mut sortie = BufReader::new(enfant.stdout.take().expect("stdout"));
    let r = demander(
        enfant,
        &mut sortie,
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
               "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                          "clientInfo": {"name": "le témoin SIGTERM", "version": "0"}}}),
    );
    assert!(r.get("result").is_some(), "initialize : {r}");
    let r = demander(enfant, &mut sortie, json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
    let n = r["result"]["tools"].as_array().map(Vec::len).unwrap_or(0);
    (sortie, n)
}

/// Attend la sortie d'un processus, au plus `limite`. `None` s'il vit encore.
fn attendre(enfant: &mut Child, limite: Duration) -> Option<std::process::ExitStatus> {
    let debut = Instant::now();
    loop {
        if let Some(s) = enfant.try_wait().expect("try_wait") {
            return Some(s);
        }
        if debut.elapsed() > limite {
            return None;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
#[ignore]
fn sigterm_vaut_la_fermeture_du_tube_et_la_base_se_rouvre() {
    let dir = tempfile::tempdir().expect("dossier");
    let manifeste = boutique(dir.path());

    // ── 1. SIGTERM : le serveur sort de lui-même, avec succès, en le disant ──
    let mut premier = lancer(&manifeste);
    let (_sortie, outils) = ouvrir_et_lister(&mut premier);
    assert!(outils > 0, "le serveur sert ses outils avant le signal");
    let tue = Command::new("kill")
        .args(["-TERM", &premier.id().to_string()])
        .status()
        .expect("envoyer SIGTERM");
    assert!(tue.success());
    let statut = attendre(&mut premier, Duration::from_secs(60));
    let mut erreurs = String::new();
    premier.stderr.take().expect("stderr").read_to_string(&mut erreurs).ok();
    let statut = statut.unwrap_or_else(|| {
        let _ = premier.kill();
        panic!("le serveur n'est pas sorti après SIGTERM — stderr : {erreurs}")
    });
    assert!(
        statut.success(),
        "SIGTERM doit finir en sortie propre (code 0), pas en mort par signal : {statut:?} — stderr : {erreurs}"
    );
    assert!(
        erreurs.contains("SIGTERM : arrêt propre"),
        "le serveur dit qu'il a pris SIGTERM pour une fermeture : {erreurs}"
    );

    // ── 2. La base se rouvre, et elle sert ────────────────────────────────
    let mut second = lancer(&manifeste);
    let (_sortie, outils_ensuite) = ouvrir_et_lister(&mut second);
    assert_eq!(outils_ensuite, outils, "la base rouverte sert les mêmes outils");
    drop(second.stdin.take());
    let statut = attendre(&mut second, Duration::from_secs(60)).expect("la fermeture du tube");
    assert!(statut.success(), "{statut:?}");
}
