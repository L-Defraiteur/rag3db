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
//! Et une conséquence de ce choix, écrite parce qu'elle se verrait sinon comme
//! une bizarrerie : le manifeste joué ici **retire `workspace.index`**. C'est
//! lui qui rend `needs_embeddings()` vrai, donc qui exigerait un embarqueur à
//! l'ouverture. Sans index déclaré, le backend s'ouvre partout — et la lecture
//! de fichiers, elle, n'a jamais eu besoin d'un vecteur.
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
        // Voir l'en-tête : sans index déclaré, pas d'embarqueur exigé.
        w.remove("index");
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
        let mut commande = Command::new(env!("CARGO_BIN_EXE_rag3weaver-backend"));
        commande.args(["mcp", "--manifest", &manifeste.to_string_lossy()]);
        if !cles.is_empty() {
            commande.args(["--keys", &cles.join(",")]);
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
        "usages", "impact",
    ] {
        assert!(noms.contains(&attendu), "{attendu} manque : {noms:?}");
    }
    assert_eq!(noms.len(), 12, "le gabarit code en déclare douze : {noms:?}");
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
    let r = s.demander(json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"list_files","arguments":{"path":"."}}
    }));
    assert_eq!(r["result"]["isError"], false, "{r}");
    let texte = r["result"]["content"][0]["text"].as_str().unwrap_or_default();
    assert!(texte.contains("port.rs"), "l'atelier contient port.rs : {texte}");

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

    let _ = std::fs::remove_dir_all(&tmp);
}
