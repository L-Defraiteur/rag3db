//! Persistent backend host. Transport adapters use the same manifest-generated tools.
use rag3weaver::{backend::PreparedBackend, Rag3dbConnection};
use serde_json::{json, Value};
use std::{
    io::{self, BufRead, Write},
    path::Path,
};
/// **Le serveur MCP** : les outils du manifeste, exposés à un client MCP sur
/// stdio. Le protocole est dans [`rag3weaver::mcp`] ; ici on ne fait qu'ouvrir
/// un backend et le lui donner.
///
/// ```text
/// rag3weaver-backend mcp --manifest <backend.json> [--keys dev,admin] [--demon 127.0.0.1:7979]
/// ```
///
/// **Deux modes, et c'est la propriété de la base qui les sépare.**
///
/// Sans `--demon`, le serveur **possède** la base : bon pour un backend de code
/// en lecture, une session. Mais une base rag3db ne s'ouvre que par un seul
/// processus, et Claude Code lance **un processus MCP par session** — donc deux
/// sessions sur la même mémoire seraient deux écrivains, refusés par le moteur.
///
/// Avec `--demon`, le serveur ne possède plus rien : son backend reste local
/// (ses outils et ses schémas ne demandent aucune base) et sa **connexion**
/// passe par `rag3daemon`, qui est le processus unique qui tient la base.
/// N serveurs, un écrivain, et aucun protocole à relayer — `DaemonConnection`
/// est une `DbConnection` comme une autre. C'est la forme produit : installer
/// rag3weaver, c'est installer un démon.
fn servir_mcp(args: &[String]) -> Result<(), String> {
    const USAGE: &str = "usage: rag3weaver-backend mcp --manifest <backend.json> \
                         [--keys dev,admin] [--demon <adresse>]";
    let mut manifeste: Option<String> = None;
    let mut cles: Vec<String> = Vec::new();
    let mut demon: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        let valeur = |i: usize| -> Result<String, String> {
            args.get(i + 1).cloned().ok_or_else(|| format!("{} attend une valeur\n{USAGE}", args[i]))
        };
        match args[i].as_str() {
            "--manifest" => { manifeste = Some(valeur(i)?); i += 2; }
            "--demon" => { demon = Some(valeur(i)?); i += 2; }
            "--keys" => {
                cles = valeur(i)?
                    .split(',')
                    .map(str::trim)
                    .filter(|k| !k.is_empty())
                    .map(str::to_string)
                    .collect();
                i += 2;
            }
            autre => return Err(format!("option inconnue : {autre}\n{USAGE}")),
        }
    }
    let manifeste = manifeste.ok_or_else(|| format!("--manifest est obligatoire\n{USAGE}"))?;
    let prepared = PreparedBackend::load(Path::new(&manifeste))?;
    let database = prepared.path(&prepared.manifest.database);
    if let Some(parent) = database.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let embedder = if prepared.needs_embeddings() {
        Some(prepared.connect_embedder()?.0)
    } else {
        None
    };
    // **Les clés présentées sont dites, pas supposées.** L'exposition par clés
    // (vision §7) écartera au chargement ce que les clés ne rendent pas vrai ;
    // aucun gabarit livré n'en porte encore, donc rien n'est écarté
    // aujourd'hui — et c'est dit, parce qu'une liste d'outils vide sans
    // explication serait un défaut et non une sécurité.
    let outils = prepared.describe()["tools"].as_array().map(Vec::len).unwrap_or(0);
    eprintln!(
        "[mcp] {} — {outils} outils, clés présentées : {}",
        prepared.manifest.name,
        if cles.is_empty() { "aucune".to_string() } else { cles.join(",") }
    );
    let conn: Box<dyn rag3weaver::connection::DbConnection> = match &demon {
        Some(adresse) => {
            let serveur = rag3weaver::daemon::DaemonConnection::serveur(
                adresse.clone(),
                "rag3daemon",
                database.display().to_string(),
            );
            eprintln!("[mcp] la base est tenue par le démon sur {adresse} — ce serveur ne la possède pas");
            Box::new(
                rag3weaver::daemon::DaemonConnection::assurer(&serveur)
                    .map_err(|e| format!("le démon sur {adresse} : {e}"))?,
            )
        }
        None => Box::new(
            Rag3dbConnection::with_manifest_buffer_pool(&database, prepared.manifest.buffer_pool)
                .map_err(|e| e.to_string())?,
        ),
    };
    let mut backend = prepared.open(conn, embedder)?;
    let mut entree = io::BufReader::new(io::stdin());
    let mut sortie = io::stdout();
    match rag3weaver::mcp::servir(&backend, &cles, &mut entree, &mut sortie)? {
        rag3weaver::mcp::Suite::Fini => backend.shutdown(),
        // **Le client a été prévenu avant qu'on parte** (décision du
        // 10 octobre). Rouvrir sous le même processus demande de lâcher le
        // catalogue et de tout remonter ; on s'arrête en le disant, et le
        // client relance — ce qui est le comportement qu'un client MCP sait
        // déjà traiter, contrairement à une disparition muette.
        rag3weaver::mcp::Suite::DoitRouvrir(raison) => {
            drop(backend);
            eprintln!("[mcp] la base doit être rouverte ({raison}) — arrêt pour relance");
            std::process::exit(rag3weaver::connection::EXIT_MUST_REOPEN);
        }
    }
}

