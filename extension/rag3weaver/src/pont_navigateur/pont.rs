//! **Le pont** : la corrélation des commandes par `id` sur un canal Native Messaging, `hello`
//! puis `ready`, le délai de chaque appel, la fermeture du canal, et les deux gardes de l'hôte.

use std::collections::{BTreeSet, HashMap};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::baux::{Bail, Baux, EtatBail};
use super::cadrage::{encoder, Decodeur};
use super::protocole::{Commande, Evenement, Resultat, VersExtension, VersHote, REVISION};

/// Les actions fermées tant qu'une [`Politique`] ne les ouvre pas : elles atteignent d'autres
/// sites, d'autres onglets ou le navigateur entier.
pub const ACTIONS_FERMEES: &[&str] =
    &["cdp", "chrome-call", "chrome-describe", "cookies", "network-capture-start", "network-capture-read", "stream-watch", "extension-logs"];

/// **Ce qu'on ouvre** des passe-partout : des actions de [`ACTIONS_FERMEES`] par leur nom, et pour
/// `cdp` des méthodes une à une. Rien n'est ouvert par défaut.
#[derive(Clone, Debug, Default)]
pub struct Politique {
    pub actions_ouvertes: BTreeSet<String>,
    pub methodes_cdp: BTreeSet<String>,
}

impl Politique {
    /// `Ok` si la commande peut partir ; sinon `action_closed`, qui dit quoi ouvrir.
    fn verifier(&self, c: &Commande) -> Result<(), ErreurPont> {
        if !ACTIONS_FERMEES.contains(&c.action.as_str()) {
            return Ok(());
        }
        if c.action == "cdp" {
            let methode = c.cdp_method.as_deref().unwrap_or("");
            if self.methodes_cdp.contains(methode) {
                return Ok(());
            }
            return Err(ErreurPont::nouvelle("action_closed", format!("« cdp » sur « {methode} » est fermé : la politique du pont ne liste pas cette méthode")));
        }
        if self.actions_ouvertes.contains(&c.action) {
            return Ok(());
        }
        Err(ErreurPont::nouvelle("action_closed", format!("« {} » est fermé par défaut (il atteint d'autres sites, d'autres onglets ou le navigateur) : la politique du pont ne l'ouvre pas", c.action)))
    }
}

/// Un refus ou un échec, avec son code (`timeout`, `extension_disconnected`, `protocol_mismatch`,
/// `action_closed`, `page_handed_off`, `page_released`, `frame_too_large`, ou celui de l'extension).
#[derive(Clone, Debug, PartialEq)]
pub struct ErreurPont {
    pub code: String,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl ErreurPont {
    fn nouvelle(code: &str, message: impl Into<String>) -> ErreurPont {
        ErreurPont { code: code.to_string(), message: message.into(), data: None }
    }
}

impl std::fmt::Display for ErreurPont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}

/// Ce que le `hello` de l'extension a dit.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EtatExtension {
    pub connectee: bool,
    pub browser_id: Option<String>,
    pub version: Option<String>,
    pub revision: Option<u32>,
    pub capacites: Vec<String>,
}

type Attente = mpsc::Sender<Result<Resultat, ErreurPont>>;

struct Interne {
    ecrivain: Mutex<Box<dyn Write + Send>>,
    /// Les appels en attente, par `id`, avec leur commande (pour tenir les baux au résultat).
    attente: Mutex<HashMap<String, (Commande, Attente)>>,
    baux: Mutex<Baux>,
    etat: Mutex<EtatExtension>,
    hello: Condvar,
    evenements: Mutex<Vec<Evenement>>,
    ferme: std::sync::atomic::AtomicBool,
    version_hote: String,
    politique: Politique,
    suivant: AtomicU64,
}

/// **Le pont vers une extension** : un fil lit ses trames ; [`Pont::envoyer`] attend le résultat
/// de sa commande.
#[derive(Clone)]
pub struct Pont {
    interne: Arc<Interne>,
}

fn maintenant_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

impl Pont {
    /// Démarre le pont sur ce canal (stdin et stdout de l'hôte lancé par Chrome).
    pub fn demarrer<R: Read + Send + 'static, W: Write + Send + 'static>(lecteur: R, ecrivain: W, version_hote: &str, politique: Politique) -> Pont {
        let interne = Arc::new(Interne {
            ecrivain: Mutex::new(Box::new(ecrivain)),
            attente: Default::default(),
            baux: Default::default(),
            etat: Default::default(),
            hello: Condvar::new(),
            evenements: Default::default(),
            ferme: Default::default(),
            version_hote: version_hote.to_string(),
            politique,
            suivant: AtomicU64::new(1),
        });
        let lu = interne.clone();
        std::thread::Builder::new()
            .name("pont-navigateur".into())
            .spawn(move || lire(lu, lecteur))
            .expect("le fil du pont navigateur démarre");
        Pont { interne }
    }

