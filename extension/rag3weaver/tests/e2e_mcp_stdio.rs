//! **Le serveur MCP, éprouvé en parlant son protocole sur stdio.**
//!
//! Les huit tests de `src/mcp.rs` éprouvent le protocole contre un hôte de
//! fantaisie : ils disent que les réponses ont la bonne forme. Ils ne disent
//! pas que le **vrai** serveur démarre, qu'il expose les outils du **vrai**
//! manifeste, ni qu'un appel traverse `run_tool` et son bac à sable. C'est ce
//! que celui-ci fait, en lançant le binaire et en lui écrivant du JSON.
//!
//! **Ce qu'il prouve, et que rien d'autre ne peut prouver :**
//!
//! 1. la liste d'outils vient du **manifeste**, pas d'une liste écrite à la
//!    main dans `mcp.rs`. C'est l'argument de toute l'architecture, et un
//!    gabarit à douze outils doit en exposer douze, nommés ;
//! 2. un refus arrive **au modèle**. Un chemin hors du workspace est refusé par
//!    son nom, dans `content` avec `isError`, et non en erreur JSON-RPC — qu'un
//!    modèle ne voit pas.
//!
//! **Ce qu'il ne prouve pas, et pourquoi.** Il ne cherche pas. Chercher demande
//! un index et donc un service d'embarquement : ce serait une passe qui
//! embarque, à régime doux, pour une question que les suites de recherche
//! traitent déjà mieux. La recherche par MCP est éprouvée par l'épreuve réelle
//! (une session Claude Code sur ce dépôt), pas ici.
//!
//! **Le gabarit est joué tel qu'il est livré, et il exige un embarqueur.**
//! J'avais d'abord retiré `workspace.index` du manifeste pour m'en passer —
//! c'est lui qui rend `needs_embeddings()` vrai. Deux passes ont montré que
//! c'était faux : **toute** la surface de code passe par l'entité `File`, que
//! `register_code_schema` déclare, et sans elle même `read_file` répond
//! « unknown entity: File ». Il faut donc `RAG3WEAVER_EMBED_SERVICE` ; aucun
//! vecteur n'est calculé ici, on ne fait que se connecter.
//!
//! Et ce refus mérite d'être signalé pour lui-même : « unknown entity: File »
//! ne dit pas à un modèle « cette base n'est pas indexée, appelle `index`
//! d'abord ». Il lira un défaut d'outil là où il y a un ordre à suivre.
//!
//! ```bash
//! ./run_e2e.sh --test e2e_mcp_stdio
//! ```

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};

use serde_json::{json, Value};

/// Le gabarit de code, rendu jouable depuis n'importe où.
///
/// Les chemins d'un gabarit livré sont relatifs à lui, et son `database` est un
/// emplacement que le produit remplit — la même réécriture que le filet des
/// gabarits (`src/gabarits.rs`), par **clé** et non par place.
fn manifeste_jouable(ou: &Path) -> PathBuf {
    let gabarit = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
    let brut = std::fs::read(gabarit.join("backend.json")).expect("le gabarit code");
    let mut m: Value = serde_json::from_slice(&brut).expect("un manifeste lisible");

    fn absolutiser(v: &mut Value, gabarit: &Path) {
        const CLES: &[&str] = &["graph", "schema", "input_schema"];
        match v {
            Value::Object(o) => {
                for (k, val) in o.iter_mut() {
                    let par_la_cle = CLES.contains(&k.as_str());
                    if let Some(s) = val.as_str() {
                        let par_l_extension = s.ends_with(".rhai");
                        if (par_la_cle || par_l_extension) && !s.starts_with('/') {
                            *val = Value::String(gabarit.join(s).to_string_lossy().to_string());
                            continue;
                        }
                    }
                    absolutiser(val, gabarit);
                }
            }
            Value::Array(a) => a.iter_mut().for_each(|val| absolutiser(val, gabarit)),
            _ => {}
        }
    }

    let atelier = ou.join("atelier");
    std::fs::create_dir_all(&atelier).expect("l'atelier");
    std::fs::write(atelier.join("port.rs"), "pub fn ouvrir() {}\n").expect("un fichier");

    m["database"] = json!(ou.join("base.rag3db").to_string_lossy());
    if let Some(ext) = m.get("vector_extension").and_then(Value::as_str) {
        if !ext.starts_with('/') {
            let racine = std::env::var("RAG3DB_ROOT").map(PathBuf::from).unwrap_or_else(|_| {
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
            });
            m["vector_extension"] =
                json!(racine.join("extension/vector/build/libvector.rag3db_extension")
                    .to_string_lossy());
        }
    }
    if let Some(w) = m.get_mut("workspace").and_then(Value::as_object_mut) {
        w["root"] = json!(atelier.to_string_lossy());
    }
    absolutiser(&mut m, &gabarit);

    let chemin = ou.join("backend.json");
    std::fs::write(&chemin, serde_json::to_vec_pretty(&m).unwrap()).expect("écrire le manifeste");
    chemin
}