/// **Le serveur HTTP** : les routes du manifeste (`"routes"`), sur la boucle
/// locale ([`rag3weaver::serve`]).
///
/// ```text
/// rag3weaver-backend serve --manifest <backend.json> [--address 127.0.0.1:7777] [--threads 4]
/// ```
///
/// `POST /arret` ferme l'écoute ; la base est alors fermée proprement ici.
fn servir_http(args: &[String]) -> Result<(), String> {
    const USAGE: &str = "usage: rag3weaver-backend serve --manifest <backend.json> \
                         [--address 127.0.0.1:7777] [--threads 4]";
    let mut manifeste: Option<String> = None;
    let mut adresse = "127.0.0.1:7777".to_string();
    let mut fils = 4usize;
    let mut i = 0;
    while i < args.len() {
        let valeur = args
            .get(i + 1)
            .cloned()
            .ok_or_else(|| format!("{} attend une valeur\n{USAGE}", args[i]))?;
        match args[i].as_str() {
            "--manifest" => manifeste = Some(valeur),
            "--address" => adresse = valeur,
            "--threads" => {
                fils = valeur
                    .parse()
                    .map_err(|_| format!("--threads attend un nombre\n{USAGE}"))?
            }
            autre => return Err(format!("option inconnue : {autre}\n{USAGE}")),
        }
        i += 2;
    }
    let manifeste = manifeste.ok_or_else(|| format!("--manifest est obligatoire\n{USAGE}"))?;
    let prepared = PreparedBackend::load(Path::new(&manifeste))?;
    let database = prepared.path(&prepared.manifest.database);
    if let Some(parent) = database.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let embedder = if prepared.needs_embeddings() {
        Some(prepared.connect_embedder()?.0)
    } else {
        None
    };
    let conn =
        Rag3dbConnection::with_manifest_buffer_pool(&database, prepared.manifest.buffer_pool)
            .map_err(|e| e.to_string())?;
    let nom = prepared.manifest.name.clone();
    let backend = std::sync::Arc::new(prepared.open(Box::new(conn), embedder)?);
    eprintln!("[serve] {nom} — http://{adresse}/ ; POST /arret pour arrêter");
    rag3weaver::serve::serve(backend.clone(), &adresse, fils)?;
    let mut backend = std::sync::Arc::try_unwrap(backend)
        .map_err(|_| "un fil tient encore le backend après l'arrêt".to_string())?;
    backend.shutdown()
}

