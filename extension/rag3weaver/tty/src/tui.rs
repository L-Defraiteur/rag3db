//! **La boucle du terminal** (lot tty-2) : la vue inline sur crossterm (Windows,
//! macOS, Linux). Trois sources entrent dans un même canal : le flux de l'hôte
//! (un fil), la boîte relue toutes les 2 s tant que le flux ne la relaie pas
//! (un fil), et le clavier. L'écran se redessine au plus 30 fois par seconde,
//! et seulement quand quelque chose a changé : un token qui arrive entre deux
//! images attend la suivante, les tokens d'une même image partent ensemble.

use crate::anim::Capacites;
use crate::client::{self, Depot};
use crate::endpoint::Endpoint;
use crate::flux;
use crate::journal::Journal;
use crate::vue::{Action, Touche, Validee, Vue, HAUTEUR};
use ratatui::crossterm::event::{self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::{TerminalOptions, Viewport};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

/// L'intervalle minimal entre deux images (30 par seconde).
pub const IMAGE: Duration = Duration::from_millis(33);

enum Msg {
    Trame(Value),
    Boite(Value),
    Note(String),
    Politiques(Value),
    Entete(String),
    /// L'état de l'équipier selon `GET /ide/teammate/list`.
    EtatHote(Option<String>),
}

/// La tâche suivie : son dépôt, et son nom.
#[derive(Clone)]
pub struct Tache {
    pub depot: Depot,
    pub task: String,
}

/// **Ce qu'on apprend d'une tâche** : le manifeste de son worktree (le filtre du
/// flux : une trame d'un autre worktree ne la regarde pas), son en-tête, et ce
/// que chacun de ses outils a le droit de faire.
fn connaitre(agent: &ureq::Agent, ep: &Endpoint, tache: &Tache, filtre: &Mutex<Option<String>>, tx: &mpsc::Sender<Msg>) {
    let chemin = format!("/ide/teammate?backend={}&task={}", client::encoder(&tache.depot.id), client::encoder(&tache.task));
    let worktree = client::lire(agent, ep, &chemin).ok().and_then(|v| v["backend"].as_str().map(str::to_string));
    if let Some(m) = &worktree {
        *filtre.lock().unwrap_or_else(|p| p.into_inner()) = Some(m.clone());
    }
    let manifeste = worktree.unwrap_or_else(|| tache.depot.manifest.clone());
    if let Ok(etat) = load_state("/ide/etat") {
        let _ = tx.send(Msg::Entete(client::entete(&etat, &manifeste, &tache.task)));
    }
    if let Ok(d) = client::poster_brut(agent, ep, "/ide/describe", &json!({ "manifest": manifeste })) {
        let _ = tx.send(Msg::Politiques(d));
    }
}

/// Comment la vue a fini (tty-13). Une seule fin par séance : la taille du
/// thème dans `Changer` ne pèse sur rien.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Fin {
    Quitter,
    /// `/tache` ou `/depot` : la vue repart sur une autre tâche ou un autre
    /// dépôt, avec le journal et le thème de la séance.
    Changer { depot: Option<String>, tache: Option<String>, journal: Option<Journal>, theme: crate::theme::Theme },
}

/// Pourquoi la vue s'est arrêtée.
#[derive(Debug)]
pub enum Echec {
    /// La vue inline n'a pas pu démarrer (le terminal ne rend pas la position du
    /// curseur, par exemple) : rien n'est encore parti à l'hôte, `--plain` peut
    /// prendre la suite ; le journal lui est rendu.
    VueIndisponible(String, Option<Journal>),
    /// Une erreur pendant la séance.
    Autre(String),
}

/// `journal` reçoit en texte brut ce qui part dans l'historique ; `mosh` dit que
/// cet historique ne se relira pas, et la zone vivante donne alors la commande
/// qui relit le journal (lot tty-7).
pub fn lancer(ep: Endpoint, tache: Tache, demande: Option<String>, depuis: Option<u64>, theme: crate::theme::Theme, mut journal: Option<Journal>, mosh: bool) -> Result<Fin, Echec> {
    // La vue d'abord : si elle ne démarre pas, rien n'est parti à l'hôte (ni la
    // demande, ni un abonnement), et `--plain` prend la suite sans doublon.
    let mut terminal = match ratatui::try_init_with_options(TerminalOptions { viewport: Viewport::Inline(HAUTEUR) }) {
        Ok(t) => t,
        Err(e) => {
            let _ = ratatui::try_restore();
            return Err(Echec::VueIndisponible(e.to_string(), journal));
        }
    };
    // Le collage d'un bloc (bracketed paste) : un texte collé arrive entier, ses
    // sauts de ligne ne valent pas Entrée. Un terminal qui ne le connaît pas
    // ignore la demande.
    let _ = ratatui::crossterm::execute!(std::io::stdout(), EnableBracketedPaste);
    // Maj-Entrée (tty-10) : le protocole clavier de kitty, demandé seulement à
    // un terminal qui l'annonce. Ailleurs, rien n'est demandé, et Alt-Entrée
    // reste le saut de ligne.
    let maj_entree = matches!(ratatui::crossterm::terminal::supports_keyboard_enhancement(), Ok(true));
    if maj_entree {
        let _ = ratatui::crossterm::execute!(std::io::stdout(), PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES));
    }
    let agent = client::agent();
    let (tx, rx) = mpsc::channel::<Msg>();
    let arret = Arc::new(AtomicBool::new(false));
    let boite_au_flux = Arc::new(AtomicBool::new(false));
    let filtre: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    connaitre(&agent, &ep, &tache, &filtre, &tx);

    // Le flux, sur son fil ; la demande part une fois l'abonnement pris.
    let flux_fil = {
        let (ep, tx, arret, boite_au_flux, agent, filtre) = (ep.clone(), tx.clone(), arret.clone(), boite_au_flux.clone(), agent.clone(), filtre.clone());
        let tache = tache.clone();
        let mut demande = demande.clone();
        std::thread::spawn(move || {
            let tx2 = tx.clone();
            let mut ouvert = || {
                if let Some(texte) = demande.take() {
                    match client::poster(&agent, &ep, "/ide/teammate/mailbox", &json!({ "backend": tache.depot.id, "task": tache.task, "kind": "request", "text": texte })) {
                        // La demande ouvre la tâche : son worktree existe désormais.
                        Ok(_) => connaitre(&agent, &ep, &tache, &filtre, &tx2),
                        Err(e) => {
                            let _ = tx2.send(Msg::Note(format!("la demande est refusée : {e}")));
                        }
                    }
                }
            };
            let tx3 = tx.clone();
            let principal = tache.depot.id.clone();
            let r = flux::suivre_sans_fin(&ep, &["agent", "tokens", "mailbox", "declarations"], depuis, &arret, &mut ouvert, |p| {
                let _ = tx3.send(Msg::Note(format!("flux coupé ({p}) : je reprends")));
            }, |t| {
                if t.event == "lacune" {
                    let _ = tx.send(Msg::Note("lacune : le tampon de l'hôte a perdu des événements".into()));
                } else if let Ok(v) = serde_json::from_str::<Value>(&t.data) {
                    // Une proposition de déclarations du dépôt principal passe aussi (tty-16) : c'est
                    // sur lui que les déclarations attendent un accord.
                    let du_principal = v["evenement"]["kind"] == "PropositionChanged" && v["backend"].as_str() == Some(principal.as_str());
                    if let Some(f) = filtre.lock().unwrap_or_else(|p| p.into_inner()).as_deref() {
                        if v["backend"].as_str() != Some(f) && !du_principal {
                            return flux::Suite::Continuer;
                        }
                    }
                    if v["evenement"]["kind"] == "MailboxLine" {
                        boite_au_flux.store(true, Ordering::SeqCst);
                    }
                    let _ = tx.send(Msg::Trame(v));
                }
                flux::Suite::Continuer
            });
            if let Err(e) = r {
                let _ = tx.send(Msg::Note(format!("le flux est refusé : {e}")));
            }
        })
    };
    // La boîte, relue tant que le flux ne la relaie pas.
    {
        let (ep, tx, arret, boite_au_flux, agent) = (ep.clone(), tx.clone(), arret.clone(), boite_au_flux.clone(), agent.clone());
        let chemin = format!("/ide/teammate?backend={}&task={}", client::encoder(&tache.depot.id), client::encoder(&tache.task));
        let (parent, task) = (tache.depot.id.clone(), tache.task.clone());
        std::thread::spawn(move || {
            let mut lues = 0usize;
            while !arret.load(Ordering::SeqCst) {
                // L'état de l'équipier (lot tty-4), par la liste de son superviseur.
                if let Ok(liste) = client::lire(&agent, &ep, "/ide/teammate/list?supervisor=supervisor") {
                    if tx.send(Msg::EtatHote(crate::boite::etat_dans_la_liste(&liste, &parent, &task))).is_err() {
                        return;
                    }
                }
                if let Ok(v) = client::lire(&agent, &ep, &chemin) {
                    let lignes = v["mailbox"].as_array().cloned().unwrap_or_default();
                    for l in lignes.iter().skip(lues) {
                        if !boite_au_flux.load(Ordering::SeqCst) && tx.send(Msg::Boite(l.clone())).is_err() {
                            return;
                        }
                    }
                    lues = lignes.len();
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        });
    }

    let largeur = terminal.size().map(|s| s.width).unwrap_or(80);
    let mut vue = Vue::new(largeur, demande.is_some());
    vue.capacites = Capacites::du_terminal();
    vue.theme = theme;
    vue.maj_entree = maj_entree;
    vue.montrer_etapes = std::env::var("RAG3WEAVER_TTY_ETAPES").is_ok_and(|v| !v.is_empty() && v != "0");
    // L'historique des envois d'une séance à l'autre (tty-13).
    if let Some(h) = crate::historique::chemin() {
        vue.reprendre_historique(crate::historique::charger(&h));
    }
    // Sous tmux : Maj-Entrée s'ouvre si tmux permet les touches étendues en CSI u (le mode 1 de
    // modifyOtherKeys, demandé ici et rendu à la sortie) ; sinon, l'indication dit le réglage.
    let tmux_ouvert = std::env::var_os("TMUX").is_some_and(|v| !v.is_empty()) && clavier_sous_tmux(&tmux) == ClavierTmux::Ouvert;
    vue.sous_tmux = std::env::var_os("TMUX").is_some_and(|v| !v.is_empty()) && !tmux_ouvert;
    if tmux_ouvert {
        use std::io::Write;
        let _ = write!(std::io::stdout(), "\x1b[>4;1m");
        let _ = std::io::stdout().flush();
        vue.maj_entree = true;
    }
    if let (true, Some(j)) = (mosh, &journal) {
        vue.repli = Some(crate::vue::Repli { phrase: "sous mosh, l'historique ne se relit pas au défilement".into(), commande: j.relecture() });
    }
    vue.graine = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1) | 1;
    let rendu = boucle(&mut terminal, &mut vue, &rx, &agent, &ep, &tache, &mut journal);
    let rendu = rendu.map(|f| match f {
        Fin::Changer { depot, tache, .. } => Fin::Changer { depot, tache, journal: journal.take(), theme: vue.theme.clone() },
        autre => autre,
    });
    arret.store(true, Ordering::SeqCst);
    // La zone vivante reste à l'écran telle quelle ; le curseur passe dessous.
    let _ = terminal.draw(|f| vue.dessiner(f, f.area()));
    if maj_entree {
        let _ = ratatui::crossterm::execute!(std::io::stdout(), PopKeyboardEnhancementFlags);
    }
    if tmux_ouvert {
        use std::io::Write;
        let _ = write!(std::io::stdout(), "\x1b[>4;0m");
    }
    let _ = ratatui::crossterm::execute!(std::io::stdout(), DisableBracketedPaste);
    ratatui::restore();
    println!();
    let _ = flux_fil.join();
    rendu.map_err(Echec::Autre)
}