    /// Attend le `hello` de l'extension au plus `delai`.
    pub fn attendre_hello(&self, delai: Duration) -> bool {
        let etat = self.interne.etat.lock().unwrap_or_else(|p| p.into_inner());
        let (etat, _) = self.interne.hello.wait_timeout_while(etat, delai, |e| !e.connectee && !self.interne.ferme.load(Ordering::SeqCst)).unwrap_or_else(|p| p.into_inner());
        etat.connectee
    }

    pub fn etat(&self) -> EtatExtension {
        self.interne.etat.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// Les baux tels que l'hôte les tient.
    pub fn baux(&self) -> Vec<Bail> {
        self.interne.baux.lock().unwrap_or_else(|p| p.into_inner()).tous()
    }

    /// Les événements reçus, dans l'ordre.
    pub fn evenements(&self) -> Vec<Evenement> {
        self.interne.evenements.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// **Envoie une commande et attend son résultat** au plus `delai`. Les gardes passent avant :
    /// rien ne part vers l'extension sur un refus. Un résultat `ok: false` est une erreur à son code.
    pub fn envoyer(&self, mut commande: Commande, delai: Duration) -> Result<Resultat, ErreurPont> {
        let i = &self.interne;
        if i.ferme.load(Ordering::SeqCst) {
            return Err(ErreurPont::nouvelle("extension_disconnected", "le canal de l'extension est fermé"));
        }
        {
            let etat = i.etat.lock().unwrap_or_else(|p| p.into_inner());
            if commande.action != "ping" && etat.connectee && etat.revision != Some(REVISION) {
                return Err(ErreurPont::nouvelle(
                    "protocol_mismatch",
                    format!("l'extension parle la révision {} du protocole, cet hôte la {REVISION} : « {} » n'est pas envoyé ; mettre à jour l'hôte et l'extension ensemble", etat.revision.map_or("inconnue".into(), |r| r.to_string()), commande.action),
                ));
            }
        }
        i.politique.verifier(&commande)?;
        garder_le_passage_de_main(&i.baux.lock().unwrap_or_else(|p| p.into_inner()), &commande)?;

        commande.id = format!("{}-{}", std::process::id(), i.suivant.fetch_add(1, Ordering::SeqCst));
        commande.deadline_at = Some(maintenant_ms() + delai.as_millis() as u64);
        let trame = encoder(&serde_json::to_value(VersExtension::Command { command: commande.clone() }).expect("une commande se sérialise"))
            .map_err(|e| ErreurPont::nouvelle("frame_too_large", e))?;
        let (tx, rx) = mpsc::channel();
        let id = commande.id.clone();
        i.attente.lock().unwrap_or_else(|p| p.into_inner()).insert(id.clone(), (commande.clone(), tx));
        let ecrit = {
            let mut e = i.ecrivain.lock().unwrap_or_else(|p| p.into_inner());
            e.write_all(&trame).and_then(|_| e.flush())
        };
        if let Err(e) = ecrit {
            i.attente.lock().unwrap_or_else(|p| p.into_inner()).remove(&id);
            return Err(ErreurPont::nouvelle("extension_disconnected", format!("l'écriture vers l'extension a échoué : {e}")));
        }
        match rx.recv_timeout(delai) {
            Ok(r) => r,
            Err(_) => {
                // Un résultat qui arrive après est jeté : plus personne ne l'attend.
                i.attente.lock().unwrap_or_else(|p| p.into_inner()).remove(&id);
                Err(ErreurPont::nouvelle("timeout", format!("« {} » sans réponse après {} ms", commande.action, delai.as_millis())))
            }
        }
    }
}

/// **La garde du passage de main** : une commande qui nomme une page rendue à l'humain (ou
/// rendue tout court) est refusée par l'hôte, jusqu'à un nouveau `claim`.
fn garder_le_passage_de_main(baux: &Baux, c: &Commande) -> Result<(), ErreurPont> {
    if c.action == "claim" {
        return Ok(());
    }
    let (Some(session), Some(page)) = (&c.session, &c.page) else { return Ok(()) };
    match baux.bail(session, page).map(|b| b.etat) {
        Some(EtatBail::Handoff) => Err(ErreurPont::nouvelle("page_handed_off", format!("l'onglet {page} est rendu à l'humain : « {} » n'est pas envoyé ; un nouveau claim le reprend, après observation", c.action))),
        Some(EtatBail::Released) => Err(ErreurPont::nouvelle("page_released", format!("l'onglet {page} est rendu : « {} » n'est pas envoyé ; un nouveau claim le reprend", c.action))),
        _ => Ok(()),
    }
}

/// Le fil de lecture : les trames de l'extension, jusqu'à la fermeture du canal.
fn lire<R: Read>(i: Arc<Interne>, mut lecteur: R) {
    let mut decodeur = Decodeur::default();
    let mut tampon = vec![0u8; 64 * 1024];
    loop {
        let n = match lecteur.read(&mut tampon) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        for message in decodeur.pousser(&tampon[..n]) {
            match message.and_then(|v| serde_json::from_value::<VersHote>(v).map_err(|e| e.to_string())) {
                Ok(m) => recevoir(&i, m),
                Err(e) => eprintln!("[pont navigateur] trame écartée : {e}"),
            }
        }
    }
    i.ferme.store(true, Ordering::SeqCst);
    i.etat.lock().unwrap_or_else(|p| p.into_inner()).connectee = false;
    i.hello.notify_all();
    for (_, (c, tx)) in i.attente.lock().unwrap_or_else(|p| p.into_inner()).drain() {
        let _ = tx.send(Err(ErreurPont::nouvelle("extension_disconnected", format!("l'extension s'est déconnectée pendant « {} »", c.action))));
    }
}

fn recevoir(i: &Interne, m: VersHote) {
    match m {
        VersHote::Hello { browser_id, extension_version, protocol_revision, features } => {
            *i.etat.lock().unwrap_or_else(|p| p.into_inner()) =
                EtatExtension { connectee: true, browser_id: Some(browser_id), version: Some(extension_version), revision: protocol_revision, capacites: features };
            if let Ok(trame) = encoder(&serde_json::to_value(VersExtension::Ready { version: i.version_hote.clone() }).expect("ready se sérialise")) {
                let mut e = i.ecrivain.lock().unwrap_or_else(|p| p.into_inner());
                let _ = e.write_all(&trame).and_then(|_| e.flush());
            }
            i.hello.notify_all();
        }
        VersHote::Result { result } => {
            let Some((commande, tx)) = i.attente.lock().unwrap_or_else(|p| p.into_inner()).remove(&result.id) else { return };
            if result.ok {
                tenir_les_baux(&mut i.baux.lock().unwrap_or_else(|p| p.into_inner()), &commande, &result);
                let _ = tx.send(Ok(result));
            } else {
                let _ = tx.send(Err(ErreurPont {
                    code: result.error_code.clone().unwrap_or_else(|| "browser_command_failed".into()),
                    message: result.error.clone().unwrap_or_else(|| format!("« {} » a échoué", commande.action)),
                    data: result.data.clone(),
                }));
            }
        }
        VersHote::Event { event } => {
            {
                let mut baux = i.baux.lock().unwrap_or_else(|p| p.into_inner());
                if let (Some(s), Some(p)) = (event.session.as_deref(), event.page.as_deref()) {
                    match event.kind.as_str() {
                        "tab_acquired" | "tab_created" => baux.ouvrir(s, p, event.origin.as_deref().unwrap_or(if event.kind == "tab_created" { "agent" } else { "user" })),
                        "tab_released" => baux.rendre(s, p),
                        "tab_closed" => baux.retirer(s, p),
                        _ => {}
                    }
                } else if event.kind == "session_released" {
                    if let Some(s) = event.session.as_deref() {
                        baux.rendre_la_session(s);
                    }
                }
            }
            i.evenements.lock().unwrap_or_else(|p| p.into_inner()).push(event);
        }
    }
}

/// Ce qu'un résultat réussi change aux baux : `claim` ouvre (un onglet de l'humain), `mark`
/// note, `session-finalize` passe la main ou rend, `tabs` en `release` rend.
fn tenir_les_baux(baux: &mut Baux, c: &Commande, r: &Resultat) {
    let Some(session) = c.session.as_deref() else { return };
    let page = r.page.as_deref().or(c.page.as_deref());
    match (c.action.as_str(), page) {
        ("claim", Some(p)) => baux.ouvrir(session, p, "user"),
        ("mark", Some(p)) => baux.marquer(session, p, c.mark.as_ref().and_then(|m| m.as_str()) == Some("handoff")),
        ("session-finalize", _) => baux.finaliser(session, c.closing == Some(true)),
        ("tabs", Some(p)) if c.op.as_deref() == Some("release") => baux.rendre(session, p),
        _ => {}
    }
}
