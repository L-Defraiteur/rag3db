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
/// rag3weaver-backend mcp --manifest <backend.json> [--keys dev,admin] [--daemon <adresse> | --standalone]
/// ```
///
/// **Par défaut, un client mince de l'hôte du démon** (forme C, marches 3 et
/// 4) : une base rag3db ne s'ouvre que par un processus, et Claude Code lance
/// un processus MCP par session — l'hôte (`rag3weaver-backend daemon`) tient
/// les bases, N sessions lui parlent. Le client trouve l'hôte par
/// `daemon.json` et le lance s'il manque ; `--daemon <adresse>` force une
/// adresse. Sans hôte joignable, le serveur démarre quand même et l'outil
/// `daemon_status` dit pourquoi.
///
/// `--standalone` : la forme d'avant, le serveur **possède** sa base — une
/// session à la fois.
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
                         [--keys dev,admin] [--daemon <adresse>] \
                         [--daemon-json <chemin> | --daemon-json-command <commande>] [--standalone]";
    let mut manifeste: Option<String> = None;
    let mut cles: Vec<String> = Vec::new();
    let mut hote: Option<String> = None;
    let mut seul = false;
    let mut source: Option<rag3weaver::daemon_client::DaemonSource> = None;
    let mut i = 0;
    while i < args.len() {
        let valeur = |i: usize| -> Result<String, String> {
            args.get(i + 1).cloned().ok_or_else(|| format!("{} attend une valeur\n{USAGE}", args[i]))
        };
        match args[i].as_str() {
            "--manifest" => { manifeste = Some(valeur(i)?); i += 2; }
            "--daemon" => { hote = Some(valeur(i)?); i += 2; }
            // **La forme d'avant la forme C** : ce processus possède sa base.
            // Une session à la fois ; gardée pour les témoins et les usages à
            // une seule session, jusqu'à son retrait (page C, en dernier).
            "--standalone" => { seul = true; i += 1; }
            // **Un hôte désigné** (marche 5) : son daemon.json, en fichier ou
            // par une commande ; le jeton n'est jamais un argument.
            "--daemon-json" => {
                source = Some(rag3weaver::daemon_client::DaemonSource::File(std::path::PathBuf::from(valeur(i)?)));
                i += 2;
            }
            "--daemon-json-command" => {
                source = Some(rag3weaver::daemon_client::DaemonSource::Command(valeur(i)?));
                i += 2;
            }
            // **L'ancien sens, refusé en le disant** (page C, §3) : `--demon`
            // ne déportait que la connexion, et N sessions y faisaient N
            // réacteurs. Pas un synonyme silencieux de `--daemon`, qui veut
            // dire l'inverse : tout passe à l'hôte.
            "--demon" => {
                return Err(format!(
                    "--demon est retiré : il ne déportait que la connexion à la base, et chaque \
                     session gardait son catalogue et son réacteur. Sous la forme C, le serveur \
                     MCP est un client mince de l'hôte : --daemon <adresse> (l'hôte se lance par \
                     `rag3weaver-backend daemon`)\n{USAGE}"
                ))
            }
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
    if seul && source.is_some() {
        return Err(format!("--standalone et --daemon-json s'excluent\n{USAGE}"));
    }
    if seul && hote.is_some() {
        return Err(format!("--standalone et --daemon s'excluent : l'un possède la base, l'autre la laisse à l'hôte\n{USAGE}"));
    }
    // **Le client mince, par défaut** (marches 3 et 4 de la forme C) : rien ne
    // s'ouvre ici, ni base ni modèle ; tout passe à l'hôte, et ce processus ne
    // garde que le protocole MCP. L'hôte se trouve par `daemon.json` et se
    // lance s'il manque ; `--daemon <adresse>` force une adresse. S'il ne peut
    // pas être joint, le serveur démarre quand même et le dit par l'outil
    // `daemon_status`.
    if !seul {
        use rag3weaver::daemon_client::{Attach, McpClient};
        let attach = match (&hote, source) {
            (adresse, Some(source)) => Attach::Designated(rag3weaver::daemon_client::Designated {
                source,
                address: adresse.clone(),
            }),
            (Some(adresse), None) => Attach::Forced(adresse.clone()),
            (None, None) => Attach::Discover(std::env::current_exe().map_err(|e| format!("l'exécutable courant : {e}"))?),
        };
        let client = McpClient::start(attach, Path::new(&manifeste), &format!("mcp (pid {})", std::process::id()), cles.clone());
        eprintln!(
            "[mcp] {} — {} outils, clés présentées : {} — client mince de l'hôte{}",
            rag3weaver::mcp::Hote::nom(&client),
            rag3weaver::mcp::Hote::outils(&client).len(),
            if cles.is_empty() { "aucune".to_string() } else { cles.join(",") },
            match client.session() {
                Some(s) => format!(", session {s}"),
                None => " — absent pour l'instant".to_string(),
            }
        );
        let reactions = client.reactions();
        if !reactions.is_empty() {
            eprintln!("[mcp] {} réaction(s) montée(s) par l'hôte : {}", reactions.len(), reactions.join(", "));
        }
        let mut entree = EntreeInterruptible::sur_stdin();
        let mut sortie = io::stdout();
        let suite = rag3weaver::mcp::servir(&client, &cles, &mut entree, &mut sortie);
        client.close();
        // `doit_rouvrir` n'existe pas pour un client mince : c'est l'hôte qui
        // rouvre, et l'appel concerné l'a dit (erreur transitoire).
        return suite.map(|_| ());
    }
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
    let conn: Box<dyn rag3weaver::connection::DbConnection> = Box::new(
        Rag3dbConnection::with_manifest_buffer_pool(&database, prepared.manifest.buffer_pool)
            .map_err(|e| e.to_string())?,
    );
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
    // L'adresse par défaut : `RAG3WEAVER_DAEMON_ADDRESS`, sinon 127.0.0.1:7970
    // (c'est ainsi qu'un client lance l'hôte) ; `--address` la donne à la main.
    let mut adresse = rag3weaver::daemon_lifecycle::launch_address();
    let mut explicite = false;
    let mut fils = 8usize;
    let mut i = 0;
    while i < args.len() {
        let valeur = args.get(i + 1).cloned().ok_or_else(|| format!("{} attend une valeur\n{USAGE}", args[i]))?;
        match args[i].as_str() {
            "--address" => {
                adresse = valeur;
                explicite = true;
            }
            "--threads" => fils = valeur.parse().map_err(|_| format!("--threads attend un nombre\n{USAGE}"))?,
            autre => return Err(format!("option inconnue : {autre}\n{USAGE}")),
        }
        i += 2;
    }
    if !rag3weaver::daemon::est_local(&adresse) {
        return Err(format!("le démon n'écoute que sur la boucle locale : {adresse}"));
    }
    use rag3weaver::daemon_lifecycle as cycle;
    // **Un démon** : un hôte vivant qui répond déjà refuse ce démarrage, en
    // le nommant — deux hôtes pourraient ouvrir les mêmes bases. Seul un
    // `--address` explicite (un hôte à la main, pour un témoin) passe outre,
    // en le disant, et sans toucher au fichier de l'autre.
    let autre = cycle::read().filter(|f| f.pid != std::process::id() && cycle::answers(f));
    if let (Some(autre), false) = (&autre, explicite) {
        return Err(format!(
            "un hôte répond déjà à {} (pid {}, {}) : un seul démon. L'arrêter : kill -TERM {} ; ou \
             --address pour un second hôte à la main, qui n'écrira pas {}",
            autre.address,
            autre.pid,
            autre.build,
            autre.pid,
            cycle::file_path().display()
        ));
    }
    // **Le port d'abord, `daemon.json` ensuite** (marche 4, page C §5) : le
    // fichier dit l'adresse réellement servie, un port 0 résolu.
    let ecoute = std::net::TcpListener::bind(&adresse).map_err(|e| format!("{adresse} : {e}"))?;
    let servie = ecoute.local_addr().map_err(|e| e.to_string())?.to_string();
    let server = tiny_http::Server::from_listener(ecoute, None).map_err(|e| format!("{servie} : {e}"))?;
    let jeton = cycle::new_token()?;
    let pid = std::process::id();
    let fiche = cycle::DaemonFile {
        address: servie.clone(),
        pid,
        token: jeton.clone(),
        build: cycle::build_id(),
        started_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    };
    if let Some(autre) = &autre {
        eprintln!(
            "[démon] un autre hôte répond déjà ({}, pid {}) : hôte à la main par --address, {} n'est \
             pas réécrit, les clients continueront d'aller à lui",
            autre.address,
            autre.pid,
            cycle::file_path().display()
        );
    } else {
        cycle::write(&fiche)?;
    }
    eprintln!(
        "[démon] {} — http://{servie}/ ; POST /session, puis /open, /describe, /call ; POST /arret pour \
         arrêter ; {} (jeton local exigé)",
        rag3weaver::daemon_host::SERVICE,
        cycle::file_path().display()
    );
    // **SIGTERM vaut `POST /arret`** : la même poignée d'arrêt, les bases
    // fermées proprement, puis `daemon.json` retiré.
    let serveur = rag3weaver::daemon_host::DaemonServer::new(server, fils);
    let arret = serveur.stopper();
    std::thread::spawn(move || {
        let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() else { return };
        rt.block_on(async {
            use tokio::signal::unix::{signal, SignalKind};
            let Ok(mut terme) = signal(SignalKind::terminate()) else { return };
            if terme.recv().await.is_some() {
                eprintln!("[démon] SIGTERM : arrêt propre, comme POST /arret");
                arret.stop();
            }
        });
    });
    serveur.serve(std::sync::Arc::new(rag3weaver::daemon_host::DaemonHost::new()), Some(jeton));
    cycle::remove_if_mine(pid);
    eprintln!("[démon] arrêté ; {} retiré", cycle::file_path().display());
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

/// **`code-tree`** : poser ou retirer un arbre de code indexé (voir
/// `rag3weaver::code_tree`).
fn code_tree(args: &[String]) -> Result<(), String> {
    const USAGE: &str = "usage: rag3weaver-backend code-tree add <arbre> [--from code-lourd] [--as <nom>] [--daemon <adresse>] [--sans-hote]\n       \
                         rag3weaver-backend code-tree remove <nom | arbre> [--daemon <adresse>] [--sans-hote]";
    use rag3weaver::code_tree::{add_files, add_with_host, name_of_tree, remove_files, unregister, FromAroundCopy, Roots};
    let (action, reste) = args.split_first().ok_or(USAGE)?;
    let mut cible = None;
    let mut from = "code-lourd".to_string();
    let mut nom = None;
    let mut hote = None;
    let mut sans_hote = false;
    let mut i = 0;
    while i < reste.len() {
        let valeur = |i: usize| reste.get(i + 1).cloned().ok_or_else(|| format!("{} demande une valeur\n{USAGE}", reste[i]));
        match reste[i].as_str() {
            "--from" => { from = valeur(i)?; i += 2; }
            "--as" => { nom = Some(valeur(i)?); i += 2; }
            "--daemon" => { hote = Some(valeur(i)?); i += 2; }
            "--sans-hote" => { sans_hote = true; i += 1; }
            x if cible.is_none() && !x.starts_with("--") => { cible = Some(x.to_string()); i += 1; }
            x => return Err(format!("argument inconnu « {x} »\n{USAGE}")),
        }
    }
    let cible = cible.ok_or(USAGE)?;
    let roots = Roots::of_user()?;
    let adresse = || hote.clone().unwrap_or_else(|| rag3weaver::daemon_lifecycle::read().map(|f| f.address).unwrap_or_else(|| rag3weaver::daemon_lifecycle::DEFAULT_ADDRESS.to_string()));
    match action.as_str() {
        "add" => {
            let (a, autour) = if sans_hote {
                (add_files(&roots, Path::new(&cible), &from, nom.as_deref())?, FromAroundCopy::NoHost)
            } else {
                add_with_host(&roots, Path::new(&cible), &from, nom.as_deref(), &adresse())?
            };
            println!("{} : manifeste {}", a.name, a.manifest.display());
            println!("base {} — {} fichier(s) de {from}, {}", a.database.display(), a.copied.len(), if a.reflink { "par reflink" } else { "copie pleine (pas de reflink ici)" });
            match autour {
                FromAroundCopy::NoHost => println!("aucun hôte ne répond : la base de {from} n'est ouverte par personne, copiée telle quelle"),
                FromAroundCopy::NotOpen => println!("{from} n'était pas ouvert dans l'hôte ; {} enregistré à l'hôte {}", a.name, adresse()),
                FromAroundCopy::ClosedAndReopened => println!("{from} fermé le temps de la copie puis rouvert (quelques secondes) ; {} enregistré à l'hôte {}", a.name, adresse()),
            }
            Ok(())
        }
        "remove" => {
            // Un nom de backend, ou le chemin de l'arbre.
            let nom = if roots.manifest(&cible).exists() { cible.clone() } else { name_of_tree(Path::new(&cible))? };
            // Fermer d'abord (le manifeste doit exister pour être canonisé),
            // déplacer ensuite.
            if !sans_hote {
                unregister(&adresse(), &roots.manifest(&nom))?;
                println!("{nom} : fermé par l'hôte {}", adresse());
            }
            let r = remove_files(&roots, &nom)?;
            println!("{nom} : manifeste et base mis de côté sous {}", r.aside.display());
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
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
    if tous.first().map(String::as_str) == Some("code-tree") {
        return code_tree(&tous[1..]);
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