fn boucle(terminal: &mut ratatui::DefaultTerminal, vue: &mut Vue, rx: &mpsc::Receiver<Msg>, agent: &ureq::Agent, ep: &Endpoint, tache: &Tache, journal: &mut Option<Journal>) -> Result<Fin, String> {
    let mut sale = true;
    let mut derniere = Instant::now() - IMAGE;
    let mut image_vue = u64::MAX;
    loop {
        // Tout ce qui est arrivé, d'un coup.
        while let Ok(m) = rx.try_recv() {
            match m {
                Msg::Trame(v) => vue.trame(&v),
                Msg::Boite(l) => vue.ligne_de_boite(&l),
                Msg::Note(n) => vue.note(&n),
                Msg::Politiques(d) => vue.politiques(&d),
                Msg::Entete(e) => vue.entete = e,
                Msg::EtatHote(e) => vue.etat_de_l_hote(e),
            }
            sale = true;
        }
        // Une animation change d'image à sa cadence (10 images/s au plus vite).
        if vue.quelle().is_some() {
            let n = vue.image(Instant::now());
            if n != image_vue {
                image_vue = n;
                sale = true;
            }
        }
        if sale && derniere.elapsed() >= IMAGE {
            let colorer = |texte: &str, langue: &str| if langue.is_empty() { None } else { client::surligner(agent, ep, texte, langue) };
            // La zone vivante à la hauteur que veut la saisie (tty-10).
            let largeur = terminal.size().map(|s| s.width).unwrap_or(80);
            let voulue = vue.hauteur_voulue(largeur);
            if voulue != terminal.get_frame().area().height {
                retailler(terminal, voulue)?;
            }
            dessiner(terminal, vue, &colorer, journal.as_mut())?;
            sale = false;
            derniere = Instant::now();
        }
        let mut attente = if sale { IMAGE.saturating_sub(derniere.elapsed()) } else { IMAGE };
        if let Some(p) = vue.prochaine_image(Instant::now()) {
            attente = attente.min(p);
        }
        if event::poll(attente).map_err(|e| e.to_string())? {
            let action = match event::read().map_err(|e| e.to_string())? {
                Event::Key(k) if k.kind == KeyEventKind::Press => {
                    sale = true;
                    touche_de(k).and_then(|t| vue.touche(t))
                }
                // Un collage d'un bloc : ses sauts de ligne restent dans la saisie.
                Event::Paste(texte) => {
                    sale = true;
                    vue.touche(Touche::Coller(texte))
                }
                Event::Resize(l, _) => {
                    vue.redimensionner(l);
                    sale = true;
                    None
                }
                _ => None,
            };
            match action {
                Some(Action::Quitter) => return Ok(Fin::Quitter),
                Some(Action::Proposition { backend, id, appliquer }) => {
                    let (route, corps) = if appliquer { ("/ide/propositions/appliquer", json!({ "backend": backend, "id": id })) } else { ("/ide/propositions/rejeter", json!({ "backend": backend, "id": id, "why": "rejetée depuis le terminal" })) };
                    match client::poster(agent, ep, route, &corps) {
                        Ok(_) => vue.note(&format!("proposition ⟨{id}⟩ {}", if appliquer { "appliquée" } else { "rejetée" })),
                        Err(e) => vue.note(&format!("proposition ⟨{id}⟩ : {e}")),
                    }
                }
                Some(Action::Commande(c)) => {
                    if let Some(fin) = executer(c, terminal, vue, agent, ep, tache, journal)? {
                        return Ok(fin);
                    }
                    sale = true;
                }
                Some(Action::Completer(debut)) => {
                    let sources = sources_de(&debut, agent, ep, tache, &|path| client::lire_brut(agent, ep, path));
                    match crate::commandes::completer(&debut, &sources) {
                        Some(c) => vue.appliquer_completion(c),
                        None => vue.propositions = Some("rien à compléter".into()),
                    }
                    sale = true;
                }
                Some(Action::Ouvrir) => {
                    let colorer = |texte: &str, langue: &str| if langue.is_empty() { None } else { client::surligner(agent, ep, texte, langue) };
                    visionner(vue, &colorer)?;
                    sale = true;
                }
                Some(Action::Envoyer(texte, kind)) => {
                    garder(&texte);
                    if let Err(e) = client::poster(agent, ep, "/ide/teammate/mailbox", &json!({ "backend": tache.depot.id, "task": tache.task, "kind": kind, "text": texte })) {
                        vue.note(&format!("refusé : {e}"));
                    }
                }
                Some(Action::Repondre { attente, texte, accord }) => {
                    // Un o ou un n d'accord ne s'écrit pas dans l'historique.
                    if accord.is_none() {
                        garder(&texte);
                    }
                    if let Err(e) = client::poster(agent, ep, "/ide/teammate/mailbox", &crate::boite::reponse(&tache.depot.id, &tache.task, &attente, &texte, accord)) {
                        vue.note(&format!("la réponse est refusée : {e}"));
                    }
                }
                Some(Action::Arreter) => {
                    if let Err(e) = client::poster(agent, ep, "/ide/teammate/mailbox", &json!({ "backend": tache.depot.id, "task": tache.task, "kind": "stop", "text": "" })) {
                        vue.note(&format!("l'arrêt est refusé : {e}"));
                    } else {
                        vue.note("arrêt demandé");
                    }
                }
                None => {}
            }
        }
    }
}

