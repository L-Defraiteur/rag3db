//! **La marche 4 de la forme C : le cycle de vie de l'hôte du démon**, éprouvé
//! par le binaire (page C, §4, §5, §7 et décisions 3 et 6 du §9).
//!
//! Sans `--daemon`, le serveur MCP lit `daemon.json`, s'attache à l'hôte qui
//! y est écrit, et **le lance s'il n'est pas là** — un seul lancement même si
//! deux clients démarrent ensemble (verrou de fichier), et l'hôte survit au
//! client. Chaque requête porte le jeton local. L'outil `daemon_status` dit ce
//! qui se passe, même quand l'hôte manque.
//!
//! Chaque témoin a son dossier de démon (`RAG3WEAVER_DAEMON_DIR`) et un port
//! libre (`RAG3WEAVER_DAEMON_ADDRESS=127.0.0.1:0`) : aucun ne touche au
//! `daemon.json` de la personne, ni au port 7970. Chacun arrête l'hôte qu'il a
//! fait lancer, par `POST /arret` et le jeton.
//!
//! Le gabarit `proto/boutique`, qui s'ouvre sans service d'embarquement.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

fn boutique(dir: &Path) -> PathBuf {
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
    let ou = dir.join("boutique");
    copier(&Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/proto/boutique"), &ou);
    let chemin = ou.join("backend.json");
    let mut m: Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
    let racine = std::env::var("RAG3DB_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."));
    m["vector_extension"] =
        json!(racine.join("extension/vector/build/libvector.rag3db_extension").to_string_lossy());
    std::fs::write(&chemin, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    chemin
}

/// Le dossier du démon d'un témoin, et ce qu'on y lit.
struct Demon {
    dir: PathBuf,
}

impl Demon {
    fn new(racine: &Path) -> Self {
        let dir = racine.join("demon");
        std::fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    fn fiche(&self) -> Option<Value> {
        let octets = std::fs::read(self.dir.join("daemon.json")).ok()?;
        serde_json::from_slice(&octets).ok()
    }

    fn pid(&self) -> Option<u32> {
        self.fiche()?["pid"].as_u64().map(|p| p as u32)
    }

    /// `GET /etat`, avec le jeton de `daemon.json`.
    fn etat(&self) -> Value {
        let f = self.fiche().expect("daemon.json");
        reqwest::blocking::Client::new()
            .get(format!("http://{}/etat", f["address"].as_str().unwrap()))
            .bearer_auth(f["token"].as_str().unwrap())
            .send()
            .and_then(|r| r.json())
            .expect("l'état du démon")
    }

    /// Combien de fois un hôte a démarré dans ce dossier : une ligne par
    /// démarrage dans son journal.
    fn demarrages(&self) -> usize {
        std::fs::read_to_string(self.dir.join("daemon.log"))
            .unwrap_or_default()
            .lines()
            .filter(|l| l.starts_with("[démon] rag3weaver-daemon"))
            .count()
    }

    fn attendre_la_fin(pid: u32) {
        let debut = Instant::now();
        while vivant(pid) {
            assert!(debut.elapsed() < Duration::from_secs(30), "le pid {pid} vit encore");
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for Demon {
    /// L'hôte survit à ses clients : le témoin l'arrête lui-même.
    fn drop(&mut self) {
        let Some(f) = self.fiche() else { return };
        let _ = reqwest::blocking::Client::new()
            .post(format!("http://{}/arret", f["address"].as_str().unwrap_or("")))
            .bearer_auth(f["token"].as_str().unwrap_or(""))
            .send();
        if let Some(pid) = f["pid"].as_u64() {
            let debut = Instant::now();
            while vivant(pid as u32) && debut.elapsed() < Duration::from_secs(30) {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

fn vivant(pid: u32) -> bool {
    // Un zombie n'est pas vivant : l'hôte est détaché, mais un enfant du test
    // tué reste zombie tant qu'il n'est pas attendu.
    match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(s) => !s.split(") ").nth(1).is_some_and(|reste| reste.starts_with('Z')),
        Err(_) => false,
    }
}

/// Un client MCP lancé comme `claude mcp add` le lance : sans `--daemon`.
struct Client {
    enfant: Child,
    entree: ChildStdin,
    sortie: BufReader<std::process::ChildStdout>,
    id: u64,
}

impl Client {
    fn lancer(manifeste: &Path, demon: &Demon, adresse: &str) -> Self {
        let mut enfant = Command::new(env!("CARGO_BIN_EXE_rag3weaver-backend"))
            .args(["mcp", "--manifest", &manifeste.to_string_lossy()])
            .env("RAG3WEAVER_DAEMON_DIR", &demon.dir)
            .env("RAG3WEAVER_DAEMON_ADDRESS", adresse)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("lancer le client MCP");
        let entree = enfant.stdin.take().unwrap();
        let sortie = BufReader::new(enfant.stdout.take().unwrap());
        Self { enfant, entree, sortie, id: 0 }
    }

    fn demander(&mut self, methode: &str, params: Value) -> Value {
        self.id += 1;
        let requete = json!({"jsonrpc": "2.0", "id": self.id, "method": methode, "params": params});
        writeln!(self.entree, "{requete}").unwrap();
        self.entree.flush().unwrap();
        let mut ligne = String::new();
        let lus = self.sortie.read_line(&mut ligne).expect("lire la réponse");
        assert!(lus > 0, "le client n'a rien répondu à {requete}");
        serde_json::from_str(&ligne).unwrap_or_else(|e| panic!("réponse illisible ({e}) : {ligne}"))
    }

    fn initialiser(&mut self) -> Value {
        self.demander(
            "initialize",
            json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "le témoin", "version": "0"}}),
        )
    }

    fn outils(&mut self) -> Vec<String> {
        let r = self.demander("tools/list", json!({}));
        r["result"]["tools"]
            .as_array()
            .map(|a| a.iter().filter_map(|t| t["name"].as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    }

    fn appeler(&mut self, nom: &str, args: Value) -> (bool, String) {
        let r = self.demander("tools/call", json!({"name": nom, "arguments": args}));
        let texte = r["result"]["content"][0]["text"].as_str().unwrap_or_default().to_string();
        (r["result"]["isError"] == true, texte)
    }

    fn fermer(mut self) {
        drop(self.entree);
        let debut = Instant::now();
        loop {
            if let Ok(Some(_)) = self.enfant.try_wait() {
                return;
            }
            if debut.elapsed() > Duration::from_secs(30) {
                let _ = self.enfant.kill();
                let _ = self.enfant.wait();
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

const LIBRE: &str = "127.0.0.1:0";

/// **(1) Sans hôte, le client le lance, et un appel répond** — puis l'hôte
/// survit au client qui l'a lancé.
#[test]
#[ignore = "e2e : lance le client MCP, qui lance l'hôte"]
fn sans_hote_le_client_lance_l_hote_et_un_appel_repond() {
    let tmp = tempfile::tempdir().unwrap();
    let manifeste = boutique(tmp.path());
    let demon = Demon::new(tmp.path());
    assert!(demon.fiche().is_none());

    let mut c = Client::lancer(&manifeste, &demon, LIBRE);
    assert!(c.initialiser().get("result").is_some());
    let outils = c.outils();
    assert!(outils.iter().any(|o| o == "daemon_status"), "l'outil d'état est là : {outils:?}");
    assert!(outils.iter().any(|o| o == "put_category"), "les outils du manifeste aussi : {outils:?}");
    let (erreur, texte) = c.appeler("put_category", json!({"record": {"key": "outils", "name": "Les outils"}}));
    assert!(!erreur, "{texte}");
    let (erreur, texte) = c.appeler("list_categories", json!({}));
    assert!(!erreur && texte.contains("outils"), "{texte}");

    let fiche = demon.fiche().expect("l'hôte lancé a écrit daemon.json");
    for champ in ["address", "pid", "token", "build"] {
        assert!(fiche.get(champ).is_some_and(|v| !v.is_null()), "daemon.json porte {champ} : {fiche}");
    }
    let pid = demon.pid().unwrap();
    c.fermer();
    assert!(vivant(pid), "l'hôte survit au client qui l'a lancé");
    assert_eq!(demon.etat()["sessions"].as_array().map(Vec::len), Some(0), "la session du client est fermée");
}

/// **(2) Deux clients lancés ensemble, sans hôte : un seul hôte**, deux
/// sessions à son état.
#[test]
#[ignore = "e2e : deux clients MCP, qui assurent l'hôte en même temps"]
fn deux_clients_ensemble_sans_hote_font_un_seul_hote() {
    let tmp = tempfile::tempdir().unwrap();
    let manifeste = boutique(tmp.path());
    let demon = Demon::new(tmp.path());

    let mut a = Client::lancer(&manifeste, &demon, LIBRE);
    let mut b = Client::lancer(&manifeste, &demon, LIBRE);
    let (ra, rb) = std::thread::scope(|s| {
        let ta = s.spawn(|| a.initialiser());
        let tb = s.spawn(|| b.initialiser());
        (ta.join().unwrap(), tb.join().unwrap())
    });
    assert!(ra.get("result").is_some() && rb.get("result").is_some());
    let _ = a.outils();
    let _ = b.outils();
    assert_eq!(demon.demarrages(), 1, "un seul hôte a démarré");
    let etat = demon.etat();
    assert_eq!(etat["sessions"].as_array().map(Vec::len), Some(2), "deux sessions : {etat}");
    assert_eq!(etat["backends"].as_array().map(Vec::len), Some(1), "un seul backend : {etat}");
    a.fermer();
    b.fermer();
}

/// **(3) L'hôte tué (`kill -TERM`) : l'appel suivant le relance**, rouvre sa
/// session, et retrouve la base.
#[test]
#[ignore = "e2e : tue l'hôte entre deux appels"]
fn l_hote_tue_est_relance_par_l_appel_suivant() {
    let tmp = tempfile::tempdir().unwrap();
    let manifeste = boutique(tmp.path());
    let demon = Demon::new(tmp.path());

    let mut c = Client::lancer(&manifeste, &demon, LIBRE);
    c.initialiser();
    let (erreur, texte) = c.appeler("put_category", json!({"record": {"key": "outils", "name": "Les outils"}}));
    assert!(!erreur, "{texte}");
    let premier = demon.pid().unwrap();
    assert!(Command::new("kill").args(["-TERM", &premier.to_string()]).status().unwrap().success());
    Demon::attendre_la_fin(premier);
    assert!(demon.fiche().is_none(), "un hôte arrêté par SIGTERM retire daemon.json");

    let (erreur, texte) = c.appeler("list_categories", json!({}));
    assert!(!erreur, "l'appel suivant relance l'hôte : {texte}");
    assert!(texte.contains("outils"), "et retrouve la base : {texte}");
    let second = demon.pid().expect("un nouvel hôte a écrit daemon.json");
    assert_ne!(second, premier);
    assert_eq!(demon.demarrages(), 2);
    c.fermer();
}

/// **(4) L'outil `daemon_status`, sans hôte et avec.** Sans hôte — et quand le
/// lancement échoue, ici parce que le port est pris — le client démarre quand
/// même, dit pourquoi, et n'expose que cet outil ; les autres appels sont des
/// erreurs dites au modèle. Avec l'hôte, l'outil dit sa version et ses sessions.
#[test]
#[ignore = "e2e : l'outil d'état du démon"]
fn l_etat_du_demon_sans_hote_et_avec() {
    let tmp = tempfile::tempdir().unwrap();
    let manifeste = boutique(tmp.path());
    let demon = Demon::new(tmp.path());

    // Sans hôte possible : le port voulu est occupé par autre chose.
    let occupe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let adresse = occupe.local_addr().unwrap().to_string();
    let mut c = Client::lancer(&manifeste, &demon, &adresse);
    let r = c.initialiser();
    assert!(r.get("result").is_some(), "le client démarre sans hôte : {r}");
    let instructions = r["result"]["instructions"].as_str().unwrap_or_default();
    assert!(!instructions.is_empty(), "initialize dit pourquoi : {r}");
    assert_eq!(c.outils(), vec!["daemon_status".to_string()], "seul l'outil d'état sans hôte");
    let (erreur, texte) = c.appeler("daemon_status", json!({}));
    assert!(!erreur, "l'état se dit, ce n'est pas une erreur : {texte}");
    assert!(texte.contains("absent") || texte.contains("injoignable"), "{texte}");
    assert!(texte.contains("daemon.log"), "il dit où lire : {texte}");
    let (erreur, texte) = c.appeler("list_categories", json!({}));
    assert!(erreur, "un autre outil sans hôte est une erreur dite au modèle : {texte}");
    c.fermer();
    drop(occupe);

    // Avec l'hôte.
    let mut c = Client::lancer(&manifeste, &demon, LIBRE);
    c.initialiser();
    let (erreur, texte) = c.appeler("daemon_status", json!({}));
    assert!(!erreur, "{texte}");
    let etat: Value = serde_json::from_str(&texte).unwrap_or_else(|e| panic!("{e} : {texte}"));
    assert_eq!(etat["state"], "running", "{etat}");
    assert!(etat["build"].as_str().is_some_and(|b| !b.is_empty()), "{etat}");
    assert_eq!(etat["sessions"].as_array().map(Vec::len), Some(1), "{etat}");
    c.fermer();
}

/// **(5) Un `daemon.json` périmé (pid mort) est ignoré et réécrit.**
#[test]
#[ignore = "e2e : daemon.json d'un hôte mort"]
fn un_daemon_json_perime_est_ignore_et_reecrit() {
    let tmp = tempfile::tempdir().unwrap();
    let manifeste = boutique(tmp.path());
    let demon = Demon::new(tmp.path());

    let mut mort = Command::new("true").spawn().unwrap();
    let pid_mort = mort.id();
    mort.wait().unwrap();
    std::fs::write(
        demon.dir.join("daemon.json"),
        serde_json::to_vec(&json!({"address": "127.0.0.1:1", "pid": pid_mort, "token": "périmé", "build": "ancien"}))
            .unwrap(),
    )
    .unwrap();

    let mut c = Client::lancer(&manifeste, &demon, LIBRE);
    c.initialiser();
    let (erreur, texte) = c.appeler("list_categories", json!({}));
    assert!(!erreur, "{texte}");
    let fiche = demon.fiche().unwrap();
    assert_ne!(fiche["pid"].as_u64(), Some(pid_mort as u64), "réécrit par le nouvel hôte : {fiche}");
    assert_ne!(fiche["token"], "périmé");
    c.fermer();
}
