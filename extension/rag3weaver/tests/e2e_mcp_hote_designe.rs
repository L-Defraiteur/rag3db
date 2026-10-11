//! **Le client vers un hôte désigné** (marche 5 de la forme C, première part) :
//! l'hôte tourne ailleurs — sur luciepc, joint par un tunnel ssh —, son
//! `daemon.json` n'est pas le daemon.json local, et son adresse n'est pas celle
//! du tunnel. Le client lit l'adresse et le jeton dans un `daemon.json` désigné
//! (`--daemon-json <chemin>`), ou rendu par une commande
//! (`--daemon-json-command <commande>`, par exemple `ssh luciepc cat …`) ;
//! `--daemon <adresse>` remplace l'adresse (le tunnel). Le jeton n'est jamais
//! un argument de ligne de commande. Sur un refus de jeton (l'hôte a
//! redémarré, son jeton a changé), le client relit sa source et rejoue une fois.
//!
//! Le témoin lance un hôte dans son propre dossier de démon ; le client a un
//! autre dossier, vide : rien ne se trouve par la découverte locale.

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

/// L'hôte « distant » : son dossier, son port fixe (le tunnel y mène).
struct Hote {
    dossier: PathBuf,
    port: u16,
    enfant: Option<Child>,
}

impl Hote {
    fn lancer(dossier: &Path, port: u16) -> Self {
        let mut h = Self { dossier: dossier.to_path_buf(), port, enfant: None };
        h.demarrer();
        h
    }

    fn demarrer(&mut self) {
        let _ = std::fs::remove_file(self.dossier.join("daemon.json"));
        let enfant = Command::new(env!("CARGO_BIN_EXE_rag3weaver-backend"))
            .args(["daemon", "--address", &format!("127.0.0.1:{}", self.port)])
            .env("RAG3WEAVER_DAEMON_DIR", &self.dossier)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("lancer l'hôte");
        self.enfant = Some(enfant);
        let debut = Instant::now();
        while self.fiche().is_none() {
            assert!(debut.elapsed() < Duration::from_secs(30), "l'hôte n'a pas écrit daemon.json");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn fiche(&self) -> Option<Value> {
        serde_json::from_slice(&std::fs::read(self.dossier.join("daemon.json")).ok()?).ok()
    }

    fn arreter(&mut self) {
        if let Some(f) = self.fiche() {
            let _ = reqwest::blocking::Client::new()
                .post(format!("http://127.0.0.1:{}/arret", self.port))
                .bearer_auth(f["token"].as_str().unwrap_or(""))
                .send();
        }
        if let Some(mut e) = self.enfant.take() {
            let debut = Instant::now();
            while debut.elapsed() < Duration::from_secs(30) {
                if let Ok(Some(_)) = e.try_wait() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            let _ = e.kill();
            let _ = e.wait();
        }
    }

    /// Redémarrer : un nouveau jeton, comme l'unité de luciepc le fait.
    fn redemarrer(&mut self) {
        let avant = self.fiche().unwrap()["token"].clone();
        self.arreter();
        self.demarrer();
        assert_ne!(self.fiche().unwrap()["token"], avant, "un nouveau jeton à chaque démarrage");
    }
}

impl Drop for Hote {
    fn drop(&mut self) {
        self.arreter();
    }
}

struct Client {
    enfant: Child,
    entree: ChildStdin,
    sortie: BufReader<std::process::ChildStdout>,
    id: u64,
}

impl Client {
    fn lancer(manifeste: &Path, dossier_local: &Path, args: &[&str]) -> Self {
        let mut enfant = Command::new(env!("CARGO_BIN_EXE_rag3weaver-backend"))
            .args(["mcp", "--manifest", &manifeste.to_string_lossy()])
            .args(args)
            .env("RAG3WEAVER_DAEMON_DIR", dossier_local)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("lancer le client");
        let entree = enfant.stdin.take().unwrap();
        let sortie = BufReader::new(enfant.stdout.take().unwrap());
        let mut c = Self { enfant, entree, sortie, id: 0 };
        let r = c.demander(
            "initialize",
            json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "désigné", "version": "0"}}),
        );
        assert!(r.get("result").is_some(), "{r}");
        c
    }