/// L'identifiant du backend de la tâche (son worktree), d'après `/ide/etat`.
fn backend_de_tache(agent: &ureq::Agent, ep: &Endpoint, tache: &Tache) -> Option<String> {
    let etat = client::lire_brut(agent, ep, "/ide/etat").ok()?;
    let backends = etat["backends"].as_array().cloned().unwrap_or_default();
    backends.iter().find(|b| b["tache"]["nom"].as_str() == Some(tache.task.as_str()) && b["tache"]["parent"].as_str() == Some(tache.depot.id.as_str())).and_then(|b| b["id"].as_str().map(str::to_string))
}

/// Le dossier où tourne une commande `!` : le worktree de la tâche s'il est sur
/// cette machine (un ancêtre du manifeste de la tâche qui porte `arbre/`), sinon
/// le dossier courant.
pub fn dossier_de_commande(manifeste: Option<&str>) -> std::path::PathBuf {
    let courant = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let Some(m) = manifeste else { return courant };
    std::path::Path::new(m).ancestors().map(|a| a.join("arbre")).find(|a| a.is_dir()).unwrap_or(courant)
}

/// Ce que Tab peut proposer pour `debut`.
fn sources_de(debut: &str, agent: &ureq::Agent, ep: &Endpoint, tache: &Tache, load_state: &dyn Fn(&str) -> Result<Value, String>) -> crate::commandes::Sources {
    let mut s = crate::commandes::Sources { themes: crate::theme::THEMES.iter().map(|(n, _)| n.to_string()).collect(), ..Default::default() };
    if debut.starts_with("/model") {
        if let Ok(v) = modeles_de_tache(agent, ep, tache) {
            s.modeles = v["models"].as_array().into_iter().flatten().filter_map(|m| m["name"].as_str().map(str::to_string)).collect();
        }
    }
    if debut.starts_with("/tache") || debut.starts_with("/depot") {
        if let Ok(etat) = load_state("/ide/etat") {
            for b in etat["backends"].as_array().cloned().unwrap_or_default() {
                match b["tache"]["nom"].as_str() {
                    Some(t) if b["tache"]["parent"].as_str() == Some(tache.depot.id.as_str()) => s.taches.push(t.to_string()),
                    None => s.depots.extend(b["repo"].as_str().map(str::to_string)),
                    _ => {}
                }
            }
        }
    }
    if let Some(reste) = debut.strip_prefix('!') {
        match crate::commandes::mot_de_fichier(debut) {
            None if !reste.contains(char::is_whitespace) => s.executables = executables(reste),
            Some(mot) => s.fichiers = fichiers(&dossier_de_commande(manifeste_de_tache(agent, ep, tache).as_deref()), mot),
            None => {}
        }
    }
    s
}

/// Le manifeste du worktree de la tâche, d'après `/ide/teammate`.
fn manifeste_de_tache(agent: &ureq::Agent, ep: &Endpoint, tache: &Tache) -> Option<String> {
    let chemin = format!("/ide/teammate?backend={}&task={}", client::encoder(&tache.depot.id), client::encoder(&tache.task));
    client::lire(agent, ep, &chemin).ok().and_then(|v| v["backend"].as_str().map(str::to_string))
}

/// Les exécutables du `PATH` qui commencent par `prefixe` (au plus 200).
fn executables(prefixe: &str) -> Vec<String> {
    let mut out = Vec::new();
    for d in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        let Ok(entrees) = std::fs::read_dir(&d) else { continue };
        for e in entrees.flatten() {
            let nom = e.file_name().to_string_lossy().into_owned();
            if nom.starts_with(prefixe) && e.metadata().is_ok_and(|m| m.is_file()) {
                out.push(nom);
            }
        }
        if out.len() > 200 {
            break;
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Les chemins sous `dossier` qui commencent par `mot` (un dossier finit par `/`).
fn fichiers(dossier: &std::path::Path, mot: &str) -> Vec<String> {
    let (partie, prefixe) = match mot.rfind('/') {
        Some(i) => (&mot[..=i], &mot[i + 1..]),
        None => ("", mot),
    };
    let base = if partie.starts_with('/') { std::path::PathBuf::from(partie) } else if let Some(r) = partie.strip_prefix("~/") { std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(r)).unwrap_or_default() } else { dossier.join(partie) };
    let Ok(entrees) = std::fs::read_dir(&base) else { return Vec::new() };
    let mut out: Vec<String> = entrees
        .flatten()
        .filter_map(|e| {
            let nom = e.file_name().to_string_lossy().into_owned();
            (nom.starts_with(prefixe) && (prefixe.starts_with('.') || !nom.starts_with('.'))).then(|| format!("{partie}{nom}{}", if e.metadata().is_ok_and(|m| m.is_dir()) { "/" } else { "" }))
        })
        .collect();
    out.sort();
    out
}

/// **Une commande du terminal** : rend une fin quand elle quitte ou change de cible.
fn executer(c: crate::commandes::Commande, terminal: &mut ratatui::DefaultTerminal, vue: &mut Vue, agent: &ureq::Agent, ep: &Endpoint, tache: &Tache, journal: &mut Option<Journal>) -> Result<Option<Fin>, String> {
    use crate::commandes::{Commande, COMMANDES};
    match c {
        Commande::Aide => {
            vue.note("les commandes (Tab les complète) :");
            for (n, aide) in COMMANDES {
                vue.note(&format!("  /{n} — {aide}"));
            }
            vue.note("  !<commande> — la lance ici ; sa sortie part à l'équipier");
            vue.note("  //… — envoie un texte qui commence par /");
            vue.note("les touches : Entrée envoie, Maj- ou Alt-Entrée va à la ligne, ↑ ↓ l'historique, Ctrl-O un appel d'outil, o / n un accord, Échap ou Ctrl-G arrête, Ctrl-C vide puis quitte");
        }
        Commande::Quitter => return Ok(Some(Fin::Quitter)),
        Commande::Arreter => {
            match client::poster(agent, ep, "/ide/teammate/mailbox", &json!({ "backend": tache.depot.id, "task": tache.task, "kind": "stop", "text": "" })) {
                Ok(_) => vue.note("arrêt demandé"),
                Err(e) => vue.note(&format!("l'arrêt est refusé : {e}")),
            }
        }
        Commande::Theme(None) => vue.note(&format!("thèmes : {} (actuel : {})", crate::theme::THEMES.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", "), vue.theme.nom)),
        Commande::Theme(Some(n)) => match crate::theme::Theme::par_nom(&n) {
            Ok(t) => {
                vue.theme = t;
                vue.note(&format!("thème : {n}"));
            }
            Err(e) => vue.note(&e),
        },
        Commande::Tache(None) | Commande::Depot(None) => vue.note("donne un nom (Tab complète) : /tache <nom>, /depot <nom>"),
        Commande::Tache(Some(t)) => return Ok(Some(Fin::Changer { depot: None, tache: Some(t), journal: None, theme: vue.theme.clone() })),
        Commande::Depot(Some(d)) => return Ok(Some(Fin::Changer { depot: Some(d), tache: None, journal: None, theme: vue.theme.clone() })),
        Commande::Journal => match journal {
            Some(j) => vue.note(&format!("la séance s'écrit dans {} : {}", j.chemin().display(), j.relecture())),
            None => vue.note("aucun journal pour cette séance : relancer avec --journal <fichier>"),
        },
        Commande::Effacer => {
            use ratatui::crossterm::{cursor::MoveTo, terminal::{Clear, ClearType}};
            let hauteur = terminal.get_frame().area().height;
            ratatui::crossterm::execute!(std::io::stdout(), Clear(ClearType::All), MoveTo(0, 0)).map_err(|e| e.to_string())?;
            *terminal = ratatui::Terminal::with_options(ratatui::backend::CrosstermBackend::new(std::io::stdout()), TerminalOptions { viewport: Viewport::Inline(hauteur) }).map_err(|e| e.to_string())?;
        }
        Commande::EffacerHistorique => match crate::historique::chemin().map(|h| crate::historique::effacer(&h)) {
            Some(Ok(())) => {
                vue.reprendre_historique(Vec::new());
                vue.note("l'historique des envois est effacé");
            }
            Some(Err(e)) => vue.note(&e),
            None => vue.note("aucun dossier d'état : pas d'historique gardé"),
        },
        Commande::Contexte => match backend_de_tache(agent, ep, tache).map(|b| client::lire(agent, ep, &format!("/ide/chat?backend={}", client::encoder(&b)))) {
            Some(Ok(v)) => vue.note(&contexte_dit(&v["context"])),
            Some(Err(e)) => vue.note(&format!("le contexte ne se lit pas : {e}")),
            None => vue.note("la tâche n'a pas encore de worktree : pas de contexte"),
        },
        Commande::Modele(None) => match modeles_de_tache(agent, ep, tache) {
            Ok(v) => modeles_dits(&v).iter().for_each(|l| vue.note(l)),
            Err(e) => vue.note(&format!("les modèles ne se lisent pas : {e}")),
        },
        Commande::Modele(Some(n)) => match client::poster(agent, ep, "/ide/teammate/model", &json!({ "backend": tache.depot.id, "task": tache.task, "name": n })) {
            Ok(v) => {
                let modele = v["models"].as_array().into_iter().flatten().find(|m| m["current"] == true).and_then(|m| m["model"].as_str()).unwrap_or("?").to_string();
                vue.note(&format!("la tâche emploie maintenant le modèle « {n} » ({modele})"));
            }
            Err(e) => vue.note(&format!("/model {n} : {e}")),
        },
        Commande::Nom(None) => vue.note(&match vue.nom_de_l_agent() {
            Some(n) => format!("l'agent de la tâche s'appelle « {n} » (/nom <x> le renomme)"),
            None => "l'agent n'a pas encore dit son nom (/nom <x> le nomme)".to_string(),
        }),
        Commande::Nom(Some(n)) => {
            let n = if n == "-" { String::new() } else { n };
            let r = client::poster(agent, ep, "/ide/teammate/name", &json!({ "backend": tache.depot.id, "task": tache.task, "agent_name": n }));
            if let Some(g) = nom_garde(&n, &r) {
                vue.nommer(&g);
            }
            vue.note(&nom_dit(&n, r));
        }
        Commande::Etapes => {
            vue.montrer_etapes = !vue.montrer_etapes;
            vue.note(if vue.montrer_etapes { "les étapes de l'équipier s'écrivent dans l'historique (/etapes les cache)" } else { "les étapes de l'équipier défilent dans la zone vivante (/etapes les montre)" });
        }
        Commande::Shell(cmd) => lancer_une_commande(&cmd, terminal, vue, agent, ep, tache, journal)?,
        Commande::Inconnue(n) => vue.note(&format!("commande inconnue : /{n} — /aide les liste")),
    }
    Ok(None)
}

