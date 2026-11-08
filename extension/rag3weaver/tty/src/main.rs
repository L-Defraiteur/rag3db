//! **rag3weaver-tty** : le terminal de l'agent de code. Par défaut la vue inline
//! (lot tty-2) ; `--plain` écrit le texte brut, sans terminal riche (lot tty-1).
//!
//! ```text
//! rag3weaver-tty [--plain] --depot <nom|chemin> --tache <nom> [--demande "<texte>"]
//!                [--daemon-json <chemin> | --daemon-cmd "<commande>"] [--adresse hôte:port]
//!                [--depuis <id>]
//! ```
//!
//! Écrit au fil de l'eau ce que fait l'équipier de la tâche : les tokens du
//! modèle, ses outils, sa boîte. Une `--demande` lance l'équipier (une
//! `request`). Le flux se reprend tout seul après une coupure, sans perte ni
//! doublon (`Last-Event-ID`). S'arrête quand l'équipier a fini (`ended`).
//!
//! L'hôte sur une autre machine : `--daemon-cmd "ssh luciepc cat .local/share/rag3weaver/daemon.json"`
//! et `--adresse 127.0.0.1:17970` derrière un `ssh -L 17970:127.0.0.1:7970 luciepc`.

use rag3weaver_tty::{client, endpoint::Endpoint, flux, plain, tui};
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
}

fn options() -> Result<Options, String> {
    let mut args = std::env::args().skip(1);
    let mut o = Options { depot: String::new(), task: String::new(), ancien_nom: false, demande: None, daemon_json: None, daemon_cmd: None, adresse: None, depuis: None, plain: false };
    while let Some(a) = args.next() {
        let mut valeur = || args.next().ok_or_else(|| format!("{a} attend une valeur"));
        match a.as_str() {
            "--plain" => o.plain = true,
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

const AIDE: &str = "rag3weaver-tty [--plain] --depot <nom|chemin> --tache <nom> [--demande \"<texte>\"] [--daemon-json <chemin> | --daemon-cmd \"<commande>\"] [--adresse hôte:port] [--depuis <id>]";

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
    if !o.plain {
        return tui::lancer(ep, tui::Tache { depot, task: o.task }, o.demande, o.depuis);
    }
    let backend = depot.id.clone();
    let sortie = Arc::new(Mutex::new(std::io::stdout()));
    let ecrire = {
        let sortie = sortie.clone();
        move |t: &str| {
            let mut s = sortie.lock().unwrap_or_else(|p| p.into_inner());
            let _ = s.write_all(t.as_bytes());
            let _ = s.flush();
        }
    };
    let arret = Arc::new(AtomicBool::new(false));
    let lues = Arc::new(AtomicUsize::new(0));

    // La boîte n'est pas encore relayée sur le flux (lot 1 de l'équipier) :
    // on la relit toutes les 2 s, et on s'arrête à la ligne `ended`. Quand le
    // flux portera `MailboxLine`, cette relecture s'efface d'elle-même.
    let boite_au_flux = Arc::new(AtomicBool::new(false));
    let releve = {
        let (ep, agent, arret, lues, boite_au_flux, ecrire) = (ep.clone(), agent.clone(), arret.clone(), lues.clone(), boite_au_flux.clone(), ecrire.clone());
        let chemin = format!("/ide/teammate?backend={}&task={}", client::encoder(&backend), client::encoder(&o.task));
        std::thread::spawn(move || {
            while !arret.load(Ordering::SeqCst) {
                if let Ok(v) = client::lire(&agent, &ep, &chemin) {
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