/// Un serveur lancé, et de quoi lui parler.
struct Serveur {
    enfant: Child,
    entree: ChildStdin,
    sortie: BufReader<std::process::ChildStdout>,
}

impl Serveur {
    fn lancer(manifeste: &Path, cles: &[&str]) -> Self {
        Self::lancer_avec(manifeste, cles, None)
    }

    /// Un serveur MCP, propriétaire de sa base (`hote` vide : `--standalone`)
    /// ou **client mince** de l'hôte du démon (`--daemon <adresse>`, marche 3 de
    /// la forme C), le jeton lu dans le dossier de démon de cet hôte.
    fn lancer_avec(manifeste: &Path, cles: &[&str], hote: Option<&Hote>) -> Self {
        let mut commande = Command::new(env!("CARGO_BIN_EXE_rag3weaver-backend"));
        commande.args(["mcp", "--manifest", &manifeste.to_string_lossy()]);
        if !cles.is_empty() {
            commande.args(["--keys", &cles.join(",")]);
        }
        match hote {
            Some(h) => {
                commande.args(["--daemon", &h.adresse]).env("RAG3WEAVER_DAEMON_DIR", h.dossier.path());
            }
            None => {
                commande.arg("--standalone");
            }
        }
        let mut enfant = commande
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("lancer le serveur MCP");
        let entree = enfant.stdin.take().expect("stdin");
        let sortie = BufReader::new(enfant.stdout.take().expect("stdout"));
        Self { enfant, entree, sortie }
    }

    /// Une requête, une réponse. **Les notifications ne passent pas par là** :
    /// attendre une réponse qui ne viendra jamais ferait pendre le test, ce qui
    /// est la façon la plus coûteuse de découvrir une règle du protocole.
    fn demander(&mut self, requete: Value) -> Value {
        writeln!(self.entree, "{requete}").expect("écrire la requête");
        self.entree.flush().expect("vider");
        let mut ligne = String::new();
        let lus = self.sortie.read_line(&mut ligne).expect("lire la réponse");
        assert!(lus > 0, "le serveur n'a rien répondu à {requete}");
        serde_json::from_str(&ligne).unwrap_or_else(|e| panic!("réponse illisible ({e}) : {ligne}"))
    }

    fn notifier(&mut self, notification: Value) {
        writeln!(self.entree, "{notification}").expect("écrire la notification");
        self.entree.flush().expect("vider");
    }
}

impl Drop for Serveur {
    fn drop(&mut self) {
        let _ = self.enfant.kill();
        let _ = self.enfant.wait();
    }
}