/// **Une commande `!`** (tty-13) : l'écran lui passe, dans un pseudo-terminal à
/// elle (un sudo y lit son mot de passe, sans écho) ; puis la zone vivante
/// revient, et sa sortie part à l'équipier en consigne.
fn lancer_une_commande(cmd: &str, terminal: &mut ratatui::DefaultTerminal, vue: &mut Vue, agent: &ureq::Agent, ep: &Endpoint, tache: &Tache, journal: &mut Option<Journal>) -> Result<(), String> {
    use std::io::Write;
    let dossier = dossier_de_commande(manifeste_de_tache(agent, ep, tache).as_deref());
    vue.note(&format!("» !{cmd}  (dans {})", dossier.display()));
    dessiner(terminal, vue, &|_, _| None, journal.as_mut())?;
    // L'écran à la commande : la zone vivante s'efface, le curseur va à son début,
    // le collage en bloc et le protocole clavier de kitty se rendent le temps qu'elle tourne.
    let zone = terminal.get_frame().area();
    terminal.clear().map_err(|e| e.to_string())?;
    terminal.set_cursor_position((0, zone.y)).map_err(|e| e.to_string())?;
    terminal.show_cursor().map_err(|e| e.to_string())?;
    let _ = ratatui::crossterm::execute!(std::io::stdout(), DisableBracketedPaste);
    if vue.maj_entree {
        let _ = ratatui::crossterm::execute!(std::io::stdout(), PopKeyboardEnhancementFlags);
    }
    let taille = terminal.size().map_err(|e| e.to_string())?;
    let mut ecran = std::io::stdout();
    let resultat = crate::coquille::lancer(cmd, &dossier, taille.width, taille.height, 0, &mut ecran);
    let _ = write!(ecran, "\r\n");
    let _ = ecran.flush();
    let _ = ratatui::crossterm::execute!(std::io::stdout(), EnableBracketedPaste);
    if vue.maj_entree {
        let _ = ratatui::crossterm::execute!(std::io::stdout(), PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES));
    }
    *terminal = ratatui::Terminal::with_options(ratatui::backend::CrosstermBackend::new(std::io::stdout()), TerminalOptions { viewport: Viewport::Inline(zone.height) }).map_err(|e| e.to_string())?;
    match resultat {
        Ok(r) => {
            let code = r.code.map(|c| format!("code {c}")).unwrap_or_else(|| "arrêtée par un signal".into());
            vue.note(&format!("!{cmd} : {code} ; la sortie part à l'équipier"));
            let genre = vue.genre_d_un_envoi_du_terminal();
            if let Err(e) = client::poster(agent, ep, "/ide/teammate/mailbox", &json!({ "backend": tache.depot.id, "task": tache.task, "kind": genre, "text": crate::coquille::consigne(cmd, &dossier, &r) })) {
                vue.note(&format!("la sortie est refusée : {e}"));
            }
        }
        Err(e) => vue.note(&format!("!{cmd} : {e}")),
    }
    Ok(())
}

/// Les lignes de `/model` : les modèles déclarés, le nom et le modèle d'abord.
/// Le nom que l'hôte a gardé pour l'agent, selon sa réponse à `/nom <n>`.
pub fn nom_garde(n: &str, r: &Result<Value, String>) -> Option<String> {
    let v = r.as_ref().ok()?;
    let nom = v["agent_name"].as_str().unwrap_or(n);
    (!nom.is_empty()).then(|| nom.to_string())
}

/// Ce que `/nom <n>` dit, selon la réponse de l'hôte (`POST /ide/teammate/name`, chez ED) : le nom
/// gardé, celui du dépôt pour un nom vide, un refus tel quel, et un hôte qui n'a pas encore la route.
pub fn nom_dit(n: &str, r: Result<Value, String>) -> String {
    match r {
        Ok(v) => {
            let nom = v["agent_name"].as_str().unwrap_or(n);
            if n.is_empty() {
                format!("l'agent de la tâche reprend le nom de son dépôt : « {nom} »")
            } else {
                format!("l'agent de la tâche s'appelle maintenant « {nom} »")
            }
        }
        Err(e) if e.contains("route inconnue") => "cet hôte ne sait pas encore renommer l'agent (POST /ide/teammate/name arrive chez ED)".to_string(),
        Err(e) => format!("/nom {n} : {e}"),
    }
}

