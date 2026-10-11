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
/// **L'entrée du serveur MCP, qui finit sur EOF ou sur SIGTERM** (11 octobre
/// 2026, marche à part de la page C, §4).
///
/// Le cycle de vie MCP sur stdio prévoit que le client ferme l'entrée, attend la
/// sortie, **puis envoie SIGTERM** s'il juge l'attente trop longue, puis SIGKILL.
/// Depuis l'arrêt propre des indexations (marche 0), fermer le tube les arrête
/// au prochain paquet validé et les attend : quelques secondes, davantage sur
/// un paquet de blobs. Un SIGTERM par défaut tuait le serveur au milieu, et
/// laissait la session de synchronisation ouverte.
///
/// Ici, SIGTERM **vaut la fermeture du tube** : la boucle MCP voit une fin
/// d'entrée ordinaire, finit l'appel en cours s'il y en a un, et l'arrêt reste
/// celui de toujours — réactions, indexations au prochain paquet, catalogue,
/// puis la base détruite proprement à la sortie de la portée. `mcp.rs` n'en sait
/// rien. Un second SIGTERM ne fait plus rien : seul SIGKILL laisse alors la
/// session en plan, et c'est le contrat.
///
/// La lecture de stdin vit dans son propre fil, parce qu'une lecture bloquée ne
/// s'interrompt pas proprement ; les lignes passent par un canal où le fil du
/// signal peut glisser la fin.
struct EntreeInterruptible {
    lignes: std::sync::mpsc::Receiver<Option<Vec<u8>>>,
    tampon: Vec<u8>,
    position: usize,
    finie: bool,
}

impl EntreeInterruptible {
    fn sur_stdin() -> Self {
        let (envoi, lignes) = std::sync::mpsc::channel::<Option<Vec<u8>>>();
        let lecteur = envoi.clone();
        std::thread::spawn(move || {
            let mut stdin = io::stdin().lock();
            loop {
                let mut ligne = Vec::new();
                match stdin.read_until(b'\n', &mut ligne) {
                    Ok(0) | Err(_) => {
                        let _ = lecteur.send(None);
                        return;
                    }
                    Ok(_) => {
                        if lecteur.send(Some(ligne)).is_err() {
                            return;
                        }
                    }
                }
            }
        });
        #[cfg(unix)]
        std::thread::spawn(move || {
            let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_io().build() else {
                eprintln!("[mcp] SIGTERM ne sera pas intercepté : runtime du signal indisponible");
                return;
            };
            rt.block_on(async {
                use tokio::signal::unix::{signal, SignalKind};
                let Ok(mut terme) = signal(SignalKind::terminate()) else {
                    eprintln!("[mcp] SIGTERM ne sera pas intercepté : gestionnaire refusé");
                    return;
                };
                if terme.recv().await.is_some() {
                    eprintln!(
                        "[mcp] SIGTERM : arrêt propre, comme une fermeture du tube — l'appel en \
                         cours finit, les indexations s'arrêtent au prochain paquet"
                    );
                    let _ = envoi.send(None);
                }
                // Le gestionnaire reste en place : un second SIGTERM ne tue
                // pas un arrêt propre en cours ; SIGKILL, si.
                std::future::pending::<()>().await;
            });
        });
        Self { lignes, tampon: Vec::new(), position: 0, finie: false }
    }
}

impl io::Read for EntreeInterruptible {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let disponible = io::BufRead::fill_buf(self)?;
        let n = disponible.len().min(buf.len());
        buf[..n].copy_from_slice(&disponible[..n]);
        io::BufRead::consume(self, n);
        Ok(n)
    }
}