#[test]
#[ignore = "e2e : lance le binaire et ouvre une base"]
fn un_client_mcp_voit_les_outils_du_manifeste_et_les_appelle() {
    let tmp = std::env::temp_dir().join(format!("rag3weaver-mcp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("dossier temporaire");
    let manifeste = manifeste_jouable(&tmp);

    let mut s = Serveur::lancer(&manifeste, &["dev"]);
    jouer(&mut s);
    drop(s);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// **Le même témoin, par l'hôte du démon, sans changer une assertion**
/// (marche 3 de la forme C) : le serveur MCP ne possède plus rien, il parle à
/// l'hôte. Si une assertion de `jouer` devait changer pour passer ici, c'est
/// que la couture `Hote` n'aurait pas tenu.
#[test]
#[ignore = "e2e : lance l'hôte du démon et un client mince"]
fn le_meme_client_mcp_par_l_hote_du_demon() {
    let tmp = std::env::temp_dir().join(format!("rag3weaver-mcp-hote-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("dossier temporaire");
    let manifeste = manifeste_jouable(&tmp);
    let hote = Hote::lancer();
    let mut s = Serveur::lancer_avec(&manifeste, &["dev"], Some(&hote));
    jouer(&mut s);
    drop(s);
    let etat = hote.etat();
    assert_eq!(etat["backends"].as_array().map(Vec::len), Some(1), "un backend ouvert par l'hôte : {etat}");
    drop(hote);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// **Deux sessions MCP sur le même backend, par l'hôte** — le cas qui compte
/// pour Lucie (« plusieurs sessions sur le même projet »). Une base, un
/// écrivain : l'hôte. Ce qu'écrit une session, l'autre le lit ; chacune a sa
/// session à l'hôte, et l'état du démon les dit toutes les deux.
#[test]
#[ignore = "e2e : lance l'hôte du démon et deux clients minces"]
fn deux_sessions_mcp_partagent_un_backend_par_l_hote() {
    let tmp = tempfile::tempdir().expect("dossier");
    let manifeste = boutique(tmp.path());
    let hote = Hote::lancer();
    let mut a = Serveur::lancer_avec(&manifeste, &[], Some(&hote));
    let mut b = Serveur::lancer_avec(&manifeste, &[], Some(&hote));
    for (s, nom) in [(&mut a, "a"), (&mut b, "b")] {
        let r = s.demander(json!({
            "jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":nom,"version":"0"}}
        }));
        assert!(r.get("result").is_some(), "{r}");
    }
    let etat = hote.etat();
    assert_eq!(etat["sessions"].as_array().map(Vec::len), Some(2), "deux sessions à l'hôte : {etat}");
    assert_eq!(etat["backends"].as_array().map(Vec::len), Some(1), "un seul backend, ouvert une fois : {etat}");

    let r = a.demander(json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"put_category","arguments":{"record":{"key":"outils","name":"Les outils"}}}
    }));
    assert_eq!(r["result"]["isError"], false, "la session a écrit : {r}");
    let r = b.demander(json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"list_categories","arguments":{}}
    }));
    assert_eq!(r["result"]["isError"], false, "{r}");
    let texte = r["result"]["content"][0]["text"].as_str().unwrap_or_default();
    assert!(texte.contains("outils"), "la session b lit ce que la session a a écrit : {texte}");

    drop(a);
    drop(b);
    drop(hote);
}

/// **L'ancien `--demon` est refusé en le disant** (page C, §3) : il ne
/// déportait que la connexion, et N sessions y faisaient N réacteurs. Pas un
/// synonyme silencieux de `--daemon`, qui veut dire l'inverse.
#[test]
#[ignore = "e2e : lance le binaire"]
fn l_ancien_drapeau_demon_est_refuse_en_nommant_le_remplacant() {
    let sortie = Command::new(env!("CARGO_BIN_EXE_rag3weaver-backend"))
        .args(["mcp", "--manifest", "nimporte.json", "--demon", "127.0.0.1:1"])
        .stdin(Stdio::null())
        .output()
        .expect("lancer le binaire");
    assert!(!sortie.status.success(), "refusé");
    let erreurs = String::from_utf8_lossy(&sortie.stderr);
    assert!(
        erreurs.contains("--demon est retiré") && erreurs.contains("--daemon <adresse>"),
        "le refus nomme le remplaçant et le changement de sens : {erreurs}"
    );
}

/// Le gabarit `proto/boutique`, qui s'ouvre sans service d'embarquement.
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
    copier(&Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/proto/boutique"), dir);
    let chemin = dir.join("backend.json");
    let mut m: Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
    let racine = std::env::var("RAG3DB_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."));
    m["vector_extension"] =
        json!(racine.join("extension/vector/build/libvector.rag3db_extension").to_string_lossy());
    std::fs::write(&chemin, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    chemin
}

/// **L'hôte du démon**, lancé pour un témoin sur un port libre, arrêté par
/// `POST /arret` à la fin (il ferme ses bases proprement).
struct Hote {
    enfant: Child,
    adresse: String,
    /// Son dossier de démon : `daemon.json` (et le jeton) y est écrit, jamais
    /// dans celui de la personne.
    dossier: tempfile::TempDir,
}

