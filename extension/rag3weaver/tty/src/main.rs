//! **rag3weaver-tty** : le terminal de l'agent de code. Par défaut la vue inline
//! (lot tty-2) ; `--plain` écrit le texte brut, sans terminal riche (lot tty-1).
//!
//! ```text
//! rag3weaver-tty [--plain] [--theme daemon|foret|cyberpunk|cyborg|alien] [--repo <nom|chemin>] [--task <nom>] [--continue] [--request "<texte>"]
//!                [--daemon-json <chemin> | --daemon-cmd "<commande>" [--user-token-cmd "<commande>"]] [--address hôte:port]
//!                [--since <id>] [--log <fichier>]
//! ```
//!
//! `--log` écrit en texte brut ce qui part dans l'historique du terminal (lot
//! tty-7). Sous mosh, qui ne garde aucun historique au-delà de l'écran, un journal
//! s'ouvre de lui-même dans `~/.local/state/rag3weaver/tty/`, et la vue dit la
//! commande qui le relit.
//!
//! Écrit au fil de l'eau ce que fait l'équipier de la tâche : les tokens du
//! modèle, ses outils, sa boîte. Une `--request` lance l'équipier (une
//! `request`). Le flux se reprend tout seul après une coupure, sans perte ni
//! doublon (`Last-Event-ID`). S'arrête quand l'équipier a fini (`ended`).
//!
//! L'hôte sur une autre machine : `--daemon-cmd "ssh luciepc cat .local/share/rag3weaver/daemon.json"`
//! et `--address 127.0.0.1:17970` derrière un `ssh -L 17970:127.0.0.1:7970 luciepc`.
//!
//! **Le titre d'humain** (mode auto, 21 novembre 2026) : ce que l'on tape au clavier (un message, une réponse, un
//! arrêt, une commande `!`) part avec `X-Rag3weaver-User`, le jeton d'utilisateur lu dans `user/user_token` à côté de
//! `daemon.json`, ou par `--user-token-cmd "ssh luciepc cat .local/share/rag3weaver/user/user_token"`. Une `--request`
//! donnée en argument ne le porte jamais.

use rag3weaver_tty::journal::{self, Journal};
use rag3weaver_tty::{client, endpoint::Endpoint, flux, plain, theme::Theme, tui};
use serde_json::{json, Value};
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
struct Options {
    depot: String,
    task: String,
    steps: bool,
    demande: Option<String>,
    daemon_json: Option<String>,
    daemon_cmd: Option<String>,
    /// The command that prints the user's token, for a host on another machine (`--user-token-cmd`).
    user_token_cmd: Option<String>,
    adresse: Option<String>,
    depuis: Option<u64>,
    plain: bool,
    /// Le thème de la vue (`--theme`, ou `RAG3WEAVER_TTY_THEME`) ; « daemon » sans rien.
    theme: Option<String>,
    /// Le journal de séance (`--journal <fichier>`, lot tty-7).
    journal: Option<String>,
    /// `--continue` : la dernière conversation du dépôt.
    continuer: bool,
}

/// The repository of a folder: its git root, or the folder itself when it has none.
fn folder_repository(folder: &std::path::Path) -> std::path::PathBuf {
    std::process::Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|t| !t.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| folder.to_path_buf())
}

/// A new conversation's name, from its date (`month0` from 0).
fn new_conversation_name(day: u32, month0: u32, hour: u32, minute: u32) -> String {
    format!("{day:02}-{:02}-{hour:02}h{minute:02}", month0 + 1)
}

/// **A card the plain mode cannot answer**: what it says, for an approval of the teammate; `None` for any other line.
fn plain_cannot_answer(l: &Value) -> Option<String> {
    if l["from"] != "teammate" || l["kind"] != "approval" {
        return None;
    }
    let quoi = l["command"].as_str().or_else(|| l["tool"].as_str()).or_else(|| l["text"].as_str()).unwrap_or("?");
    Some(format!("une carte attend ton accord : {quoi} — réponds dans la vue (rag3weaver-tty sans --plain) ou dans l'IDE"))
}