pub fn modeles_dits(v: &Value) -> Vec<String> {
    let modeles = v["models"].as_array().cloned().unwrap_or_default();
    if modeles.is_empty() {
        return vec!["le manifeste de la tâche ne déclare aucun modèle de langage (models.llm)".to_string()];
    }
    let mut out = vec!["les modèles de la tâche (/model <nom> en change) :".to_string()];
    if v["current_unknown"] == true {
        out.push(format!("  ▸ {} (inconnu) — le choix de la tâche ne désigne plus aucun modèle déclaré", v["current"].as_str().unwrap_or("?")));
    }
    for m in modeles {
        let acces: Vec<&str> = [m["provider"].as_str(), m["protocol"].as_str()].into_iter().flatten().collect();
        let acces = if acces.is_empty() { String::new() } else { format!(" ({})", acces.join(", ")) };
        let mut marques = Vec::new();
        if m["current"] == true {
            marques.push("celui de la tâche");
        }
        if m["default"] == true {
            marques.push("par défaut");
        }
        let marques = if marques.is_empty() { String::new() } else { format!(" · {}", marques.join(" · ")) };
        let puce = if m["current"] == true { "  ▸ " } else { "    " };
        out.push(format!("{puce}{} — {}{acces}{marques}", m["name"].as_str().unwrap_or("?"), m["model"].as_str().unwrap_or("?")));
    }
    out
}

/// Les modèles de la tâche, par `GET /ide/teammate/models`.
fn modeles_de_tache(agent: &ureq::Agent, ep: &Endpoint, tache: &Tache) -> Result<Value, String> {
    client::lire(agent, ep, &format!("/ide/teammate/models?backend={}&task={}", client::encoder(&tache.depot.id), client::encoder(&tache.task)))
}

/// « contexte : 12 345 / 1 048 576 jetons (1,2 %) — historique 60 %, outils 25 % ».
pub fn contexte_dit(c: &Value) -> String {
    let milliers = |n: u64| {
        let s = n.to_string();
        let mut out = String::new();
        for (i, ch) in s.chars().enumerate() {
            if i > 0 && (s.len() - i).is_multiple_of(3) {
                out.push('\u{202f}');
            }
            out.push(ch);
        }
        out
    };
    let Some(used) = c["used"].as_u64() else { return "le contexte n'est pas encore connu : aucun appel au modèle".into() };
    let mut dit = match c["max"].as_u64().filter(|m| *m > 0) {
        Some(m) => format!("contexte : {} / {} jetons ({:.1} %)", milliers(used), milliers(m), used as f64 * 100.0 / m as f64).replace('.', ","),
        None => format!("contexte : {} jetons (fenêtre du modèle inconnue)", milliers(used)),
    };
    if let Some(parts) = c["parts"].as_object() {
        let total: f64 = parts.values().filter_map(Value::as_f64).sum();
        if total > 0.0 {
            let mut p: Vec<(&String, f64)> = parts.iter().filter_map(|(k, v)| v.as_f64().map(|x| (k, x))).collect();
            p.sort_by(|a, b| b.1.total_cmp(&a.1));
            let t: Vec<String> = p.iter().map(|(k, x)| format!("{k} {:.0} %", x * 100.0 / total)).collect();
            dit.push_str(&format!(" — {}", t.join(", ")));
        }
    }
    dit
}

/// Ajoute un envoi à l'historique gardé ; une erreur ne coupe pas la séance.
fn garder(texte: &str) {
    if let Some(h) = crate::historique::chemin() {
        let _ = crate::historique::ajouter(&h, texte);
    }
}

/// **La visionneuse, dans un écran alterné** (tty-11) : l'écran principal, son
/// défilement et son historique restent intacts derrière, et reviennent tels
/// quels à la fermeture. Les lignes de chaque appel se calculent une fois (la
/// coloration passe par l'hôte).
fn visionner(vue: &Vue, colorer: Colorer) -> Result<(), String> {
    use ratatui::crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
    ratatui::crossterm::execute!(std::io::stdout(), EnterAlternateScreen).map_err(|e| e.to_string())?;
    let rendu = (|| -> Result<(), String> {
        let mut t = ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout())).map_err(|e| e.to_string())?;
        let mut v = crate::visionneuse::Visionneuse::sur_le_dernier(vue.appels.len());
        let mut cache: std::collections::HashMap<(usize, u16), Vec<ratatui::text::Line<'static>>> = std::collections::HashMap::new();
        loop {
            let taille = t.size().map_err(|e| e.to_string())?;
            let lignes = cache.entry((v.index, taille.width)).or_insert_with(|| crate::visionneuse::lignes(&vue.appels[v.index], &vue.theme, vue.capacites, taille.width as usize, colorer)).clone();
            t.draw(|f| crate::visionneuse::dessiner(f, &v, vue.appels.len(), &lignes, &vue.theme, vue.capacites)).map_err(|e| e.to_string())?;
            match event::read().map_err(|e| e.to_string())? {
                Event::Key(k) if k.kind == KeyEventKind::Press => {
                    if let Some(touche) = touche_de(k) {
                        if !v.touche(&touche, vue.appels.len(), lignes.len(), taille.height as usize) {
                            return Ok(());
                        }
                    }
                }
                Event::Resize(..) => t.autoresize().map_err(|e| e.to_string())?,
                _ => {}
            }
        }
    })();
    // L'écran principal revient, quoi qu'il soit arrivé dans la visionneuse.
    let _ = ratatui::crossterm::execute!(std::io::stdout(), LeaveAlternateScreen);
    rendu
}

/// **Change la hauteur de la zone vivante** : ratatui ne sait pas la changer
/// une fois ouverte, alors la zone s'efface, le curseur revient à son début, et
/// un terminal neuf s'ouvre là, à la hauteur voulue (il fait défiler l'écran
/// s'il manque de place en bas, comme à l'ouverture).
fn retailler(terminal: &mut ratatui::DefaultTerminal, hauteur: u16) -> Result<(), String> {
    let zone = terminal.get_frame().area();
    terminal.clear().map_err(|e| e.to_string())?;
    terminal.set_cursor_position((0, zone.y)).map_err(|e| e.to_string())?;
    let neuf = ratatui::Terminal::with_options(ratatui::backend::CrosstermBackend::new(std::io::stdout()), TerminalOptions { viewport: Viewport::Inline(hauteur) }).map_err(|e| e.to_string())?;
    *terminal = neuf;
    Ok(())
}

/// **Le clavier sous tmux** (ticket « sous tmux, Maj-Entrée envoie ») : ce que permet le
/// réglage de tmux, lu par `tmux show -gv`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClavierTmux {
    /// `extended-keys` est à `on` ou `always` et `extended-keys-format` à `csi-u` : le terminal
    /// demande le mode 1 (`CSI > 4 ; 1 m`), et Maj-Entrée lui arrive en CSI u.
    Ouvert,
    /// Autre chose (le défaut de tmux, `off`, ou le format `xterm`, que crossterm ne lit pas) :
    /// rien n'est demandé, Maj-Entrée reste une Entrée, et l'indication dit les deux réglages.
    Ferme,
}

/// Ce que permet tmux, d'après `lire(["show", "-gv", <option>])` (la sortie de la commande tmux).
pub fn clavier_sous_tmux(lire: &dyn Fn(&[&str]) -> Option<String>) -> ClavierTmux {
    let option = |nom: &str| lire(&["show", "-gv", nom]).map(|v| v.trim().to_string());
    let permis = matches!(option("extended-keys").as_deref(), Some("on" | "always"));
    if permis && option("extended-keys-format").as_deref() == Some("csi-u") {
        ClavierTmux::Ouvert
    } else {
        ClavierTmux::Ferme
    }
}

/// Lance `tmux <args>` et rend sa sortie, ou rien.
fn tmux(args: &[&str]) -> Option<String> {
    let o = std::process::Command::new("tmux").args(args).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).into_owned())
}