fn run() -> Result<(), String> {
    // La sous-commande se reconnaît au premier mot ; la forme positionnelle
    // d'avant (`rag3weaver-backend backend.json [--describe]`) reste intacte.
    let tous: Vec<String> = std::env::args().skip(1).collect();
    if tous.first().map(String::as_str) == Some("mcp") {
        return servir_mcp(&tous[1..]);
    }
    if tous.first().map(String::as_str) == Some("serve") {
        return servir_http(&tous[1..]);
    }
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: rag3weaver-backend backend.json [--describe]")?;
    let option = args.next();
    if option.as_deref().is_some_and(|v| v != "--describe") || args.next().is_some() {
        return Err("usage: rag3weaver-backend backend.json [--describe]".into());
    }
    let prepared = PreparedBackend::load(Path::new(&path))?;
    if option.as_deref() == Some("--describe") {
        println!("{}", prepared.describe());
        return Ok(());
    }
    let database = prepared.path(&prepared.manifest.database);
    if let Some(parent) = database.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // Un backend en mots seuls n'exige pas de service d'embarquement.
    let embedder = if prepared.needs_embeddings() {
        Some(prepared.connect_embedder()?.0)
    } else {
        None
    };
    let tampon = rag3weaver::rag3db_connection::buffer_pool_choice(prepared.manifest.buffer_pool);
    eprintln!("[rag3weaver] tampon du moteur : {}", rag3weaver::rag3db_connection::describe_buffer_pool(tampon));
    let conn = Rag3dbConnection::with_manifest_buffer_pool(&database, prepared.manifest.buffer_pool).map_err(|e| e.to_string())?;
    // Crochet de test (`scripts/test_backend_must_reopen.py`) : le k-ième
    // appel d'outil rencontre la base comme après un point de reprise échoué.
    let hook = conn.failed_checkpoint_hook();
    let fail_at_call: Option<usize> = std::env::var("RAG3WEAVER_TEST_FAILED_CHECKPOINT_AT_CALL")
        .ok()
        .and_then(|v| v.parse().ok());
    let mut calls = 0usize;
    let mut backend = prepared.open(Box::new(conn), embedder)?;
    for line in io::stdin().lock().lines() {
        let request: Result<Value, String> = line.map_err(|e| e.to_string())
            .and_then(|line| serde_json::from_str(&line).map_err(|e| e.to_string()));
        if request.as_ref().ok().is_some_and(|v| v["op"] == "shutdown") {
            if let Some(reason) = backend.must_reopen() {
                println!("{}", json!({"ok":false,"error":format!("la base doit être rouverte : {reason}"),"mustReopen":true}));
                io::stdout().flush().map_err(|e| e.to_string())?;
                drop(backend);
                std::process::exit(rag3weaver::connection::EXIT_MUST_REOPEN);
            }
            backend.shutdown()?;
            drop(backend); // Database checkpoint/destruction must finish before acknowledgement.
            println!("{}", json!({"ok":true,"result":{"closed":true}}));
            io::stdout().flush().map_err(|e| e.to_string())?;
            return Ok(());
        }
        let result = request.and_then(|v| -> Result<Value, String> {
            match v["op"].as_str() {
                Some("describe") => Ok(backend.prepared().describe()),
                Some("call") => {
                    calls += 1;
                    if fail_at_call == Some(calls) {
                        hook.after(0);
                    }
                    backend.call(
                        v["name"].as_str().ok_or("tool name missing")?,
                        v.get("arguments").cloned().unwrap_or(json!({})),
                    )
                }
                // Réservées à l'hôte : jamais exposées comme outils.
                Some("journal") => backend.journal(v["events"].as_array().ok_or("events missing")?),
                Some("index_state") => backend.index_states(),
                Some("journal_read") => backend.journal_read(
                    v["conversation"].as_str().ok_or("conversation missing")?,
                    v["since_ms"].as_i64().unwrap_or(0),
                ),
                _ => Err("op must be describe, call, journal, journal_read or index_state".into()),
            }
        });
        // **Un point de reprise a échoué** : la réponse le dit, puis l'hôte
        // s'arrête avec `EXIT_MUST_REOPEN` pour être relancé sur une base
        // rouverte. Pas de `shutdown` : il demanderait un point de reprise
        // que le moteur refuse.
        let must_reopen = backend.must_reopen();
        let response = match (result, &must_reopen) {
            (_, Some(reason)) => json!({"ok":false,"error":format!("la base doit être rouverte : {reason}"),"mustReopen":true}),
            (Ok(v), None) => json!({"ok":true,"result":v}),
            (Err(e), None) => json!({"ok":false,"error":e}),
        };
        println!("{response}");
        io::stdout().flush().map_err(|e| e.to_string())?;
        if let Some(reason) = must_reopen {
            eprintln!("rag3weaver-backend : la base doit être rouverte — arrêt pour relance ({reason})");
            drop(backend);
            std::process::exit(rag3weaver::connection::EXIT_MUST_REOPEN);
        }
    }
    backend.shutdown()
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