/// **Whether the plain mode stops**: only when the teammate is suspended on a card it cannot answer (a host line « en
/// attente du superviseur » after an approval still unanswered); a card while the teammate works on is said, not a stop.
fn plain_suspended_on_a_card(lines: &[Value], since: usize) -> bool {
    let answered = |id: &Value| lines.iter().any(|r| r["reply_to"] == *id && r["kind"] == "answer");
    let Some(n) = lines.iter().rposition(|l| l["from"] == "teammate" && l["kind"] == "approval" && !answered(&l["id"])) else {
        return false;
    };
    // The suspension is this session's: a host line « en attente du superviseur » after the card and after `since`.
    lines.iter().enumerate().skip((n + 1).max(since)).any(|(_, l)| l["from"] == "host" && l["text"].as_str().is_some_and(|t| t.starts_with("en attente du superviseur")))
}

/// The last conversation of a repository in the host's list of teammates.
/// What the host's answer to opening a folder says at the start (its first commit in a new folder).
fn opening_line(_r: &Value) -> Option<String> {
    None
}

fn last_conversation(list: &Value, parent: &str) -> Option<String> {
    let all = list["teammates"].as_array().or_else(|| list["result"]["teammates"].as_array())?;
    all.iter().filter(|t| t["parent"] == parent).max_by_key(|t| t["at"].as_u64().unwrap_or(0)).and_then(|t| t["task"].as_str()).map(str::to_string)
}

fn options() -> Result<Options, String> {
    parse_options(std::env::args().skip(1), std::env::var("RAG3WEAVER_TTY_THEME").ok())
}

fn parse_options(args: impl IntoIterator<Item = String>, default_theme: Option<String>) -> Result<Options, String> {
    let mut args = args.into_iter();
    let mut o = Options { depot: String::new(), task: String::new(), steps: false, demande: None, daemon_json: None, daemon_cmd: None, user_token_cmd: None, adresse: None, depuis: None, plain: false, theme: default_theme.filter(|v| !v.is_empty()), journal: None, continuer: false };
    while let Some(a) = args.next() {
        let mut valeur = || args.next().ok_or_else(|| format!("{a} attend une valeur"));
        match a.as_str() {
            "--plain" => o.plain = true,
            "--theme" => o.theme = Some(valeur()?),
            "--repo" | "--depot" | "--backend" => o.depot = valeur()?,
            "--task" | "--tache" => o.task = valeur()?,
            "--request" | "--demande" => o.demande = Some(valeur()?),
            "--log" | "--journal" => o.journal = Some(valeur()?),
            // Les étapes de l'équipier dans l'historique (sinon, elles défilent dans la zone vivante).
            "--steps" | "--etapes" => o.steps = true,
            "--daemon-json" => o.daemon_json = Some(valeur()?),
            "--daemon-cmd" => o.daemon_cmd = Some(valeur()?),
            "--user-token-cmd" => o.user_token_cmd = Some(valeur()?),
            "--address" | "--adresse" => o.adresse = Some(valeur()?),
            "--continue" => o.continuer = true,
            "--since" | "--depuis" => o.depuis = Some(valeur()?.parse().map_err(|e| format!("--since : {e}"))?),
            "-h" | "--help" => return Err(String::new()),
            autre => return Err(format!("option inconnue : {autre}")),
        }
    }
    // **Nothing is required** (Lucie, 22 November 2026): the repository is the current folder's, the conversation a
    // new one; `--repo`, `--task` and `--continue` choose otherwise.
    Ok(o)
}

const AIDE: &str = "rag3weaver-tty [--plain] [--theme daemon|foret|cyberpunk|cyborg|alien] [--repo <nom|chemin>] [--task <nom>] [--continue] [--request \"<texte>\"] [--daemon-json <chemin> | --daemon-cmd \"<commande>\" [--user-token-cmd \"<commande>\"]] [--address hôte:port] [--since <id>] [--log <fichier>] [--steps]\nAnciens noms acceptés : --depot/--backend, --tache, --demande, --journal, --etapes, --adresse, --depuis.";

fn main() {
    let o = match options() {
        Ok(o) => o,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("rag3weaver-tty : {e}");
            }
            eprintln!("{AIDE}");
            std::process::exit(2);
        }
    };
    if let Err(e) = lancer(o) {
        eprintln!("rag3weaver-tty : {e}");
        std::process::exit(1);
    }
}