    fn demander(&mut self, methode: &str, params: Value) -> Value {
        self.id += 1;
        writeln!(self.entree, "{}", json!({"jsonrpc": "2.0", "id": self.id, "method": methode, "params": params})).unwrap();
        self.entree.flush().unwrap();
        let mut ligne = String::new();
        assert!(self.sortie.read_line(&mut ligne).unwrap() > 0, "pas de réponse à {methode}");
        serde_json::from_str(&ligne).unwrap()
    }

    fn appeler(&mut self, nom: &str, args: Value) -> (bool, String) {
        let r = self.demander("tools/call", json!({"name": nom, "arguments": args}));
        (r["result"]["isError"] == true, r["result"]["content"][0]["text"].as_str().unwrap_or_default().to_string())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.enfant.kill();
        let _ = self.enfant.wait();
    }
}

fn port_libre() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

/// **Un hôte désigné, son adresse remplacée par le tunnel, son jeton relu
/// quand il redémarre.**
#[test]
#[ignore = "e2e : un hôte désigné par son daemon.json"]
fn un_hote_designe_par_son_daemon_json_et_son_jeton_relu() {
    let tmp = tempfile::tempdir().unwrap();
    let manifeste = boutique(tmp.path());
    let distant = tmp.path().join("distant");
    let local = tmp.path().join("local");
    std::fs::create_dir_all(&distant).unwrap();
    std::fs::create_dir_all(&local).unwrap();
    let port = port_libre();
    let mut hote = Hote::lancer(&distant, port);

    // La copie du daemon.json d'en face : son adresse est celle de là-bas, pas
    // celle du tunnel.
    let copie = tmp.path().join("luciepc.daemon.json");
    let mut fiche = hote.fiche().unwrap();
    fiche["address"] = json!("127.0.0.1:1");
    std::fs::write(&copie, serde_json::to_vec(&fiche).unwrap()).unwrap();
    let tunnel = format!("127.0.0.1:{port}");
    let commande = format!("cat {}", distant.join("daemon.json").display());

    let mut par_copie = Client::lancer(&manifeste, &local, &["--daemon", &tunnel, "--daemon-json", &copie.to_string_lossy()]);
    let mut par_commande = Client::lancer(&manifeste, &local, &["--daemon", &tunnel, "--daemon-json-command", &commande]);
    let (erreur, texte) = par_copie.appeler("put_category", json!({"record": {"key": "outils", "name": "Les outils"}}));
    assert!(!erreur, "le jeton de la copie ouvre l'hôte désigné : {texte}");
    let (erreur, texte) = par_commande.appeler("list_categories", json!({}));
    assert!(!erreur && texte.contains("outils"), "{texte}");
    let (_, etat) = par_commande.appeler("daemon_status", json!({}));
    assert!(etat.contains("\"running\""), "{etat}");
    assert!(!local.join("daemon.json").exists(), "rien n'a été lancé ni écrit dans le dossier local");

    hote.redemarrer();
    let (erreur, texte) = par_commande.appeler("list_categories", json!({}));
    assert!(!erreur && texte.contains("outils"), "la commande relit le jeton neuf, et l'appel passe : {texte}");
    let (erreur, texte) = par_copie.appeler("list_categories", json!({}));
    assert!(erreur, "la copie figée a l'ancien jeton : refus nommé");
    assert!(texte.contains("jeton") && texte.contains("luciepc.daemon.json"), "le refus nomme le jeton et sa source : {texte}");
}

/// **Une source absente se dit** : le client démarre, n'expose que l'état du
/// démon, et nomme le fichier.
#[test]
#[ignore = "e2e : une source de jeton absente"]
fn une_source_absente_se_dit() {
    let tmp = tempfile::tempdir().unwrap();
    let manifeste = boutique(tmp.path());
    let absent = tmp.path().join("nulle-part.daemon.json");
    let mut c = Client::lancer(&manifeste, tmp.path(), &["--daemon-json", &absent.to_string_lossy()]);
    let (erreur, texte) = c.appeler("daemon_status", json!({}));
    assert!(!erreur, "{texte}");
    assert!(texte.contains("nulle-part.daemon.json"), "l'état nomme la source absente : {texte}");
    let (erreur, texte) = c.appeler("list_categories", json!({}));
    assert!(erreur && texte.contains("nulle-part.daemon.json"), "{texte}");
}