/// La touche de la vue pour une touche du terminal, ou rien.
pub fn touche_de(k: KeyEvent) -> Option<Touche> {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let alt = k.modifiers.contains(KeyModifiers::ALT);
    match k.code {
        // AltGr (Ctrl+Alt sous Windows) : le caractère qu'il compose.
        KeyCode::Char(c) if ctrl && alt => Some(Touche::Car(c)),
        KeyCode::Char('c') if ctrl => Some(Touche::CtrlC),
        KeyCode::Char('d') if ctrl => Some(Touche::CtrlD),
        KeyCode::Char('g') if ctrl => Some(Touche::CtrlG),
        KeyCode::Char('o') if ctrl => Some(Touche::CtrlO),
        KeyCode::Char('a') if ctrl => Some(Touche::Debut),
        KeyCode::Char('e') if ctrl => Some(Touche::Fin),
        KeyCode::Char('w') if ctrl => Some(Touche::EffacerMot),
        KeyCode::Char('u') if ctrl => Some(Touche::EffacerDebut),
        KeyCode::Char('k') if ctrl => Some(Touche::EffacerFin),
        // Une autre combinaison Ctrl + lettre n'entre pas dans la saisie.
        KeyCode::Char(_) if ctrl => None,
        KeyCode::Char('b') if alt => Some(Touche::MotGauche),
        KeyCode::Char('f') if alt => Some(Touche::MotDroite),
        KeyCode::Char(_) if alt => None,
        KeyCode::Char(c) => Some(Touche::Car(c)),
        // Alt-Entrée partout ; Maj-Entrée là où le terminal la distingue.
        KeyCode::Enter if alt || k.modifiers.contains(KeyModifiers::SHIFT) => Some(Touche::Saut),
        KeyCode::Enter => Some(Touche::Entree),
        KeyCode::Backspace if alt || ctrl => Some(Touche::EffacerMot),
        KeyCode::Backspace => Some(Touche::Retour),
        KeyCode::Left if alt || ctrl => Some(Touche::MotGauche),
        KeyCode::Right if alt || ctrl => Some(Touche::MotDroite),
        KeyCode::Left => Some(Touche::Gauche),
        KeyCode::Right => Some(Touche::Droite),
        KeyCode::Home => Some(Touche::Debut),
        KeyCode::End => Some(Touche::Fin),
        KeyCode::Delete => Some(Touche::Suppr),
        KeyCode::Tab => Some(Touche::Tab),
        KeyCode::Up => Some(Touche::Haut),
        KeyCode::PageUp => Some(Touche::PageHaut),
        KeyCode::PageDown => Some(Touche::PageBas),
        KeyCode::Down => Some(Touche::Bas),
        KeyCode::Esc => Some(Touche::Echap),
        _ => None,
    }
}

/// Ce qui colore un bloc de code : `(texte, langue) → spans`, ou rien.
pub type Colorer<'a> = &'a dyn Fn(&str, &str) -> Option<Vec<crate::client::Span>>;