impl Hote {
    fn lancer() -> Self {
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let adresse = format!("127.0.0.1:{port}");
        let dossier = tempfile::tempdir().expect("dossier du démon");
        let enfant = Command::new(env!("CARGO_BIN_EXE_rag3weaver-backend"))
            .args(["daemon", "--address", &adresse])
            .env("RAG3WEAVER_DAEMON_DIR", dossier.path())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("lancer l'hôte du démon");
        let debut = std::time::Instant::now();
        while reqwest::blocking::get(format!("http://{adresse}/sante")).is_err() {
            assert!(debut.elapsed() < std::time::Duration::from_secs(30), "l'hôte ne répond pas sur {adresse}");
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Self { enfant, adresse, dossier }
    }

    /// Le jeton local de cet hôte, dans son `daemon.json`.
    fn jeton(&self) -> String {
        let debut = std::time::Instant::now();
        loop {
            if let Some(t) = std::fs::read(self.dossier.path().join("daemon.json"))
                .ok()
                .and_then(|o| serde_json::from_slice::<Value>(&o).ok())
                .and_then(|v| v["token"].as_str().map(str::to_string))
            {
                return t;
            }
            assert!(debut.elapsed() < std::time::Duration::from_secs(30), "pas de daemon.json");
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    fn etat(&self) -> Value {
        reqwest::blocking::Client::new()
            .get(format!("http://{}/etat", self.adresse))
            .bearer_auth(self.jeton())
            .send()
            .and_then(|r| r.json())
            .expect("l'état du démon")
    }
}

impl Drop for Hote {
    fn drop(&mut self) {
        let _ = reqwest::blocking::Client::new()
            .post(format!("http://{}/arret", self.adresse))
            .bearer_auth(self.jeton())
            .send();
        let debut = std::time::Instant::now();
        while debut.elapsed() < std::time::Duration::from_secs(30) {
            if let Ok(Some(_)) = self.enfant.try_wait() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let _ = self.enfant.kill();
        let _ = self.enfant.wait();
    }
}

/// Le corps du témoin, joué tel quel contre un serveur propriétaire de sa base
/// et contre un client mince de l'hôte.
fn jouer(s: &mut Serveur) {
    // ── 1. La poignée de main ────────────────────────────────────────────
    let r = s.demander(json!({
        "jsonrpc":"2.0","id":1,"method":"initialize",
        "params":{"protocolVersion":"2025-06-18","capabilities":{},
                  "clientInfo":{"name":"le test","version":"0"}}
    }));
    assert_eq!(r["result"]["protocolVersion"], "2025-06-18", "{r}");
    assert_eq!(r["result"]["serverInfo"]["keys"][0], "dev", "les clés présentées se disent : {r}");
    assert!(r["result"]["capabilities"]["tools"].is_object(), "{r}");

    // Une notification n'attend aucune réponse : si le serveur en envoyait une,
    // elle serait lue par la requête suivante et ferait échouer son `id`.
    s.notifier(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));

    // ── 2. La liste vient du manifeste ───────────────────────────────────
    let r = s.demander(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}));
    assert_eq!(r["id"], 2, "une notification a-t-elle reçu une réponse ? {r}");
    let outils = r["result"]["tools"].as_array().expect("une liste").clone();
    let noms: Vec<&str> = outils.iter().filter_map(|t| t["name"].as_str()).collect();
    // **La liste exacte du gabarit `code`**, pas « au moins quelques outils » :
    // un filet qui accepte n'importe quelle liste n'attrape pas une liste
    // écrite à la main.
    for attendu in [
        "search_code", "read_file", "grep_files", "list_files", "schema",
        "edit_file", "run_command", "wait_output", "estimate", "index",
        "usages", "impact", "callees",
    ] {
        assert!(noms.contains(&attendu), "{attendu} manque : {noms:?}");
    }
    // **Les noms exigés, le compte NON figé.** La première version écrivait
    // `assert_eq!(noms.len(), 12)` ; `callees` est arrivé et quelqu'un a dû la
    // passer à 13. Un filet qui interdit d'ajouter un outil ne protège rien :
    // ce qu'il doit garantir, c'est que la liste vient du **manifeste**, donc
    // que tous les outils déclarés sont là. Le prochain outil n'aura pas à
    // toucher ce test.
    assert!(
        noms.len() >= 13,
        "le gabarit code en déclare au moins treize ; un outil manque : {noms:?}"
    );
    // Chaque outil porte son schéma, et il est typé.
    let lecture = outils.iter().find(|t| t["name"] == "read_file").expect("read_file");
    assert_eq!(lecture["inputSchema"]["type"], "object", "{lecture}");
    assert!(
        lecture["inputSchema"]["properties"].is_object(),
        "un schéma sans propriétés ne guide personne : {lecture}"
    );
    assert!(
        lecture["description"].as_str().is_some_and(|d| !d.is_empty()),
        "un outil sans description est un outil qu'un modèle n'appellera pas : {lecture}"
    );

    // ── 3. Un appel traverse run_tool et le bac à sable ──────────────────
    // **Lu, cette fois, et non supposé.** Deux interfaces inventées m'ont
    // refusée avant d'arriver ici, et la seconde apprend quelque chose sur le
    // gabarit : `list_files` ne lit pas l'arbre de travail, il liste les
    // fichiers **indexés** (l'entité `File`). Sur une base neuve il répond
    // « ListFilesNode: unknown entity: File ». Les outils qui lisent vraiment
    // le disque sont ceux de la capacité `read_files` — `read_file`,
    // `grep_files` —, et c'est par eux qu'un appel se prouve sans index.
    let r = s.demander(json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"read_file","arguments":{"path":"port.rs"}}
    }));
    assert_eq!(r["result"]["isError"], false, "{r}");
    let texte = r["result"]["content"][0]["text"].as_str().unwrap_or_default();
    assert!(
        texte.contains("pub fn ouvrir"),
        "l'appel traverse run_tool et le bac à sable, et rend le fichier : {texte}"
    );

    // ── 3 bis. Un argument qui n'existe pas est refusé par son nom ────────
    //
    // **Ce cas est né d'une erreur à moi** : j'avais appelé `list_files` avec
    // `path` alors qu'il déclare `path_prefix`, et le serveur a répondu
    // « unknown argument path » dans `content` avec `isError`. C'est la faute
    // qu'un modèle commet le plus souvent, et c'est exactement le chemin par
    // lequel il doit l'apprendre — donc le cas reste, au lieu d'être corrigé
    // et oublié.
    let r = s.demander(json!({
        "jsonrpc":"2.0","id":31,"method":"tools/call",
        "params":{"name":"read_file","arguments":{"chemin":"port.rs"}}
    }));
    assert!(r["error"].is_null(), "un mauvais argument n'est pas une erreur de protocole : {r}");
    assert_eq!(r["result"]["isError"], true, "{r}");
    let refus = r["result"]["content"][0]["text"].as_str().unwrap_or_default();
    assert!(
        refus.contains("chemin"),
        "le refus doit nommer l'argument fautif pour qu'un modèle se corrige : {refus}"
    );

    // ── 4. Un refus arrive au modèle ─────────────────────────────────────
    //
    // Le point qui compte : **dans `content` avec `isError`**, pas en erreur
    // JSON-RPC — qu'un modèle ne voit pas. Tous nos refus nommés ne servent à
    // rien si le modèle ne les lit pas.
    let r = s.demander(json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"read_file","arguments":{"path":"../../../etc/passwd"}}
    }));
    assert!(r["error"].is_null(), "un refus d'outil n'est pas une erreur de protocole : {r}");
    assert_eq!(r["result"]["isError"], true, "{r}");
    let refus = r["result"]["content"][0]["text"].as_str().unwrap_or_default();
    assert!(!refus.trim().is_empty(), "un refus muet ne vaut rien : {r}");
    println!("▸ le refus dit : {refus}");

    // ── 5. Un outil hors manifeste n'existe pas ──────────────────────────
    let r = s.demander(json!({
        "jsonrpc":"2.0","id":5,"method":"tools/call",
        "params":{"name":"rm_rf","arguments":{}}
    }));
    assert_eq!(r["error"]["code"], -32602, "{r}");
}