fn lancer(o: Options) -> Result<(), String> {
    if o.steps { std::env::set_var("RAG3WEAVER_TTY_ETAPES", "1"); }
    let ep = match (&o.daemon_json, &o.daemon_cmd) {
        (Some(p), _) => Endpoint::depuis_fichier(std::path::Path::new(p))?,
        (None, Some(c)) => Endpoint::depuis_commande(c)?,
        (None, None) => Endpoint::par_defaut()?,
    };
    let ep = match &o.adresse {
        Some(a) => ep.a_l_adresse(a),
        None => ep,
    };
    let ep = match &o.user_token_cmd {
        Some(c) => ep.with_user_token_from_command(c)?,
        None => ep,
    };
    // Without the user's token, what is typed reaches the host without the human title: said once, nothing blocks.
    if ep.user_token.is_none() {
        eprintln!("[tty] pas de jeton d'utilisateur (user/user_token à côté de daemon.json, ou --user-token-cmd) : ce que vous tapez part sans le titre d'humain, et ne vaut pas accord en mode auto");
    }
    let agent = client::agent();
    let mut o = o;
    // **Le dépôt du dossier courant** (Lucie, 22 nov.) : sans --repo, la racine git du dossier, ou le dossier ;
    // l'hôte qui ne le connaît pas l'ouvre, comme « Ouvrir un dossier » de l'IDE.
    let depuis_le_dossier = o.depot.is_empty();
    if depuis_le_dossier {
        let ici = std::env::current_dir().map_err(|e| format!("le dossier courant : {e}"))?;
        o.depot = folder_repository(&ici).display().to_string();
    }
    let depot = match client::resoudre(&agent, &ep, &o.depot) {
        Ok(d) => d,
        Err(_) if depuis_le_dossier || std::path::Path::new(&o.depot).is_absolute() => {
            client::open_folder(&agent, &ep, &o.depot).map_err(|e| format!("l'hôte n'ouvre pas {} : {}", o.depot, tui::refusal_in_words(&e)))?;
            client::resoudre(&agent, &ep, &o.depot).map_err(|e| tui::refusal_in_words(&e))?
        }
        Err(e) => return Err(tui::refusal_in_words(&e)),
    };
    // **La conversation** : nommée, reprise (--continue), ou neuve, nommée par sa date.
    let quoi = if !o.task.is_empty() {
        format!("conversation « {} »", o.task)
    } else if o.continuer {
        let liste = client::lire(&agent, &ep, "/ide/teammate/list?supervisor=supervisor").unwrap_or_default();
        match last_conversation(&liste, &depot.id) {
            Some(t) => {
                o.task = t;
                format!("reprise de « {} »", o.task)
            }
            None => return Err(format!("aucune conversation à reprendre dans {} : lance-en une sans --continue", depot.repo)),
        }
    } else {
        let maintenant = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
        let (j, m, h, mi) = rag3weaver_tty::boite::local_parts(maintenant);
        o.task = new_conversation_name(j, m, h, mi);
        format!("conversation neuve ({})", o.task)
    };
    eprintln!("[tty] {} · {quoi}", depot.repo);
    // Le journal de séance : `--journal`, ou de lui-même sous mosh, qui ne garde
    // aucun historique au-delà de l'écran (lot tty-7). Un journal qui ne s'ouvre
    // pas de lui-même se dit, et la séance continue.
    let mosh = journal::sous_mosh();
    let mut journal = match (&o.journal, mosh) {
        (Some(c), _) => Some(Journal::ouvrir(std::path::Path::new(c))?),
        (None, true) => match journal::dossier_d_etat().ok_or_else(|| "aucun dossier d'état (ni XDG_STATE_HOME ni HOME)".to_string()).and_then(|d| Journal::neuf_dans(&d)) {
            Ok(j) => Some(j),
            Err(e) => {
                eprintln!("[tty] sous mosh, l'historique ne se relit pas au défilement, et le journal ne s'ouvre pas : {e}");
                None
            }
        },
        (None, false) => None,
    };
    let mut depot = depot;
    if !o.plain {
        // Le thème se choisit avant d'ouvrir l'écran : un nom inconnu s'y dit.
        let mut theme = Theme::par_nom(o.theme.as_deref().unwrap_or("daemon"))?;
        loop {
            match tui::lancer(ep.clone(), tui::Tache { depot: depot.clone(), task: o.task.clone() }, o.demande.clone(), o.depuis, theme, journal.take(), mosh) {
                Ok(tui::Fin::Quitter) => return Ok(()),
                // `/tache` ou `/depot` (tty-13) : la vue repart sur l'autre cible, sans
                // redemander quoi que ce soit à l'équipier.
                Ok(tui::Fin::Changer { depot: d, tache: t, journal: j, theme: th }) => {
                    if let Some(d) = d {
                        match client::resoudre(&agent, &ep, &d) {
                            Ok(neuf) => depot = neuf,
                            Err(e) => eprintln!("[tty] /depot {d} : {e}"),
                        }
                    }
                    if let Some(t) = t {
                        o.task = t;
                    }
                    o.demande = None;
                    o.depuis = None;
                    journal = j;
                    theme = th;
                }
                Err(tui::Echec::Autre(e)) => return Err(e),
                // La vue ne démarre pas (un terminal qui ne rend pas la position du
                // curseur, un lien trop lent) : on le dit, et le texte brut prend la
                // suite, plutôt que de s'arrêter.
                Err(tui::Echec::VueIndisponible(e, j)) => {
                    eprintln!("[tty] la vue ne démarre pas ({e}) : je passe en texte brut (--plain)");
                    journal = j;
                    break;
                }
            }
        }
    }
    if let (true, Some(j)) = (mosh, &journal) {
        eprintln!("[tty] sous mosh, l'historique ne se relit pas au défilement : {}", j.relecture());
    }
    let backend = depot.id.clone();
    let sortie = Arc::new(Mutex::new((std::io::stdout(), journal)));
    let ecrire = {
        let sortie = sortie.clone();
        move |t: &str| {
            let mut s = sortie.lock().unwrap_or_else(|p| p.into_inner());
            let _ = s.0.write_all(t.as_bytes());
            let _ = s.0.flush();
            if let Some(j) = s.1.as_mut() {
                j.ecrire(t);
            }
        }
    };
    let arret = Arc::new(AtomicBool::new(false));
    // **Ce qui est déjà dans la boîte n'est pas de cette séance** (T1, 16 nov.) : une demande précédente de la
    // même tâche y a laissé sa fin (`ended`), que la relecture prenait pour la sienne et qui rendait la main en
    // 3 s. On part de la longueur de la boîte avant la demande ; le manifeste de la tâche filtre le flux,
    // comme dans le terminal riche : une trame d'une autre tâche ne la regarde pas.
    let chemin = format!("/ide/teammate?backend={}&task={}", client::encoder(&backend), client::encoder(&o.task));
    let au_depart = client::lire(&agent, &ep, &chemin).ok();
    let deja = au_depart.as_ref().and_then(|v| v["mailbox"].as_array().map(Vec::len)).unwrap_or(0);
    let task_manifest: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(au_depart.as_ref().and_then(|v| v["backend"].as_str().map(str::to_string))));
    let lues = Arc::new(AtomicUsize::new(deja));

    // La boîte n'est pas encore relayée sur le flux (lot 1 de l'équipier) :
    // on la relit toutes les 2 s, et on s'arrête à la ligne `ended`. Quand le
    // flux portera `MailboxLine`, cette relecture s'efface d'elle-même.
    let boite_au_flux = Arc::new(AtomicBool::new(false));
    let releve = {
        let (ep, agent, arret, lues, boite_au_flux, ecrire, task_manifest) = (ep.clone(), agent.clone(), arret.clone(), lues.clone(), boite_au_flux.clone(), ecrire.clone(), task_manifest.clone());
        let chemin = chemin.clone();
        let (repo_id, task_name) = (backend.clone(), o.task.clone());
        std::thread::spawn(move || {
            // **The end of a request** (ticket of 16 nov., 18 nov.): the host writes `ended` once per run, but a
            // supervisor line that came in during the last step restarts the teammate right after. An `ended`
            // is the end only when no line has followed it for RESUME_GRACE and the task is no longer running.
            let mut end_seen: Option<std::time::Instant> = None;
            while !arret.load(Ordering::SeqCst) {
                if let Ok(v) = client::lire(&agent, &ep, &chemin) {
                    // La tâche peut naître de la demande : son manifeste se lit dès qu'il existe.
                    if let Some(m) = v["backend"].as_str() {
                        *task_manifest.lock().unwrap_or_else(|p| p.into_inner()) = Some(m.to_string());
                    }
                    let lignes = v["mailbox"].as_array().cloned().unwrap_or_default();
                    for l in lignes.iter().skip(lues.load(Ordering::SeqCst)) {
                        if !boite_au_flux.load(Ordering::SeqCst) {
                            ecrire(&plain::ligne_de_boite(l));
                        }
                        // **A card the plain mode cannot answer** is said once (c7, 22 nov.)…
                        if let Some(dit) = plain_cannot_answer(l).filter(|_| !lignes.iter().any(|r| r["reply_to"] == l["id"])) {
                            eprintln!("[tty] {dit}");
                        }
                        end_seen = (l["kind"] == "ended").then(std::time::Instant::now);
                    }
                    lues.store(lignes.len(), Ordering::SeqCst);
                    // … and the session stops only when the teammate is suspended on it (essai de la demande 4).
                    if plain_suspended_on_a_card(&lignes, deja) {
                        eprintln!("[tty] l'équipier attend ton accord pour continuer : réponds dans la vue (rag3weaver-tty sans --plain) ou dans l'IDE");
                        arret.store(true, Ordering::SeqCst);
                    }
                }
                if let Some(t) = end_seen {
                    let state = client::lire(&agent, &ep, "/ide/teammate/list?supervisor=supervisor").ok().and_then(|v| {
                        v["teammates"].as_array().into_iter().flatten().find(|x| x["task"].as_str() == Some(task_name.as_str()) && x["parent"].as_str().is_none_or(|p| p == repo_id)).and_then(|x| x["state"].as_str().map(str::to_string))
                    });
                    if run_is_over(t.elapsed(), state.as_deref()) {
                        arret.store(true, Ordering::SeqCst);
                    }
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        })
    };

    // La demande part une fois l'abonnement pris : rien de son run ne manque.
    let mut demande = o.demande.clone();
    let mut erreur_de_demande: Option<String> = None;
    let arret_de_la_demande = arret.clone();
    let mut ouvert = || {
        if let Some(texte) = demande.take() {
            match client::poster(&agent, &ep, "/ide/teammate/mailbox", &json!({ "backend": backend, "task": o.task, "kind": "request", "text": texte })) {
                Ok(r) => eprintln!("[tty] demande posée ({})", if r["started"] == true { "l'équipier part" } else { "l'équipier tournait déjà" }),
                // **A refused request is said at once** (c7, 22 nov.: 280 s of silence): the session stops on it.
                Err(e) => {
                    let dit = tui::refusal_in_words(&e);
                    eprintln!("[tty] la demande est refusée : {dit}");
                    erreur_de_demande = Some(dit);
                    arret_de_la_demande.store(true, Ordering::SeqCst);
                }
            }
        }
    };
    let sujets = ["agent", "tokens", "mailbox"];
    let rendu = flux::suivre_sans_fin(
        &ep,
        &sujets,
        o.depuis,
        &arret,
        &mut ouvert,
        |pourquoi| eprintln!("\n[tty] flux coupé ({pourquoi}) : je reprends"),
        |t| {
            if t.event == "lacune" {
                eprintln!("\n[tty] lacune : le tampon de l'hôte a perdu des événements ({})", t.data);
                return flux::Suite::Continuer;
            }
            if let Ok(v) = serde_json::from_str::<Value>(&t.data) {
                if let Some(m) = task_manifest.lock().unwrap_or_else(|p| p.into_inner()).as_deref() {
                    if v["backend"].as_str() != Some(m) {
                        return flux::Suite::Continuer;
                    }
                }
                if v["evenement"]["kind"] == "MailboxLine" {
                    boite_au_flux.store(true, Ordering::SeqCst);
                }
                if let Some(texte) = plain::trame(&v) {
                    ecrire(&texte);
                }
            }
            flux::Suite::Continuer
        },
    );
    arret.store(true, Ordering::SeqCst);
    let _ = releve.join();
    if let Some(e) = erreur_de_demande {
        return Err(e);
    }
    rendu.map(|_| ())
}

/// How long an `ended` must stay the last line before the plain mode takes it for the end of its request.
const RESUME_GRACE: Duration = Duration::from_secs(2);

/// **Is the request over**, an `ended` having been the last line for `since`, the task's state being `state`
/// (`None` when the host could not say)? Not while the host runs the teammate again, nor before the grace.
fn run_is_over(since: Duration, state: Option<&str>) -> bool {
    since >= RESUME_GRACE && state != Some("running")
}

#[cfg(test)]
mod run_end_tests {
    use super::*;

    #[test]
    fn an_end_is_the_end_only_after_the_grace_and_when_the_task_no_longer_runs() {
        assert!(!run_is_over(Duration::from_millis(500), Some("ended")), "too early: a restart may follow");
        assert!(!run_is_over(Duration::from_secs(3), Some("running")), "the host runs the teammate again");
        assert!(run_is_over(Duration::from_secs(3), Some("ended")));
        assert!(run_is_over(Duration::from_secs(3), Some("waiting")));
        assert!(run_is_over(Duration::from_secs(3), None), "the host cannot say: the end stands");
    }
}

#[cfg(test)]
mod english_options_tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Options, String> {
        parse_options(args.iter().map(|s| s.to_string()), None)
    }

    #[test]
    fn english_options_match_legacy_and_last_value_wins() {
        let english = parse(&["--repo", "r", "--task", "t", "--request", "hello", "--log", "l", "--address", "localhost:1", "--since", "9", "--steps"]).unwrap();
        let legacy = parse(&["--depot", "r", "--tache", "t", "--demande", "hello", "--journal", "l", "--adresse", "localhost:1", "--depuis", "9", "--etapes"]).unwrap();
        assert_eq!(english, legacy);
        assert!(AIDE.starts_with("rag3weaver-tty"));
        assert!(AIDE.contains("--repo") && AIDE.contains("Anciens noms"));
        let options = parse(&["--repo", "first", "--depot", "last", "--task", "sample",
            "--request", "hello", "--log", "session.log", "--address", "127.0.0.1:1",
            "--since", "42", "--steps"]).unwrap();
        assert_eq!(options.depot, "last");
        assert_eq!(options.task, "sample");
        assert_eq!(options.demande.as_deref(), Some("hello"));
        assert_eq!(options.journal.as_deref(), Some("session.log"));
        assert_eq!(options.adresse.as_deref(), Some("127.0.0.1:1"));
        assert_eq!(options.depuis, Some(42));
        assert!(options.steps);
        let mixed = parse(&["--backend", "repo", "--task", "sample"]).unwrap();
        assert_eq!(mixed.depot, "repo");
        assert_eq!(mixed.task, "sample");
        for flag in ["--repo", "--task", "--request", "--log", "--address", "--since"] {
            assert!(parse(&["--repo", "repo", "--task", "sample", flag]).is_err(), "{flag}");
        }
    }
}