/// Les lignes validées dans l'historique (repliées, les blocs de code colorés
/// par `colorer`), puis la zone vivante. Les mêmes lignes, sans leurs couleurs,
/// vont au `journal` s'il y en a un.
pub fn dessiner<B: ratatui::backend::Backend>(terminal: &mut ratatui::Terminal<B>, vue: &mut Vue, colorer: Colorer, journal: Option<&mut Journal>) -> Result<(), String> {
    let validees: Vec<Validee> = vue.prendre_lignes();
    if !validees.is_empty() {
        let largeur = terminal.size().map(|s| s.width).unwrap_or(80) as usize;
        let mut lignes = Vec::new();
        for v in &validees {
            let spans = if v.genre == crate::vue::Genre::Code { colorer(&v.texte, &v.langue) } else { None };
            lignes.extend(crate::code::lignes(v, &vue.theme, vue.capacites, largeur, spans.as_deref()));
        }
        if let Some(j) = journal {
            let brutes: Vec<String> = lignes.iter().map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect()).collect();
            j.lignes(brutes.iter().map(String::as_str));
        }
        terminal
            .insert_before(lignes.len() as u16, |buf| {
                Paragraph::new(lignes).render(buf.area, buf);
            })
            .map_err(|e| format!("{e:?}"))?;
    }
    terminal.draw(|f| vue.dessiner(f, f.area())).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::{Backend, ClearType, CrosstermBackend, WindowSize};
    use ratatui::buffer::Cell;
    use ratatui::layout::{Position, Size};
    use ratatui::Terminal;
    use serde_json::json;
    use std::io::{self, Write};
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn partial_english_and_legacy_commands_collect_catalogue_before_completion() {
        let agent = client::agent();
        let endpoint = Endpoint { address: "127.0.0.1:1".into(), token: "fake-test-token".into() };
        let task = Tache {
            depot: Depot { id: "faux".into(), manifest: "/fake/backend.json".into(), repo: "faux-depot".into(), branch: "main".into() },
            task: "mesure".into(),
        };
        let catalogue = json!({"backends":[
            {"id":"faux", "repo":"faux-depot", "tache":null},
            {"id":"faux-task", "repo":"faux-depot", "tache":{"nom":"mesure", "parent":"faux"}}
        ]});
        for (input, expected) in [
            ("/tache mes", "/tache mesure"),
            ("/depot faux-d", "/depot faux-depot"),
            ("/task mes", "/task mesure"),
            ("/repo faux-d", "/repo faux-depot"),
        ] {
            let requests = std::cell::RefCell::new(Vec::new());
            let load_state = |path: &str| -> Result<Value, String> {
                requests.borrow_mut().push(path.to_string());
                Ok(catalogue.clone())
            };
            let sources = sources_de(input, &agent, &endpoint, &task, &load_state);
            assert_eq!(*requests.borrow(), vec!["/ide/etat"], "catalogue request for {input}");
            let completion = crate::commandes::completer(input, &sources).expect("live catalogue candidates");
            assert_eq!(completion.texte, expected, "completion for {input}");
        }
    }

    /// Un écrivain qui compte les octets et les jette.
    struct Compteur(Arc<AtomicUsize>);
    impl Write for Compteur {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            self.0.fetch_add(b.len(), Ordering::SeqCst);
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// Le vrai backend crossterm, sans terminal : la taille est fixée, et tout ce
    /// qu'il écrirait est compté. Ce sont les octets qui passeraient par ssh.
    struct Compte {
        inner: CrosstermBackend<Compteur>,
        taille: Size,
    }
    impl Backend for Compte {
        type Error = io::Error;
        fn draw<'a, I: Iterator<Item = (u16, u16, &'a Cell)>>(&mut self, c: I) -> io::Result<()> {
            self.inner.draw(c)
        }
        fn append_lines(&mut self, n: u16) -> io::Result<()> {
            self.inner.append_lines(n)
        }
        fn hide_cursor(&mut self) -> io::Result<()> {
            self.inner.hide_cursor()
        }
        fn show_cursor(&mut self) -> io::Result<()> {
            self.inner.show_cursor()
        }
        fn get_cursor_position(&mut self) -> io::Result<Position> {
            Ok(Position::new(0, self.taille.height - HAUTEUR))
        }
        fn set_cursor_position<P: Into<Position>>(&mut self, p: P) -> io::Result<()> {
            self.inner.set_cursor_position(p)
        }
        fn clear(&mut self) -> io::Result<()> {
            self.inner.clear()
        }
        fn clear_region(&mut self, t: ClearType) -> io::Result<()> {
            self.inner.clear_region(t)
        }
        fn size(&self) -> io::Result<Size> {
            Ok(self.taille)
        }
        fn window_size(&mut self) -> io::Result<WindowSize> {
            Ok(WindowSize { columns_rows: self.taille, pixels: Size::new(0, 0) })
        }
        fn flush(&mut self) -> io::Result<()> {
            Backend::flush(&mut self.inner)
        }
    }

    /// Les octets émis pour `n` tokens d'un mot, l'écran redessiné toutes les
    /// `par_image` tokens.
    fn octets(n: usize, par_image: usize) -> usize {
        let compte = Arc::new(AtomicUsize::new(0));
        let backend = Compte { inner: CrosstermBackend::new(Compteur(compte.clone())), taille: Size::new(80, 24) };
        let mut t = Terminal::with_options(backend, TerminalOptions { viewport: Viewport::Inline(HAUTEUR) }).unwrap();
        let mut vue = Vue::new(80, true);
        compte.store(0, Ordering::SeqCst);
        for i in 0..n {
            vue.trame(&json!({ "evenement": { "kind": "Token", "text": format!(" mot{i}") } }));
            if (i + 1) % par_image == 0 {
                dessiner(&mut t, &mut vue, &|_, _| None, None).unwrap();
            }
        }
        dessiner(&mut t, &mut vue, &|_, _| None, None).unwrap();
        compte.load(Ordering::SeqCst)
    }

    /// Les touches du terminal deviennent celles de l'éditeur : flèches, Début et
    /// Fin, Suppr, les raccourcis Ctrl et Alt d'un shell, Alt-Entrée.
    #[test]
    fn terminal_keys_map_to_the_line_editor() {
        let k = |code, m| touche_de(KeyEvent::new(code, m));
        let (rien, ctrl, alt) = (KeyModifiers::NONE, KeyModifiers::CONTROL, KeyModifiers::ALT);
        let attendus = [
            (k(KeyCode::Left, rien), Some(Touche::Gauche)),
            (k(KeyCode::Right, rien), Some(Touche::Droite)),
            (k(KeyCode::Home, rien), Some(Touche::Debut)),
            (k(KeyCode::End, rien), Some(Touche::Fin)),
            (k(KeyCode::Delete, rien), Some(Touche::Suppr)),
            (k(KeyCode::Up, rien), Some(Touche::Haut)),
            (k(KeyCode::Down, rien), Some(Touche::Bas)),
            (k(KeyCode::Left, ctrl), Some(Touche::MotGauche)),
            (k(KeyCode::Right, ctrl), Some(Touche::MotDroite)),
            (k(KeyCode::Left, alt), Some(Touche::MotGauche)),
            (k(KeyCode::Right, alt), Some(Touche::MotDroite)),
            (k(KeyCode::Char('b'), alt), Some(Touche::MotGauche)),
            (k(KeyCode::Char('f'), alt), Some(Touche::MotDroite)),
            (k(KeyCode::Char('a'), ctrl), Some(Touche::Debut)),
            (k(KeyCode::Char('e'), ctrl), Some(Touche::Fin)),
            (k(KeyCode::Char('w'), ctrl), Some(Touche::EffacerMot)),
            (k(KeyCode::Backspace, alt), Some(Touche::EffacerMot)),
            (k(KeyCode::Char('u'), ctrl), Some(Touche::EffacerDebut)),
            (k(KeyCode::Char('k'), ctrl), Some(Touche::EffacerFin)),
            (k(KeyCode::Enter, alt), Some(Touche::Saut)),
            (k(KeyCode::Enter, KeyModifiers::SHIFT), Some(Touche::Saut)),
            (k(KeyCode::Char('c'), ctrl), Some(Touche::CtrlC)),
            (k(KeyCode::Char('g'), ctrl), Some(Touche::CtrlG)),
            (k(KeyCode::Char('x'), ctrl), None),
            (k(KeyCode::Char('é'), rien), Some(Touche::Car('é'))),
            (k(KeyCode::Char('B'), KeyModifiers::SHIFT), Some(Touche::Car('B'))),
            (k(KeyCode::Enter, rien), Some(Touche::Entree)),
            // AltGr d'un clavier français arrive comme Ctrl+Alt sous Windows : c'est un caractère.
            (k(KeyCode::Char('o'), ctrl), Some(Touche::CtrlO)),
            (k(KeyCode::Tab, rien), Some(Touche::Tab)),
            (k(KeyCode::PageUp, rien), Some(Touche::PageHaut)),
            (k(KeyCode::PageDown, rien), Some(Touche::PageBas)),
            (k(KeyCode::Char('@'), ctrl | alt), Some(Touche::Car('@'))),
            (k(KeyCode::Char('{'), ctrl | alt), Some(Touche::Car('{'))),
        ];
        let faux: Vec<_> = attendus.iter().enumerate().filter(|(_, (vu, attendu))| vu != attendu).map(|(i, (vu, attendu))| format!("#{i} : {vu:?} au lieu de {attendu:?}")).collect();
        assert!(faux.is_empty(), "{faux:#?}");
    }

    /// `/nom <x>` dit le nom que l'hôte rend (un nom vide rend celui du backend) ; un refus le dit ; un
    /// hôte qui n'a pas encore la route (404) le dit en clair.
    #[test]
    fn nom_says_the_name_the_host_kept_or_why_not() {
        assert_eq!(nom_dit("Ombre", Ok(json!({ "agent_name": "Ombre" }))), "l'agent de la tâche s'appelle maintenant « Ombre »");
        assert_eq!(nom_dit("", Ok(json!({ "agent_name": "Algareth" }))), "l'agent de la tâche reprend le nom de son dépôt : « Algareth »");
        assert_eq!(nom_dit("Om bre", Err("POST /ide/teammate/name : agent_name : un nom sans blanc (400)".into())), "/nom Om bre : POST /ide/teammate/name : agent_name : un nom sans blanc (400)");
        assert_eq!(nom_dit("Ombre", Err("POST /ide/teammate/name : route inconnue : POST /ide/teammate/name (404)".into())), "cet hôte ne sait pas encore renommer l'agent (POST /ide/teammate/name arrive chez ED)");
        assert_eq!(nom_dit("Ombre", Err("POST /ide/teammate/name : tâche inconnue : doc (404)".into())), "/nom Ombre : POST /ide/teammate/name : tâche inconnue : doc (404)", "un autre 404 se dit tel quel");
    }

    /// Le nom que l'hôte a gardé, que la vue retient (`/nom` seul le dit ensuite) ; un refus n'en garde aucun.
    #[test]
    fn the_name_kept_by_the_host_is_the_one_the_view_keeps() {
        assert_eq!(nom_garde("Ombre", &Ok(json!({ "agent_name": "Ombre" }))), Some("Ombre".to_string()));
        assert_eq!(nom_garde("", &Ok(json!({ "agent_name": "Algareth" }))), Some("Algareth".to_string()), "un nom vide : celui du dépôt");
        assert_eq!(nom_garde("Ombre", &Err("POST /ide/teammate/name : route inconnue : POST /ide/teammate/name (404)".into())), None);
    }

    /// `/model` dit les modèles déclarés : le nom et le modèle d'abord, la manière d'y
    /// accéder ensuite, le défaut et celui de la tâche marqués.
    #[test]
    fn model_lists_names_and_models_marking_the_default_and_the_current() {
        let v = json!({ "models": [
            { "name": "rapide", "model": "gemini-3.8-flash", "provider": "compatible", "protocol": "vertex", "default": false, "current": true },
            { "name": "fin", "model": "claude-opus-5-5", "provider": "compatible", "protocol": "anthropic", "default": true, "current": false }
        ], "current": "rapide", "default": "fin" });
        assert_eq!(modeles_dits(&v), vec![
            "les modèles de la tâche (/model <nom> en change) :".to_string(),
            "  ▸ rapide — gemini-3.8-flash (compatible, vertex) · celui de la tâche".to_string(),
            "    fin — claude-opus-5-5 (compatible, anthropic) · par défaut".to_string(),
        ]);
        assert_eq!(modeles_dits(&json!({ "models": [] })), vec!["le manifeste de la tâche ne déclare aucun modèle de langage (models.llm)".to_string()]);
        let inconnu = json!({ "models": [{ "name": "vite", "model": "m", "default": true, "current": false }], "current": "rapide", "current_unknown": true });
        assert_eq!(modeles_dits(&inconnu)[1], "  ▸ rapide (inconnu) — le choix de la tâche ne désigne plus aucun modèle déclaré", "le choix inconnu se dit (ED)");
    }

    /// **Sous tmux** : les touches étendues ne se demandent que si tmux les permet ET les rend en
    /// CSI u ; le format `xterm` de tmux (`ESC[27;2;13~`) n'est pas lu par crossterm, et le
    /// demander perdrait Maj-Entrée tout à fait.
    #[test]
    fn under_tmux_extended_keys_are_asked_only_when_tmux_allows_them_in_csi_u() {
        let reglage = |ek: Option<&'static str>, format: Option<&'static str>| {
            move |args: &[&str]| -> Option<String> {
                match args {
                    ["show", "-gv", "extended-keys"] => ek.map(str::to_string),
                    ["show", "-gv", "extended-keys-format"] => format.map(str::to_string),
                    _ => None,
                }
            }
        };
        assert_eq!(clavier_sous_tmux(&reglage(Some("off"), Some("xterm"))), ClavierTmux::Ferme, "le défaut de tmux");
        assert_eq!(clavier_sous_tmux(&reglage(Some("on"), Some("xterm"))), ClavierTmux::Ferme, "le format xterm, que crossterm ne lit pas");
        assert_eq!(clavier_sous_tmux(&reglage(Some("on"), Some("csi-u"))), ClavierTmux::Ouvert);
        assert_eq!(clavier_sous_tmux(&reglage(Some("always"), Some("csi-u"))), ClavierTmux::Ouvert);
        assert_eq!(clavier_sous_tmux(&reglage(Some("on\n"), Some("csi-u\n"))), ClavierTmux::Ouvert, "la fin de ligne de tmux");
        assert_eq!(clavier_sous_tmux(&reglage(None, None)), ClavierTmux::Ferme, "un tmux qui ne répond pas");
        assert_eq!(clavier_sous_tmux(&reglage(Some("on"), None)), ClavierTmux::Ferme, "un tmux trop vieux pour le format");
    }

    /// `/context` dit le remplissage du contexte du modèle, en jetons et en
    /// pour cent, et ce qui pèse, du plus lourd au plus léger.
    #[test]
    fn the_context_says_its_fill_and_what_weighs() {
        let c = json!({ "used": 12345, "max": 1048576, "parts": { "outils": 0.25, "historique": 0.6, "instructions": 0.15 } });
        assert_eq!(contexte_dit(&c), "contexte : 12\u{202f}345 / 1\u{202f}048\u{202f}576 jetons (1,2 %) — historique 60 %, outils 25 %, instructions 15 %");
        assert_eq!(contexte_dit(&json!({ "used": 900, "max": null })), "contexte : 900 jetons (fenêtre du modèle inconnue)");
        assert_eq!(contexte_dit(&json!({})), "le contexte n'est pas encore connu : aucun appel au modèle");
    }

    /// Une commande `!` tourne dans le worktree de la tâche s'il est sur cette
    /// machine (un ancêtre du manifeste porte `arbre/`), sinon dans le dossier courant.
    #[test]
    fn a_bang_command_runs_in_the_task_worktree_when_it_is_here() {
        let d = std::env::temp_dir().join(format!("tty-cwd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("tache/arbre")).unwrap();
        std::fs::create_dir_all(d.join("tache/declarations/backends/code")).unwrap();
        let m = d.join("tache/declarations/backends/code/x.json");
        assert_eq!(dossier_de_commande(Some(m.to_str().unwrap())), d.join("tache/arbre"));
        assert_eq!(dossier_de_commande(Some("/nulle/part/x.json")), std::env::current_dir().unwrap());
        assert_eq!(dossier_de_commande(None), std::env::current_dir().unwrap());
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// Le journal reçoit, en texte brut, chaque ligne envoyée dans l'historique,
    /// dans l'ordre : le texte replié à la largeur et le bloc de code entier.
    #[test]
    fn the_journal_receives_every_line_sent_to_the_history_without_colours() {
        let d = std::env::temp_dir().join(format!("tty-dessin-journal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mut j = Journal::ouvrir(&d.join("j.txt")).unwrap();
        let backend = Compte { inner: CrosstermBackend::new(Compteur(Arc::new(AtomicUsize::new(0)))), taille: Size::new(40, 24) };
        let mut t = Terminal::with_options(backend, TerminalOptions { viewport: Viewport::Inline(HAUTEUR) }).unwrap();
        let mut vue = Vue::new(40, true);
        for i in 0..30 {
            vue.trame(&json!({ "evenement": { "kind": "Token", "text": format!(" mot{i}") } }));
            dessiner(&mut t, &mut vue, &|_, _| None, Some(&mut j)).unwrap();
        }
        vue.trame(&json!({ "evenement": { "kind": "Token", "text": "\n```rust\nfn main() {}\n```\n" } }));
        dessiner(&mut t, &mut vue, &|_, _| None, Some(&mut j)).unwrap();
        let texte = std::fs::read_to_string(j.chemin()).unwrap();
        let mots: Vec<String> = texte.split_whitespace().filter(|m| m.starts_with("mot")).map(String::from).collect();
        assert_eq!(mots, (0..30).map(|i| format!("mot{i}")).collect::<Vec<_>>(), "{texte}");
        assert!(texte.contains("fn main() {}"), "{texte}");
        assert!(!texte.contains('\x1b'), "aucune séquence de couleur : {texte:?}");
        assert!(texte.lines().all(|l| l.chars().count() <= 40), "replié à la largeur : {texte}");
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// Les octets émis pour `n` images d'une animation, la vue en `etat`.
    fn octets_d_animation(evenement: Value, images: u64) -> usize {
        let compte = Arc::new(AtomicUsize::new(0));
        let backend = Compte { inner: CrosstermBackend::new(Compteur(compte.clone())), taille: Size::new(80, 24) };
        let mut t = Terminal::with_options(backend, TerminalOptions { viewport: Viewport::Inline(HAUTEUR) }).unwrap();
        let mut vue = Vue::new(80, true);
        vue.politiques(&json!({ "tools": [{ "name": "edit_file", "policy": { "write_files": true } }] }));
        vue.entete = "rag3db · tty-3 · tâche parle".into();
        vue.trame(&evenement);
        dessiner(&mut t, &mut vue, &|_, _| None, None).unwrap();
        let debut = std::time::Instant::now();
        compte.store(0, Ordering::SeqCst);
        let ms = vue.quelle().unwrap().de(&vue.theme).ms;
        for n in 1..=images {
            let a = debut + std::time::Duration::from_millis(n * ms);
            t.draw(|f| vue.dessiner_a(f, f.area(), a)).unwrap();
        }
        compte.load(Ordering::SeqCst)
    }

    /// **La mesure du lot tty-3** : ce que coûte une image du Daemon Code
    /// Summoning, l'écran sans autre changement (seule la ligne d'état bouge).
    /// Mesuré le 7 nov. 2026, couleur vraie, 80 colonnes, l'en-tête affiché :
    /// Cercle 82 o par image, Transmutation 60, Glitch 158. À leurs cadences,
    /// cela fait au plus 1,6 Ko/s (le Glitch, 10 images/s). Les bornes laissent un
    /// quart de marge.
    #[test]
    fn mesure_des_octets_par_image_d_animation() {
        let cercle = octets_d_animation(json!({ "evenement": { "kind": "LlmCall" } }), 100);
        let glitch = octets_d_animation(json!({ "evenement": { "kind": "ToolCallStarted", "tool": "edit_file" } }), 100);
        let lit = octets_d_animation(json!({ "evenement": { "kind": "ToolCallStarted", "tool": "read_file" } }), 100);
        eprintln!("MESURE tty-3 : 100 images — Cercle {cercle} o ({:.1} o/image), Transmutation {lit} o ({:.1}), Glitch {glitch} o ({:.1})", cercle as f64 / 100.0, lit as f64 / 100.0, glitch as f64 / 100.0);
        assert!(cercle > 0 && glitch > 0 && lit > 0);
        assert!(cercle <= 10_500, "Cercle : {cercle} o pour 100 images (mesuré 8 200)");
        assert!(lit <= 7_500, "Transmutation : {lit} o pour 100 images (mesuré 6 000)");
        assert!(glitch <= 20_000, "Glitch : {glitch} o pour 100 images (mesuré 15 812)");
    }

    /// **La mesure du lot tty-2** : ce que coûte un flux de 1 000 tokens à
    /// l'écran, une image par token (le pire) et une image pour dix (30 images/s
    /// à 300 tokens/s). Mesuré le 7 nov. 2026 : texte 6 890 o, une image par
    /// token 62 454 o (62,5 o/token), une pour dix 28 072 o (28,1 o/token) au lot
    /// tty-2 ; 63 174 et 28 792 au lot tty-3, où la ligne d'état prend la couleur
    /// vraie du thème au lieu du rouge de base (une séquence RGB par image), soit
    /// environ 280 o par image dont 69 de texte. Les bornes laissent un quart de
    /// marge sur ces chiffres : un changement qui les passe se justifie d'abord.
    #[test]
    fn mesure_des_octets_pour_mille_tokens() {
        let pire = octets(1000, 1);
        let groupe = octets(1000, 10);
        let texte: usize = (0..1000).map(|i| format!(" mot{i}").len()).sum();
        eprintln!("MESURE tty-2 : texte {texte} o ; une image par token {pire} o ({:.1} o/token) ; une pour dix {groupe} o ({:.1} o/token)", pire as f64 / 1000.0, groupe as f64 / 1000.0);
        assert!(groupe > texte, "le texte passe au moins une fois");
        assert!(groupe <= 35_000, "une image pour dix tokens : {groupe} o pour 1 000 tokens (mesuré 28 072)");
        assert!(pire <= 78_000, "une image par token : {pire} o pour 1 000 tokens (mesuré 62 454)");
    }
}