impl io::BufRead for EntreeInterruptible {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.position >= self.tampon.len() && !self.finie {
            match self.lignes.recv() {
                Ok(Some(ligne)) => {
                    self.tampon = ligne;
                    self.position = 0;
                }
                Ok(None) | Err(_) => self.finie = true,
            }
        }
        if self.finie && self.position >= self.tampon.len() {
            return Ok(&[]);
        }
        Ok(&self.tampon[self.position..])
    }

    fn consume(&mut self, n: usize) {
        self.position = (self.position + n).min(self.tampon.len());
    }
}

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
    // **Les réactions montées sont dites, elles aussi.** `Backend::open` les
    // monte depuis le manifeste ; sans cette ligne, rien en service ne dit
    // qu'un graphe tourne derrière les outils, et un opérateur ne peut pas
    // distinguer « la réaction est montée » de « la section a été ignorée ».
    let reactions = backend.prepared().reaction_graphs().keys().cloned().collect::<Vec<_>>();
    if reactions.is_empty() {
        eprintln!("[mcp] aucune réaction déclarée");
    } else {
        eprintln!(
            "[mcp] {} réaction(s) montée(s) : {}",
            reactions.len(),
            reactions.join(", ")
        );
    }
    let mut entree = EntreeInterruptible::sur_stdin();
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
/// rag3weaver-backend serve --manifest <backend.json> [--address 127.0.0.1:7777] [--threads 4] [--atelier]
/// ```
///
/// `--atelier` ajoute la page vivante sous `/_atelier` ([`rag3weaver::atelier`]) :
/// son chat prend le modèle déclaré par `models.llm` (binaire bâti avec la
/// feature `openai-llm`) ; sans modèle, la page dit pourquoi le chat est éteint.
///
/// `POST /arret` ferme l'écoute ; la base est alors fermée proprement ici.
/// Le modèle du chat de l'atelier : `models.llm` du manifeste, ou pourquoi
/// il n'y en a pas.
fn modele_du_chat(
    prepared: &PreparedBackend,
) -> Result<std::sync::Arc<dyn rag3weaver::llm::Llm>, String> {
    let Some(source) = prepared
        .manifest
        .models
        .get(&rag3weaver::model_source::Capability::Llm)
    else {
        return Err("le manifeste ne déclare pas de modèle de langage (`models.llm`, \
                    provider « compatible ») — déclarez-en un pour allumer le chat"
            .into());
    };
    #[cfg(feature = "openai-llm")]
    {
        rag3weaver::model_source::connect_llm(source)
            .map(|(llm, _)| std::sync::Arc::new(llm) as std::sync::Arc<dyn rag3weaver::llm::Llm>)
    }
    #[cfg(not(feature = "openai-llm"))]
    {
        let _ = source;
        Err("ce binaire est bâti sans la feature `openai-llm` : rebâtir avec elle pour \
             allumer le chat"
            .into())
    }
}

/// **Le démon de la forme C** (marche 1) : plusieurs backends, des sessions,
/// des clients minces sur la boucle locale ([`rag3weaver::daemon_host`]).
///
/// ```text
/// rag3weaver-backend daemon [--address 127.0.0.1:7970] [--threads 8]
/// ```
///
/// `POST /arret` ferme l'écoute, puis chaque backend proprement.
fn servir_le_demon(args: &[String]) -> Result<(), String> {
    const USAGE: &str = "usage: rag3weaver-backend daemon [--address 127.0.0.1:7970] [--threads 8]";
    let mut adresse = "127.0.0.1:7970".to_string();
    let mut fils = 8usize;
    let mut i = 0;
    while i < args.len() {
        let valeur = args.get(i + 1).cloned().ok_or_else(|| format!("{} attend une valeur\n{USAGE}", args[i]))?;
        match args[i].as_str() {
            "--address" => adresse = valeur,
            "--threads" => fils = valeur.parse().map_err(|_| format!("--threads attend un nombre\n{USAGE}"))?,
            autre => return Err(format!("option inconnue : {autre}\n{USAGE}")),
        }
        i += 2;
    }
    if !rag3weaver::daemon::est_local(&adresse) {
        return Err(format!("le démon n'écoute que sur la boucle locale : {adresse}"));
    }
    let server = tiny_http::Server::http(&adresse).map_err(|e| format!("{adresse} : {e}"))?;
    eprintln!("[démon] {} — http://{adresse}/ ; POST /session, puis /open, /describe, /call ; POST /arret pour arrêter", rag3weaver::daemon_host::SERVICE);
    rag3weaver::daemon_host::serve_daemon(std::sync::Arc::new(rag3weaver::daemon_host::DaemonHost::new()), server, fils);
    Ok(())
}

fn servir_http(args: &[String]) -> Result<(), String> {
    const USAGE: &str = "usage: rag3weaver-backend serve --manifest <backend.json> \
                         [--address 127.0.0.1:7777] [--threads 4] [--atelier]";
    let mut manifeste: Option<String> = None;
    let mut adresse = "127.0.0.1:7777".to_string();
    let mut fils = 4usize;
    let mut atelier = false;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--atelier" {
            atelier = true;
            i += 1;
            continue;
        }
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
    let modele = modele_du_chat(&prepared);
    let backend = std::sync::Arc::new(prepared.open(Box::new(conn), embedder)?);
    let atelier = if atelier {
        let page = rag3weaver::atelier::Atelier::new(backend.clone(), modele)?;
        let montre = page.watch(std::time::Duration::from_millis(500));
        eprintln!("[serve] atelier : http://{adresse}/_atelier");
        Some((page, montre))
    } else {
        None
    };
    eprintln!("[serve] {nom} — http://{adresse}/ ; POST /arret pour arrêter");
    rag3weaver::serve::serve(
        backend.clone(),
        &adresse,
        fils,
        atelier.as_ref().map(|(page, _)| page.clone()),
    )?;
    if let Some((page, montre)) = atelier {
        page.stop();
        let _ = montre.join();
    }
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
    if tous.first().map(String::as_str) == Some("daemon") {
        return servir_le_demon(&tous[1..]);
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