#[cfg(test)]
mod launch_from_a_folder_tests {
    use super::*;

    /// **`rag3weaver-tty` alone starts** (Lucie, 22 November 2026: « je lance une session depuis un répertoire, déjà
    /// ça devrait commencer comme ça ») : `--repo` and `--task` are options; `--continue` takes the last conversation.
    #[test]
    fn no_argument_is_required_and_continue_is_an_option() {
        let o = parse_options(Vec::<String>::new(), None).expect("no argument is required");
        assert_eq!((o.depot.as_str(), o.task.as_str(), o.continuer), ("", "", false));
        let c = parse_options(vec!["--continue".to_string()], None).unwrap();
        assert!(c.continuer);
    }

    /// The repository is the folder's git root, or the folder itself when it has none.
    #[test]
    fn the_repository_is_the_folders_git_root_or_the_folder() {
        let d = std::env::temp_dir().join(format!("tty-dossier-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("projet/src")).unwrap();
        assert_eq!(folder_repository(&d.join("projet/src")), d.join("projet/src"), "no git: the folder itself");
        assert!(std::process::Command::new("git").args(["init", "-q"]).current_dir(d.join("projet")).status().unwrap().success());
        let root = folder_repository(&d.join("projet/src"));
        assert_eq!(root.canonicalize().unwrap(), d.join("projet").canonicalize().unwrap(), "the git root");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A new conversation is named by its date; `--continue` takes the last one of the repository.
    #[test]
    fn a_new_conversation_is_named_by_its_date_and_continue_takes_the_last() {
        assert_eq!(new_conversation_name(22, 10, 22, 41), "22-11-22h41");
        let list = serde_json::json!({ "teammates": [
            { "parent": "b1", "task": "ancienne", "state": "ended" },
            { "parent": "b2", "task": "autre-depot", "state": "ended" },
            { "parent": "b1", "task": "derniere", "state": "waiting" }
        ] });
        assert_eq!(last_conversation(&list, "b1").as_deref(), Some("derniere"));
        assert_eq!(last_conversation(&list, "b3"), None);
    }
}

#[cfg(test)]
mod plain_approval_tests {
    use super::*;

    /// **The plain mode says a card it cannot answer, and stops** (c7, 22 Nov.: 600 s waiting on an approval with no
    /// way to answer it).
    #[test]
    fn a_card_the_plain_mode_cannot_answer_is_said() {
        let card = serde_json::json!({ "from": "teammate", "kind": "approval", "command": "python3 fizzbuzz.py", "text": "commande en attente d'accord : python3 fizzbuzz.py" });
        let said = plain_cannot_answer(&card).expect("an approval is said");
        assert!(said.contains("python3 fizzbuzz.py") && said.contains("sans --plain"), "{said}");
        assert_eq!(plain_cannot_answer(&serde_json::json!({ "from": "teammate", "kind": "report", "text": "fait" })), None);
    }
}

#[cfg(test)]
mod plain_suspension_tests {
    use super::*;

    /// **The plain mode stops only when the teammate is suspended on a card** (essai de la demande 4, 23 Nov.: it
    /// stopped on the card while the teammate went on, and its answer was lost).
    #[test]
    fn the_plain_mode_stops_only_when_the_teammate_is_suspended_on_a_card() {
        let card = serde_json::json!({ "id": "b/t:3", "from": "teammate", "kind": "approval", "command": "find / -name pytest" });
        let working = vec![card.clone(), serde_json::json!({ "from": "teammate", "kind": "report", "text": "j'avance sans la commande" })];
        assert!(!plain_suspended_on_a_card(&working, 0), "a card while the teammate works on is not a stop");
        let suspended = vec![card.clone(), serde_json::json!({ "from": "host", "kind": "report", "text": "en attente du superviseur depuis 01:26 : approval b/t:3" })];
        assert!(plain_suspended_on_a_card(&suspended, 0), "suspended on it: stop");
        // **Only this session's lines** (essai de la demande 4 rejouée, 23 Nov.: the card of the previous session
        // stopped the next one in 6 s, before the teammate even read the new request).
        let mut next = suspended.clone();
        next.push(serde_json::json!({ "from": "supervisor", "kind": "request", "text": "ok corrige" }));
        assert!(!plain_suspended_on_a_card(&next, suspended.len()), "a card of an earlier session does not stop this one");
        let answered = vec![card, serde_json::json!({ "from": "host", "kind": "report", "text": "en attente du superviseur depuis 01:26 : approval b/t:3" }), serde_json::json!({ "from": "user", "kind": "answer", "reply_to": "b/t:3", "approved": true })];
        assert!(!plain_suspended_on_a_card(&answered, 0), "answered elsewhere: no stop");
    }
}

#[cfg(test)]
mod help_tests {
    use super::*;

    /// **The help says what is optional** (23 Nov.: `--help` still showed `--repo` and `--task` as required).
    #[test]
    fn the_help_shows_repo_and_task_as_options_and_names_continue() {
        assert!(AIDE.contains("[--repo <nom|chemin>]") && AIDE.contains("[--task <nom>]"), "{AIDE}");
        assert!(AIDE.contains("--continue"), "{AIDE}");
    }
}

#[cfg(test)]
mod first_commit_tests {
    use super::*;

    /// **The host's first commit in a new folder is said at the start** (23 Nov.: it was only in the folder's git).
    #[test]
    fn the_first_commit_of_a_new_folder_is_said() {
        let r = serde_json::json!({ "ok": true, "first_commit": "premier commit par rag3weaver (dossier neuf)", "git": true });
        assert_eq!(opening_line(&r).as_deref(), Some("premier commit par rag3weaver (dossier neuf)"));
        assert_eq!(opening_line(&serde_json::json!({ "ok": true })), None);
    }
}
