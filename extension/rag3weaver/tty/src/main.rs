//! **rag3weaver-tty** : le terminal de l'agent de code. Par défaut la vue inline
//! (lot tty-2) ; `--plain` écrit le texte brut, sans terminal riche (lot tty-1).
//!
//! ```text
//! rag3weaver-tty [--plain] [--theme daemon|foret|cyberpunk|cyborg|alien] --depot <nom|chemin> --tache <nom> [--demande "<texte>"]
//!                [--daemon-json <chemin> | --daemon-cmd "<commande>"] [--adresse hôte:port]
//!                [--depuis <id>] [--journal <fichier>]
//! ```
//!
//! `--journal` écrit en texte brut ce qui part dans l'historique du terminal (lot
//! tty-7). Sous mosh, qui ne garde aucun historique au-delà de l'écran, un journal
//! s'ouvre de lui-même dans `~/.local/state/rag3weaver/tty/`, et la vue dit la
//! commande qui le relit.
//!
//! Écrit au fil de l'eau ce que fait l'équipier de la tâche : les tokens du
//! modèle, ses outils, sa boîte. Une `--demande` lance l'équipier (une
//! `request`). Le flux se reprend tout seul après une coupure, sans perte ni
//! doublon (`Last-Event-ID`). S'arrête quand l'équipier a fini (`ended`).
//!
//! L'hôte sur une autre machine : `--daemon-cmd "ssh luciepc cat .local/share/rag3weaver/daemon.json"`
//! et `--adresse 127.0.0.1:17970` derrière un `ssh -L 17970:127.0.0.1:7970 luciepc`.

use rag3weaver_tty::journal::{self, Journal};
use rag3weaver_tty::{client, endpoint::Endpoint, flux, plain, theme::Theme, tui};
use serde_json::{json, Value};
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct Options {
    depot: String,
    task: String,
    /// L'ancien nom (`--backend`, `--task`) a été employé : on le dit.
    ancien_nom: bool,
    demande: Option<String>,
    daemon_json: Option<String>,
    daemon_cmd: Option<String>,
    adresse: Option<String>,
    depuis: Option<u64>,
    plain: bool,
    /// Le thème de la vue (`--theme`, ou `RAG3WEAVER_TTY_THEME`) ; « daemon » sans rien.
    theme: Option<String>,
    /// Le journal de séance (`--journal <fichier>`, lot tty-7).
    journal: Option<String>,
}

fn options() -> Result<Options, String> {
    let mut args = std::env::args().skip(1);
    let mut o = Options { depot: String::new(), task: String::new(), ancien_nom: false, demande: None, daemon_json: None, daemon_cmd: None, adresse: None, depuis: None, plain: false, theme: std::env::var("RAG3WEAVER_TTY_THEME").ok().filter(|v| !v.is_empty()), journal: None };
    while let Some(a) = args.next() {
        let mut valeur = || args.next().ok_or_else(|| format!("{a} attend une valeur"));
        match a.as_str() {
            "--plain" => o.plain = true,
            "--theme" => o.theme = Some(valeur()?),
            "--depot" => o.depot = valeur()?,
            "--tache" => o.task = valeur()?,
            // Les noms du lot tty-1, lus encore un lot (docs/07-novembre-2026-tty/02, § Vocabulaire).
            "--backend" => {
                o.depot = valeur()?;
                o.ancien_nom = true;
            }
            "--task" => {
                o.task = valeur()?;
                o.ancien_nom = true;
            }
            "--demande" => o.demande = Some(valeur()?),
            "--journal" => o.journal = Some(valeur()?),
            // Les étapes de l'équipier dans l'historique (sinon, elles défilent dans la zone vivante).
            "--etapes" => std::env::set_var("RAG3WEAVER_TTY_ETAPES", "1"),
            "--daemon-json" => o.daemon_json = Some(valeur()?),
            "--daemon-cmd" => o.daemon_cmd = Some(valeur()?),
            "--adresse" => o.adresse = Some(valeur()?),
            "--depuis" => o.depuis = Some(valeur()?.parse().map_err(|e| format!("--depuis : {e}"))?),
            "-h" | "--help" => return Err(String::new()),
            autre => return Err(format!("option inconnue : {autre}")),
        }
    }
    if o.depot.is_empty() || o.task.is_empty() {
        return Err("--depot et --tache sont exigés".into());
    }
    Ok(o)
}

const AIDE: &str = "rag3weaver-tty [--plain] [--theme daemon|foret|cyberpunk|cyborg|alien] --depot <nom|chemin> --tache <nom> [--demande \"<texte>\"] [--daemon-json <chemin> | --daemon-cmd \"<commande>\"] [--adresse hôte:port] [--depuis <id>] [--journal <fichier>] [--etapes]";

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
    let ep = match (&o.daemon_json, &o.daemon_cmd) {
        (Some(p), _) => Endpoint::depuis_fichier(std::path::Path::new(p))?,
        (None, Some(c)) => Endpoint::depuis_commande(c)?,
        (None, None) => Endpoint::par_defaut()?,
    };
    let ep = match &o.adresse {
        Some(a) => ep.a_l_adresse(a),
        None => ep,
    };
    let agent = client::agent();
    if o.ancien_nom {
        eprintln!("rag3weaver-tty : --backend et --task deviennent --depot et --tache (l'ancien nom part au prochain lot)");
    }
    // Le dépôt se désigne par son nom ou son chemin ; son id ne sert qu'au JSON.
    let etat = client::lire_brut(&agent, &ep, "/ide/etat")?;
    let depot = client::resoudre_depot(&etat, &o.depot)?;
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
    let mut o = o;
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
                        match client::lire_brut(&agent, &ep, "/ide/etat").and_then(|etat| client::resoudre_depot(&etat, &d)) {
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
        std::thread::spawn(move || {
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
                        if l["kind"] == "ended" {
                            arret.store(true, Ordering::SeqCst);
                        }
                    }
                    lues.store(lignes.len(), Ordering::SeqCst);
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        })
    };

    // La demande part une fois l'abonnement pris : rien de son run ne manque.
    let mut demande = o.demande.clone();
    let mut erreur_de_demande: Option<String> = None;
    let mut ouvert = || {
        if let Some(texte) = demande.take() {
            match client::poster(&agent, &ep, "/ide/teammate/mailbox", &json!({ "backend": backend, "task": o.task, "kind": "request", "text": texte })) {
                Ok(r) => eprintln!("[tty] demande posée ({})", if r["started"] == true { "l'équipier part" } else { "l'équipier tournait déjà" }),
                Err(e) => erreur_de_demande = Some(e),
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
