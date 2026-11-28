//! **L'IDE servi par l'hôte** (page 05 de la session everything declarative,
//! §2, lot A2) : le socle sur lequel la page se bâtit.
//!
//! La page n'est qu'une vue : elle appelle les outils que l'hôte sert déjà, et
//! se tient à jour par le flux de l'hôte. Rien ici ne garde un état qui ferait
//! foi ; le tampon du flux ne sert qu'à rejouer à une fenêtre ce qu'elle a
//! manqué pendant une coupure.
//!
//! | route | |
//! |---|---|
//! | `GET /ide/entrer?jeton=…` | le jeton local posé en cookie (`HttpOnly`, `SameSite=Lax`), puis la page |
//! | `GET /ide`, `GET /ide/` | la page nouvelle (React, page 08) : le bundle commité `templates/ide/app`, embarqué |
//! | `GET /ide/app.js`, `GET /ide/app.css` | le bundle, servi d'ici (jamais d'un CDN) |
//! | `GET /ide/…`, `GET /ide/ancienne/…` | l'ancienne page (une seule, qui lit son URL) : les vues pas encore portées |
//! | `GET /ide/page.js` | le code de la page, en fichier (lisible par les outils de code, comme le reste) |
//! | `GET /ide/graphe?backend=<id>&tool=…` | le graphe d'un outil (ou `reaction=`, `component=`), en JSON |
//! | `GET /ide/reactions?backend=<id>` | les réactions du backend, ses runs, l'état de ses montres |
//! | `GET /ide/propositions?backend=<id>[&id=<n>]` | les propositions, ou le détail d'une (diffs, graphes avant/après) |
//! | `POST /ide/propositions[/appliquer\|/rejeter]` | proposer `{backend, files, reason}`, appliquer `{backend, id}`, rejeter `{backend, id, why}` |
//! | `GET /ide/taches?backend=<id>[&nom=<n>]` | les tâches de l'agent, ou la relecture d'une (fichiers, graphes, témoins) |
//! | `POST /ide/taches[/fusionner\|/rejeter]` | ouvrir `{backend, nom}`, fusionner `{backend, nom, message}`, rejeter `{backend, nom, why}` |
//! | `GET /ide/bilans?backend=<id>` | les bilans du backend : ses fiches (via juge), leurs apports dans l'ordre et leurs notes |
//! | `GET /ide/teammate?backend=<id>&task=<nom>` | l'équipier d'une tâche : sa boîte, ses plans vivants et clos, s'il tourne |
//! | `POST /ide/teammate/mailbox` | écrire à l'équipier `{backend, task, kind, item?, text, from?, to?, reply_to?, approved?, expects?}` (`expects` : `report` ou `approval`, sur une `request`) : une `request` ouvre la tâche et le lance, un `stop` arrête le pas |
//! | `GET /ide/teammate/pending?for=<clé>` | ce qui attend la clé dans toutes les boîtes, en lecture pure : `{for, items: [{id, task, parent, kind, item?, text, at}]}` |
//! | `POST /ide/teammate/seen` | `{for, ids}` : un compte rendu, une fin, une livraison sortent de `pending` pour la clé |
//! | `GET /ide/teammate/models?backend=<id>&task=<nom>` | les modèles de langage que déclare le manifeste d'une tâche : `{models: [{name, model, provider, protocol, default, current}], current, default}` |
//! | `POST /ide/teammate/model` | `{backend, task, name}` : la tâche emploie ce modèle déclaré, gardé avec elle ; un nom inconnu est refusé en nommant la liste |
//! | `POST /ide/teammate/name` | `{backend, task, agent_name}` : l'agent de la tâche ouverte porte ce nom, gardé au registre et lu au tour suivant, sans rouvrir la tâche ; `""` rend celui du backend ; `{agent_name}` rendu ; un nom vide d'espaces ou de plusieurs lignes est refusé en nommant `agent_name` |
//! | `GET /ide/teammate/list?supervisor=<clé>` | les équipiers de ce superviseur : `{supervisor, teammates: [{task, parent, state, last_kind, at}]}` |
//! | `GET /ide/recherches?backend=<id>&qui=&limite=&avant=` | mes recherches : les appels journalisés des outils de recherche (IDE et agents), du plus récent, avec leurs clics |
//! | `POST /ide/recherches/clic` | un clic `{backend, run, uuid, rang}` sur un résultat d'une recherche : rangé dans son run, ou refusé en disant pourquoi |
//! | `POST /ide/runs/epingler` | épingler un run `{backend, run, raison}` : son journal ne tombe jamais (ni poids ni âge) |
//! | `POST /ide/bilans` | juger un run `{backend, run}` : le juge relit son journal et range sa fiche dans la mémoire déclarée (`bilans`) |
//! | `POST /ide/runs/arreter` | arrêter un run en vol `{backend, run}`, au nom de l'IDE (C4) |
//! | `GET /ide/chat?backend=<id>` | le chat de l'atelier : allumé ou non, le run en cours, le fil, les orphelines (page 11) |
//! | `POST /ide/chat` | un tour `{backend, message}` : publié sur `agent` et `tokens`, inscrit dans les runs en vol |
//! | `GET /ide/etat` | l'état de l'hôte (`/etat`) |
//! | `POST /ide/call` | `{manifest, name, arguments}`, comme `/call`, dans la session de l'IDE |
//! | `POST /ide/describe` | `{manifest}`, comme `/describe` |
//! | `GET /ide/flux?sujets=…&depuis=…` | le flux SSE : le bus des backends ouverts, chaque événement numéroté |
//!
//! **Le jeton** : une fenêtre ouverte par l'URL d'entrée reçoit le cookie, et
//! toutes les fenêtres du navigateur l'ont ensuite — rien n'est gardé par
//! onglet, et le jeton ne traîne pas dans l'URL des vues. Un appelant qui
//! porte `Authorization: Bearer` passe aussi. Le cookie est `SameSite=Lax`, pas
//! `Strict` : l'URL d'entrée se clique depuis une autre application (un
//! terminal, un chat), et un navigateur ne renvoie pas un cookie `Strict` sur la
//! redirection d'une navigation venue d'ailleurs — `/ide/` répondait 403 au
//! premier clic, et F5 arrangeait tout (Lucie, 25 octobre 2026). `Lax` suit une
//! navigation GET de premier niveau et refuse autant un POST étranger.
//!
//! **Le flux** : chaque backend ouvert par l'hôte voit ses sujets relayés
//! ([`SUJETS`]). Un événement porte un `id:` croissant ; une fenêtre qui se
//! reconnecte avec `Last-Event-ID` (ou `?depuis=`) reçoit ce qu'elle a
//! manqué tant que le tampon le garde, et une `lacune` dite sinon.

use std::collections::{HashSet, VecDeque};
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};

use serde_json::{json, Value};

use crate::daemon_host::DaemonHost;
use crate::events::EventBus;

/// Le préfixe des routes de l'IDE.
pub const PREFIX: &str = "/ide";
/// Le cookie qui porte le jeton local.
pub const COOKIE: &str = "rag3weaver_ide";
/// Les sujets du bus que le flux relaie.
pub const SUJETS: [&str; 8] = ["files", "dataflow", "agent", "tokens", "catalog", "components", "reactions", "declarations"];
/// Combien d'événements le flux garde pour rejouer.
const TAMPON: usize = 2048;
/// **What a window may owe before it counts as gone** (ticket
/// `docs/recherche/tickets/2026-11-14-le-temoin-du-chat-d-une-tache-rougit-sous-charge.md`): the frames queued for its
/// writer, a whole replay of the buffer included. Beyond, the window stopped reading: it is dropped, and comes back
/// with its `Last-Event-ID`, replayed from the buffer.
const EN_ATTENTE_PAR_FENETRE: usize = TAMPON + 256;
/// Le pas du battement qui fait tomber une fenêtre fermée.
const BATTEMENT: std::time::Duration = std::time::Duration::from_secs(15);

const PAGE: &str = include_str!("ide/page.html");
/// Le code de la page, en fichier (`GET /ide/page.js`) : lisible par un serveur
/// de langage et par l'index de code, comme le reste du code.
const PAGE_JS: &str = include_str!("ide/page.js");
/// **Les thèmes livrés** (`templates/themes/<nom>.json`, communs à l'IDE et au terminal :
/// `tty/src/theme.rs` embarque les mêmes fichiers), par leur nom, dans l'ordre des noms.
/// Ils viennent des gabarits embarqués : un fichier posé dans le dossier est servi sans
/// rien changer ici.
fn noms_des_themes() -> Vec<&'static str> {
    crate::builtin_templates::names_under("themes/")
        .filter_map(|n| n.strip_prefix("themes/")?.strip_suffix(".json"))
        .filter(|n| !n.is_empty() && !n.contains('/'))
        .collect()
}

/// Le fichier du thème livré `nom`, s'il en est un.
fn theme_livre(nom: &str) -> Option<&'static [u8]> {
    noms_des_themes().contains(&nom).then(|| crate::builtin_templates::get(&format!("themes/{nom}.json"))).flatten()
}

/// Un chemin de l'IDE ?
pub fn est_a_lui(path: &str) -> bool {
    path == PREFIX || path.starts_with("/ide/")
}

// ─── Le flux ────────────────────────────────────────────────────────────────

/// Un événement relayé, numéroté.
struct Entree {
    id: u64,
    sujet: String,
    trame: String,
}

/// Une fenêtre abonnée : ses sujets (tous si `None`), le dernier id envoyé, et si c'est une
/// fenêtre du navigateur (`GET /ide/flux`) ou un lecteur de l'hôte lui-même (la livraison
/// Codex, les témoins).
struct Abonne {
    sujets: Option<HashSet<String>>,
    dernier: u64,
    /// The frames for this window's own writer thread: never a write under the subscribers' lock, which a window
    /// that stopped reading would hold for ever (ticket of the 14th).
    envoi: std::sync::mpsc::SyncSender<Arc<str>>,
    /// False once its writer failed: the window is gone.
    vivant: Arc<AtomicBool>,
    navigateur: bool,
}

impl Abonne {
    fn veut(&self, sujet: &str) -> bool {
        self.sujets.as_ref().is_none_or(|s| s.contains(sujet))
    }

    /// Écrit une entrée qu'il n'a pas encore reçue ; faux si la fenêtre est partie.
    fn envoyer(&mut self, e: &Entree) -> bool {
        if e.id <= self.dernier || !self.veut(&e.sujet) {
            return true;
        }
        self.dernier = e.id;
        self.pousser(e.trame.as_str())
    }

    /// Queue a frame for the window's writer, never waiting: a full queue (the window stopped reading) or a writer
    /// gone (the window left) drops the window.
    fn pousser(&self, trame: &str) -> bool {
        self.vivant.load(Ordering::SeqCst) && self.envoi.try_send(Arc::from(trame)).is_ok()
    }
}

/// **Le flux de l'IDE** : le bus des backends, numéroté, rejouable.
pub struct Flux {
    prochain: AtomicU64,
    tampon: Mutex<VecDeque<Arc<Entree>>>,
    abonnes: Mutex<Vec<Abonne>>,
    /// Told of each event published, by subject and backend (the host's state keeps what no event changed).
    ecouteur: std::sync::OnceLock<Box<dyn Fn(&str, &str) + Send + Sync>>,
}

impl Flux {
    /// Un flux vide, et son battement (un fil qui s'arrête avec lui).
    pub fn new() -> Arc<Self> {
        let flux = Arc::new(Self { prochain: AtomicU64::new(1), tampon: Mutex::new(VecDeque::new()), abonnes: Mutex::new(Vec::new()), ecouteur: std::sync::OnceLock::new() });
        let faible: Weak<Self> = Arc::downgrade(&flux);
        let _ = std::thread::Builder::new().name("ide-battement".into()).spawn(move || loop {
            std::thread::sleep(BATTEMENT);
            let Some(flux) = faible.upgrade() else { break };
            flux.battre();
        });
        flux
    }

    /// **Publie un état** (l'IDE par le flux, 9 novembre 2026) : `{kind: "Etat", de, cle,
    /// etat}` sous le sujet `etat`. Celui qui change un état le pousse ; la page le garde
    /// (`onEtat` dans `lib/host.ts`) au lieu de le relire à l'hôte après un événement.
    pub fn publier_etat(&self, de: &str, cle: &str, etat: Value) {
        self.publier("etat", cle, serde_json::json!({ "kind": "Etat", "de": de, "cle": cle, "etat": etat }));
    }

    /// **Who is told of each event published** (23 November 2026): set once; the host's state forgets what an event
    /// says changed.
    pub fn ecouter(&self, f: impl Fn(&str, &str) + Send + Sync + 'static) {
        let _ = self.ecouteur.set(Box::new(f));
    }

    /// Publie `evenement` (déjà en JSON) sous `sujet`, pour le backend `backend`.
    pub fn publier(&self, sujet: &str, backend: &str, evenement: Value) {
        if let Some(f) = self.ecouteur.get() {
            f(sujet, backend);
        }
        // **L'id se tire sous le verrou des abonnés**, avec l'empilement et
        // l'envoi (ticket du premier jeton perdu, 7 novembre 2026) : tiré
        // dehors, deux relais pouvaient envoyer N+1 avant N, et `envoyer`, qui
        // saute ce qui n'est pas plus neuf que le dernier envoyé, perdait N en
        // silence. L'ordre des verrous reste celui d'`abonner` : abonnés, puis
        // tampon.
        let mut abonnes = self.abonnes.lock().unwrap_or_else(|p| p.into_inner());
        let id = self.prochain.fetch_add(1, Ordering::SeqCst);
        // Un message ordinaire, le genre dans les données : une page n'a pas à
        // écouter chaque genre par son nom pour ne rien perdre.
        // L'enveloppe commune des événements relayés : l'id du flux, le sujet,
        // le backend, l'instant du relais (epoch ms UTC) ; l'événement garde
        // sa forme (`kind`, `run` quand il en a un, ses champs à plat).
        let at_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
        let data = json!({ "id": id, "sujet": sujet, "backend": backend, "at_ms": at_ms, "evenement": evenement });
        let entree = Arc::new(Entree { id, sujet: sujet.to_string(), trame: format!("id: {id}\ndata: {data}\n\n") });
        {
            let mut t = self.tampon.lock().unwrap_or_else(|p| p.into_inner());
            t.push_back(entree.clone());
            while t.len() > TAMPON {
                t.pop_front();
            }
        }
        abonnes.retain_mut(|a| a.envoyer(&entree));
    }

    /// **Abonne une fenêtre** : l'en-tête SSE, puis ce qu'elle a manqué depuis
    /// `depuis` (une `lacune` dite si le tampon ne le garde plus), puis la suite.
    pub fn abonner(&self, ecrivain: Box<dyn Write + Send>, sujets: Option<HashSet<String>>, depuis: Option<u64>) {
        self.abonner_avec(ecrivain, sujets, depuis, false);
    }

    /// [`Self::abonner`] pour une fenêtre du navigateur (`GET /ide/flux`) : celles que
    /// [`Self::fenetres`] compte.
    pub fn abonner_une_fenetre(&self, ecrivain: Box<dyn Write + Send>, sujets: Option<HashSet<String>>, depuis: Option<u64>) {
        self.abonner_avec(ecrivain, sujets, depuis, true);
    }

    fn abonner_avec(&self, mut ecrivain: Box<dyn Write + Send>, sujets: Option<HashSet<String>>, depuis: Option<u64>, navigateur: bool) {
        // Sous le verrou des abonnés : une publication qui tombe pendant le
        // rejeu attend, puis passe ; l'abonné saute ce qu'il a déjà eu.
        // **Its own writer** (ticket of the 14th): the header, then each frame queued for it, written outside the
        // subscribers' lock. It ends when the window leaves (a write fails) or is dropped (its queue closes).
        let (envoi, recu) = std::sync::mpsc::sync_channel::<Arc<str>>(EN_ATTENTE_PAR_FENETRE);
        let vivant = Arc::new(AtomicBool::new(true));
        let du_fil = vivant.clone();
        let lance = std::thread::Builder::new().name("ide-fenetre".into()).spawn(move || {
            let entete = b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\n\
                Connection: keep-alive\r\n\r\n: ide\n\n";
            if ecrivain.write_all(entete).and_then(|_| ecrivain.flush()).is_err() {
                du_fil.store(false, Ordering::SeqCst);
                return;
            }
            for trame in recu {
                if ecrivain.write_all(trame.as_bytes()).and_then(|_| ecrivain.flush()).is_err() {
                    du_fil.store(false, Ordering::SeqCst);
                    return;
                }
            }
        });
        if lance.is_err() {
            return;
        }
        let mut abonnes = self.abonnes.lock().unwrap_or_else(|p| p.into_inner());
        let mut abonne = Abonne { sujets, dernier: 0, envoi, vivant, navigateur };
        if let Some(depuis) = depuis {
            abonne.dernier = depuis;
            let tampon: Vec<Arc<Entree>> = self.tampon.lock().unwrap_or_else(|p| p.into_inner()).iter().cloned().collect();
            let premier = tampon.first().map(|e| e.id).unwrap_or_else(|| self.prochain.load(Ordering::SeqCst));
            if depuis + 1 < premier {
                let trame = format!(
                    "event: lacune\ndata: {}\n\n",
                    json!({ "depuis": depuis, "premier": premier, "raison": "le tampon du flux ne garde plus ces événements : relire l'état" })
                );
                if !abonne.pousser(&trame) {
                    return;
                }
            }
            for e in tampon.iter().filter(|e| e.id > depuis) {
                if !abonne.envoyer(e) {
                    return;
                }
            }
        } else {
            // Une fenêtre neuve commence à maintenant.
            abonne.dernier = self.prochain.load(Ordering::SeqCst).saturating_sub(1);
        }
        abonnes.push(abonne);
    }

    /// Un commentaire SSE à chaque fenêtre : celles qui sont parties tombent.
    fn battre(&self) {
        self.abonnes
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .retain_mut(|a| a.pousser(": battement\n\n"));
    }

    /// Combien de fenêtres du navigateur écoutent (une par navigateur quand ses onglets
    /// partagent le flux, une par page sinon) ; les lecteurs de l'hôte n'y comptent pas.
    pub fn fenetres(&self) -> usize {
        // Only the living ones: a window whose writer failed (it left) no longer listens, even before the next
        // frame drops it from the list.
        self.abonnes.lock().unwrap_or_else(|p| p.into_inner()).iter().filter(|a| a.navigateur && a.vivant.load(Ordering::SeqCst)).count()
    }

    /// **Relaie le bus d'un backend** qui vient de s'ouvrir : un fil par
    /// sujet, qui finit quand le bus se ferme (le backend fermé ou rouvert,
    /// dont le nouveau bus est relayé à son tour).
    ///
    /// **Un fil à lui, pas une tâche du runtime des graphes** (25 octobre
    /// 2026) : ce runtime a deux fils pour tout le processus, et les nœuds qui
    /// bloquent les y tiennent. Le témoin du chat dans la page rougissait sous
    /// la lib entière : le tour publiait, le relais attendait derrière les
    /// graphes des autres témoins, la page ne voyait rien en 20 s. En service,
    /// un graphe long gelait de même la vue vivante de l'IDE.
    pub fn relayer(self: &Arc<Self>, backend: &str, bus: &EventBus) {
        for sujet in SUJETS {
            // `agent` and `tokens` share one thread, which keeps a run's words between its start and its end.
            if sujet == "agent" || sujet == "tokens" {
                continue;
            }
            let mut rx = bus.subscribe(sujet);
            let (flux, backend, du_fil) = (Arc::downgrade(self), backend.to_string(), sujet.to_string());
            let lance = std::thread::Builder::new().name(format!("ide-relais-{sujet}")).spawn(move || loop {
                let evenement = match rx.recv_blocking() {
                    Ok(e) => e.to_json(),
                    Err(async_broadcast::RecvError::Overflowed(n)) => json!({ "kind": "EventsMissed", "count": n }),
                    Err(async_broadcast::RecvError::Closed) => break,
                };
                let Some(flux) = flux.upgrade() else { break };
                flux.publier(&du_fil, &backend, evenement);
            });
            if let Err(e) = lance {
                eprintln!("[ide] le relais de « {sujet} » ne part pas : {e}");
            }
        }
        self.relayer_les_runs(backend, bus);
    }

    /// **`agent` and `tokens` by one thread, in a run's order** (an end-to-end witness on 23 November 2026: by two
    /// threads, 132 to 162 of 300 words of a step passed after the `RunFinished` of their run; the ticket of
    /// 7 November had seen a `Token` pass its `RunStarted`). The words are emitted before their run's end: the
    /// relay empties `tokens` before it publishes a `RunFinished`. A run's start is emitted before its first word:
    /// on the first word of a run it has not seen start, the relay publishes the `agent` events waiting, up to that
    /// run's `RunStarted`; an end met on the way waits behind the words. Both channels are awaited at once, `tokens`
    /// first (`select!` biased), on a current-thread runtime of its own (not the graphs' runtime, see above).
    fn relayer_les_runs(self: &Arc<Self>, backend: &str, bus: &EventBus) {
        let (mut agent, mut tokens) = (bus.subscribe("agent"), bus.subscribe("tokens"));
        let (flux, backend) = (Arc::downgrade(self), backend.to_string());
        let lance = std::thread::Builder::new().name("ide-relais-runs".into()).spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().build() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("[ide] le relais des runs ne part pas : {e}");
                    return;
                }
            };
            rt.block_on(async move {
                let publier = |sujet: &str, e: Value| -> bool {
                    match flux.upgrade() {
                        Some(f) => {
                            f.publier(sujet, &backend, e);
                            true
                        }
                        None => false,
                    }
                };
                let manque = |n: u64| json!({ "kind": "EventsMissed", "count": n });
                let mut commences: HashSet<String> = HashSet::new();
                let mut retenues: VecDeque<Value> = VecDeque::new();
                let (mut agent_ouvert, mut tokens_ouvert) = (true, true);
                loop {
                    // An end held back waits behind the words already emitted.
                    if !retenues.is_empty() {
                        while let Ok(t) = tokens.try_recv() {
                            if !publier("tokens", t.to_json()) {
                                return;
                            }
                        }
                        while let Some(v) = retenues.pop_front() {
                            if !publier("agent", v) {
                                return;
                            }
                        }
                    }
                    if !agent_ouvert && !tokens_ouvert {
                        break;
                    }
                    tokio::select! {
                        biased;
                        r = tokens.recv(), if tokens_ouvert => match r {
                            Ok(e) => {
                                let v = e.to_json();
                                let run = v["run"].as_str().unwrap_or_default().to_string();
                                if !run.is_empty() && !commences.contains(&run) {
                                    // Its start was emitted before it: publish the agent events waiting, up to it.
                                    while let Ok(a) = agent.try_recv() {
                                        let av = a.to_json();
                                        match av["kind"].as_str() {
                                            Some("RunFinished") => retenues.push_back(av),
                                            Some("RunStarted") => {
                                                let lui = av["run"] == run.as_str();
                                                if let Some(r) = av["run"].as_str() {
                                                    commences.insert(r.to_string());
                                                }
                                                if !publier("agent", av) {
                                                    return;
                                                }
                                                if lui {
                                                    break;
                                                }
                                            }
                                            _ => {
                                                if !publier("agent", av) {
                                                    return;
                                                }
                                            }
                                        }
                                    }
                                    commences.insert(run);
                                }
                                if !publier("tokens", v) {
                                    return;
                                }
                            }
                            Err(async_broadcast::RecvError::Overflowed(n)) => {
                                if !publier("tokens", manque(n)) {
                                    return;
                                }
                            }
                            Err(async_broadcast::RecvError::Closed) => tokens_ouvert = false,
                        },
                        r = agent.recv(), if agent_ouvert => match r {
                            Ok(a) => {
                                let v = a.to_json();
                                match v["kind"].as_str() {
                                    Some("RunStarted") => {
                                        if let Some(r) = v["run"].as_str() {
                                            commences.insert(r.to_string());
                                        }
                                        if !publier("agent", v) {
                                            return;
                                        }
                                    }
                                    // Its words were emitted before it: they pass first.
                                    Some("RunFinished") => {
                                        if let Some(r) = v["run"].as_str() {
                                            commences.remove(r);
                                        }
                                        retenues.push_back(v);
                                    }
                                    _ => {
                                        if !publier("agent", v) {
                                            return;
                                        }
                                    }
                                }
                            }
                            Err(async_broadcast::RecvError::Overflowed(n)) => {
                                if !publier("agent", manque(n)) {
                                    return;
                                }
                            }
                            Err(async_broadcast::RecvError::Closed) => agent_ouvert = false,
                        },
                    }
                }
            });
        });
        if let Err(e) = lance {
            eprintln!("[ide] le relais des runs ne part pas : {e}");
        }
    }
}

// ─── L'aperçu d'un composant (marche 3, lot 3) ──────────────────────────────

const APERCU: &str = include_str!("ide/apercu.html");

/// `/ide/apercu/<backend>/<Composant>` : la page ; `…/modules.js` : ses
/// modules, enveloppés, puis l'entrée et le démarrage pour le registre.
/// `relecture` : la page relit ses modules (rechargement à chaud, `?v=`), et non son
/// premier chargement.
fn apercu(host: &DaemonHost, path: &str, proposition: Option<u64>, relecture: bool) -> (u16, &'static str, String) {
    let reste = path.trim_start_matches("/ide/apercu/");
    let morceaux: Vec<&str> = reste.split('/').collect();
    match morceaux.as_slice() {
        // La page monte la montre des composants : ce qu'on regarde se suit.
        // Une montre qui ne se monte pas n'empêche pas l'aperçu ; la page le
        // dit à sa console et se passe du rechargement à chaud.
        [id, nom] if !nom.is_empty() => {
            let montre = host.with_backend_id(id, |b| b.watch_components()).and_then(|r| r);
            let page = match montre {
                Ok(()) => APERCU.to_string(),
                Err(e) => APERCU.replace(
                    "<script src=\"/ide/registre.js\"></script>",
                    &format!("<script>console.info({});</script>\n<script src=\"/ide/registre.js\"></script>", serde_json::to_string(&format!("sans rechargement à chaud : {e}")).unwrap().replace("</", "<\\/")),
                ),
            };
            (200, "text/html; charset=utf-8", page)
        }
        // Les déclarations relues sur le disque : une déclaration éditée
        // depuis l'ouverture du backend se voit au graphe suivant.
        [id, nom, "modules.js"] => {
            let nom = decoder(nom);
            // Sur une proposition (`?proposition=<n>`) : le composant tel qu'elle
            // le propose, lu sur son recouvrement, sans l'appliquer.
            let rendu = host.with_backend_id(id, |b| {
                let prepared = b.prepared();
                let graphe = match proposition {
                    None => prepared.components_on_disk()?.module_graph(&prepared.directory, &nom, crate::components::Mode::Development)?,
                    Some(pid) => {
                        let r = b.recouvrement_de_proposition(pid)?;
                        prepared.components_on_disk_avec(&r)?.module_graph_avec(&prepared.directory, &nom, crate::components::Mode::Development, &r)?
                    }
                };
                // Un graphe de clic blanc se relit (l'instant d'une écriture en place,
                // la page attend la suite) ; au premier chargement, il n'y a rien à
                // rendre : le dire (relecture d'everything declarative).
                let clic_blanc = prepared.components_on_disk()?.components.get(&nom).and_then(|c| c.clic.clone()).filter(|_| graphe.modules.get(&format!("comp:{nom}")).is_some_and(|m| m.empty_source));
                if let (Some(clic), false) = (clic_blanc, relecture) {
                    return Err(format!("comp:{nom} : graphe de clic vide (components/{clic}) : rien à rendre tant qu'il n'est pas écrit"));
                }
                Ok::<_, String>(modules_js(&graphe))
            });
            match rendu.and_then(|r| r) {
                Ok(js) => (200, "text/javascript; charset=utf-8", js),
                Err(e) => (422, "text/plain; charset=utf-8", e),
            }
        }
        _ => (404, "text/plain; charset=utf-8", format!("aperçu : {path} n'est ni /ide/apercu/<backend>/<Composant> ni …/modules.js")),
    }
}

// ─── L'éditeur de la page (B1, partie 2) ─────────────────────────────────────

/// Les libs que la page de l'IDE prend pour son éditeur, au démarrage du graphe.
pub const EDITEUR: &[&str] = &["@codemirror/view", "@codemirror/state", "@codemirror/commands", "@codemirror/search", "@codemirror/language"];

/// **Les modules de l'éditeur** (CodeMirror), par le magasin de libs embarqué
/// dans le binaire (`templates/ide/`, rangé par `scripts/ide_libs.sh`, extrait par
/// `crate::builtin_templates`) et le graphe de modules de l'aperçu. Rendus une
/// fois par processus : le magasin embarqué ne change pas. Une lib que le
/// graphe ne résout pas est une erreur ici, pas un avertissement dans la page.
pub fn editeur_modules() -> &'static Result<String, String> {
    static M: std::sync::OnceLock<Result<String, String>> = std::sync::OnceLock::new();
    M.get_or_init(|| {
        let dir = crate::builtin_templates::root()?.join("ide");
        let libs = crate::components::Components::load(&dir).map_err(|e| format!("les libs de l'éditeur : {}", e.join(" ; ")))?;
        let g = libs.module_graph_of(&dir, EDITEUR, crate::components::Mode::Production)?;
        if !g.unresolved.is_empty() {
            return Err(format!("les libs de l'éditeur ne se résolvent pas : {}", g.unresolved.join(" ; ")));
        }
        Ok(modules_js(&g))
    })
}

/// **Les réactions d'un backend**, et ce que ses montres en disent : chaque
/// réaction déclarée (son nom de manifeste, sa description, ses sujets, sa
/// politique), le nombre de runs et les erreurs du réacteur, et l'état de
/// l'index (`index_states` : les montres, le retard des fichiers).
pub fn reactions_du_backend(b: &crate::backend::Backend) -> Value {
    let prepared = b.prepared();
    let reactions: Vec<Value> = prepared
        .reaction_graphs()
        .iter()
        .map(|(nom, t)| {
            let description = prepared
                .manifest
                .reactions
                .get(nom)
                .and_then(|a| a.description.clone())
                .unwrap_or_else(|| t.description().to_string());
            json!({ "name": nom, "description": description, "on": t.on(), "policy": format!("{:?}", t.policy()) })
        })
        .collect();
    json!({
        "reactions": reactions,
        "runs": b.reaction_runs(),
        "errors": b.reaction_errors(),
        "index": b.index_states().unwrap_or_else(|e| json!({ "error": e })),
    })
}

/// Le script des modules d'un graphe, puis son entrée, son démarrage et ce qui
/// ne s'est pas résolu (dit à la console, refusé par le registre s'il s'exécute).
pub fn modules_js(g: &crate::components::modules::Graph) -> String {
    let mut js = g.script();
    js.push_str(&format!(
        "__rag3weaver.entree = {};
__rag3weaver.demarrage = {};
__rag3weaver.frontieres = {};
__rag3weaver.vides = {};
",
        serde_json::to_string(&g.entry).unwrap(),
        serde_json::to_string(&g.boot).unwrap(),
        serde_json::to_string(&g.modules.values().filter(|m| m.exports_only_components).map(|m| m.id.as_str()).collect::<Vec<_>>()).unwrap(),
        serde_json::to_string(&g.modules.values().filter(|m| m.empty_source).map(|m| m.id.as_str()).collect::<Vec<_>>()).unwrap()
    ));
    for u in &g.unresolved {
        js.push_str(&format!("console.warn({});
", serde_json::to_string(u).unwrap()));
    }
    js
}

// ─── Les routes ─────────────────────────────────────────────────────────────

/// Les paramètres d'une URL, décodés.
fn parametres(url: &str) -> Vec<(String, String)> {
    let Some((_, q)) = url.split_once('?') else { return Vec::new() };
    q.split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (decoder(k), decoder(v))
        })
        .collect()
}

fn decoder(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                match u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16) {
                    Ok(v) => {
                        out.push(v);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn entete<'a>(request: &'a tiny_http::Request, nom: &'static str) -> Option<&'a str> {
    request.headers().iter().find(|h| h.field.equiv(nom)).map(|h| h.value.as_str())
}

/// Le jeton porté par le cookie de l'IDE, s'il y en a un.
/// The user cookie (the title of a human), set by the entry with a one-use code.
const COOKIE_USER: &str = "rag3weaver_user";
/// **The user's title lasts in her browser** (c7, 22 Nov 2026: entering as a human is one gesture): a year.
const TITRE_DURE_S: u64 = 365 * 24 * 3600;

/// A cookie of the request by its name.
fn cookie(request: &tiny_http::Request, nom: &str) -> Option<String> {
    entete(request, "Cookie")?.split(';').filter_map(|c| c.trim().split_once('=')).find(|(k, _)| *k == nom).map(|(_, v)| v.to_string())
}

fn jeton_du_cookie(request: &tiny_http::Request) -> Option<String> {
    entete(request, "Cookie")?
        .split(';')
        .filter_map(|c| c.trim().split_once('='))
        .find(|(k, _)| *k == COOKIE)
        .map(|(_, v)| v.to_string())
}

/// **L'outil que la vue recherche appelle** (`Search.tsx`) : une ligne de « mes
/// recherches » faite par lui est `relancable` depuis la vue, que la vue ne
/// nomme pas d'outil.
pub const SEARCH_VIEW_TOOL: &str = "search_code";

/// **Les vues portées sur la page nouvelle** (page 08) : `/ide/code/<id>/<vue>…`
/// pour ces vues est servi par le bundle ; les autres, par l'ancienne page.
const VUES_PORTEES: &[&str] = &["chat", "fichier", "recherche", "recherches", "taches", "bilans", "arbre", "symbole", "graphe", "dag", "reactions", "propositions"];

fn vue_portee(path: &str) -> bool {
    let Some(reste) = path.strip_prefix("/ide/code/") else { return false };
    let mut parts = reste.split('/').filter(|s| !s.is_empty());
    let _id = parts.next();
    parts.next().is_some_and(|vue| VUES_PORTEES.contains(&vue))
}

/// Un fichier du bundle de l'IDE, embarqué (`templates/ide/app/<nom>`) ; absent,
/// un bâti sans `scripts/ide_bundle.sh` : la réponse le dit.
fn bundle(nom: &str) -> Vec<u8> {
    crate::builtin_templates::get(&format!("ide/app/{nom}"))
        .map(<[u8]>::to_vec)
        .unwrap_or_else(|| format!("/* le bundle de l'IDE n'est pas embarqué : scripts/ide_bundle.sh ({nom}) */").into_bytes())
}

fn reponse_octets(statut: u16, type_: &str, corps: Vec<u8>) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    tiny_http::Response::from_data(corps)
        .with_status_code(statut)
        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], type_.as_bytes()).expect("en-tête littéral"))
        .with_header(tiny_http::Header::from_bytes(&b"Cache-Control"[..], &b"no-cache"[..]).expect("en-tête littéral"))
}

fn reponse(statut: u16, type_: &str, corps: String) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    tiny_http::Response::from_string(corps)
        .with_status_code(statut)
        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], type_.as_bytes()).expect("en-tête littéral"))
}

fn json_(statut: u16, v: &Value) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    reponse(statut, "application/json", v.to_string())
}

/// **Sert une requête de l'IDE.** `attendu` est `Bearer <jeton>` quand l'hôte
/// en exige un ; le cookie de l'IDE, ou l'en-tête, le portent.
pub fn servir(host: &Arc<DaemonHost>, mut request: tiny_http::Request, attendu: Option<&str>) {
    let method = request.method().as_str().to_string();
    let url = request.url().to_string();
    let path = url.split('?').next().unwrap_or("").to_string();
    let params = parametres(&url);
    let param = |k: &str| params.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
    let juste = |jeton: &str| attendu.is_none_or(|a| crate::daemon_host::same_token(&format!("Bearer {jeton}"), a));

    // **The entry by a one-use code** (21 Nov 2026): the code that `rag3weaver-backend ide` asked with
    // the user's token becomes the user cookie (the title of a human), and the URL loses it.
    if method == "GET" && path == "/ide/entrer" && param("code").is_some() {
        let r = match param("code").and_then(|c| host.redeem_entry_code(&c)) {
            Some(u) => {
                let cookie = format!("{COOKIE_USER}={u}; HttpOnly; SameSite=Strict; Path={PREFIX}; Max-Age={TITRE_DURE_S}");
                tiny_http::Response::from_string(String::new())
                    .with_status_code(303)
                    .with_header(tiny_http::Header::from_bytes(&b"Location"[..], &b"/ide/"[..]).expect("en-tête littéral"))
                    .with_header(tiny_http::Header::from_bytes(&b"Set-Cookie"[..], cookie.as_bytes()).expect("cookie ASCII"))
            }
            None => json_(403, &json!({ "ok": false, crate::daemon_host::BAD_TOKEN: true, "error": "entry code unknown, used or expired: ask a new URL of `rag3weaver-backend ide`" })),
        };
        let _ = request.respond(r);
        return;
    }
    // L'entrée : le jeton de l'URL devient le cookie, et l'URL le perd.
    if method == "GET" && path == "/ide/entrer" {
        let jeton = param("jeton").unwrap_or_default();
        let r = if juste(&jeton) {
            let cookie = format!("{COOKIE}={jeton}; HttpOnly; SameSite=Lax; Path={PREFIX}");
            let r = tiny_http::Response::from_string(String::new())
                .with_status_code(303)
                .with_header(tiny_http::Header::from_bytes(&b"Location"[..], &b"/ide/"[..]).expect("en-tête littéral"))
                .with_header(tiny_http::Header::from_bytes(&b"Set-Cookie"[..], cookie.as_bytes()).expect("cookie ASCII"));
            // An entry with the user's own token is hers: her title too (in service it differs from the host's token).
            if host.is_user_token(Some(&jeton)) {
                let humain = format!("{COOKIE_USER}={jeton}; HttpOnly; SameSite=Strict; Path={PREFIX}; Max-Age={TITRE_DURE_S}");
                r.with_header(tiny_http::Header::from_bytes(&b"Set-Cookie"[..], humain.as_bytes()).expect("cookie ASCII"))
            } else {
                r
            }
        } else {
            json_(403, &json!({ "ok": false, crate::daemon_host::BAD_TOKEN: true, "error": "jeton absent ou faux : l'URL d'entrée est donnée par `rag3weaver-backend ide`" }))
        };
        let _ = request.respond(r);
        return;
    }

    let porte = attendu.is_none()
        || entete(&request, "Authorization").is_some_and(|h| attendu.is_some_and(|a| crate::daemon_host::same_token(h, a)))
        || jeton_du_cookie(&request).is_some_and(|j| juste(&j))
        || host.is_user_token(cookie(&request, COOKIE_USER).as_deref());
    if !porte {
        let _ = request.respond(json_(
            403,
            &json!({ "ok": false, crate::daemon_host::BAD_TOKEN: true, "error": "pas de jeton : entrer par l'URL que donne `rag3weaver-backend ide`" }),
        ));
        return;
    }

    // **Who calls** (c7, 22 Nov 2026): the user's title (her cookie, or the terminal's header after a keystroke) makes a
    // human; the host's token alone, which an agent may hold, does not. The routes that grant ask for a human.
    let humain = host.is_user_token(cookie(&request, COOKIE_USER).as_deref()) || host.is_user_token(entete(&request, "X-Rag3weaver-User"));
    let refus_humain = || json!({ "ok": false, "error": "réservé à une réponse humaine : ouvre l'IDE depuis ta session (ton lanceur), ou réponds dans le terminal" });
    if !humain && matches!(path.as_str(), "/ide/propositions/appliquer" | "/ide/taches/fusionner") && method == "POST" {
        let _ = request.respond(json_(403, &refus_humain()));
        return;
    }
    match (method.as_str(), path.as_str()) {
        ("GET", "/ide/flux") => {
            let sujets = param("sujets").map(|s| s.split(',').filter(|x| !x.is_empty()).map(str::to_string).collect::<HashSet<_>>());
            let depuis = entete(&request, "Last-Event-ID").map(str::to_string).or_else(|| param("depuis")).and_then(|v| v.trim().parse::<u64>().ok());
            let ecrivain = request.into_writer();
            host.ide_flux().abonner_une_fenetre(ecrivain, sujets, depuis);
        }
        // **La page nouvelle** (page 08 du 25 octobre 2026) : le bundle React commité
        // sous templates/ide/app, embarqué avec les gabarits et servi d'ici — jamais
        // d'un CDN. L'accueil est porté ; une vue qui ne l'est pas encore est servie
        // par l'ancienne page, et `/ide/ancienne/…` la donne toujours.
        ("GET", "/ide/app.js") => {
            let _ = request.respond(reponse_octets(200, "text/javascript; charset=utf-8", bundle("app.js")));
        }
        // **Le flux partagé** (13 novembre 2026) : le `SharedWorker` qui tient un seul flux
        // pour tous les onglets d'un navigateur (`ui/ide/lib/flux-worker.ts`).
        ("GET", "/ide/flux-worker.js") => {
            let _ = request.respond(reponse_octets(200, "text/javascript; charset=utf-8", bundle("flux-worker.js")));
        }
        ("GET", "/ide/app.css") => {
            let _ = request.respond(reponse_octets(200, "text/css; charset=utf-8", bundle("app.css")));
        }
        // **Les thèmes communs** (Lucie, 7 et 8 novembre 2026 ; forêt et les suivants le
        // 13) : la liste des thèmes livrés, puis chaque fichier que le terminal embarque,
        // servi tel quel ; la page en tire ses couleurs et son animation. Un nom inconnu, 404.
        ("GET", "/ide/themes.json") => {
            let _ = request.respond(reponse(200, "application/json; charset=utf-8", json!(noms_des_themes()).to_string()));
        }
        ("GET", p) if p.starts_with("/ide/theme/") => {
            let nom = p.strip_prefix("/ide/theme/").and_then(|r| r.strip_suffix(".json")).unwrap_or_default();
            let _ = request.respond(match theme_livre(nom) {
                Some(octets) => reponse_octets(200, "application/json; charset=utf-8", octets.to_vec()),
                None => reponse(404, "text/plain; charset=utf-8", format!("thème inconnu : {p} (les thèmes : /ide/themes.json)")),
            });
        }
        ("GET", "/ide") | ("GET", "/ide/") => {
            let _ = request.respond(reponse_octets(200, "text/html; charset=utf-8", bundle("index.html")));
        }
        ("GET", p) if vue_portee(p) => {
            let _ = request.respond(reponse_octets(200, "text/html; charset=utf-8", bundle("index.html")));
        }
        ("GET", "/ide/page.js") => {
            let _ = request.respond(reponse(200, "text/javascript; charset=utf-8", PAGE_JS.to_string()));
        }
        ("GET", "/ide/registre.js") => {
            let _ = request.respond(reponse(200, "text/javascript; charset=utf-8", crate::components::modules::REGISTRE.to_string()));
        }
        ("GET", "/ide/editeur/modules.js") => {
            let (statut, corps) = match editeur_modules() {
                Ok(js) => (200, js.clone()),
                Err(e) => (500, e.clone()),
            };
            let _ = request.respond(reponse(statut, if statut == 200 { "text/javascript; charset=utf-8" } else { "text/plain; charset=utf-8" }, corps));
        }
        ("GET", p) if p.starts_with("/ide/apercu/") => {
            let (statut, type_, corps) = apercu(host, p, param("proposition").and_then(|v| v.parse().ok()), param("v").is_some());
            let _ = request.respond(reponse(statut, type_, corps));
        }
        // Les réactions d'un backend et l'état de ses montres ((b)) : ce que la
        // vue des réactions montre, et que le flux (`reactions`) tient à jour.
        ("GET", "/ide/reactions") => {
            let (statut, v) = match param("backend") {
                None => (400, json!({ "ok": false, "error": "réactions : il faut backend=<id>" })),
                Some(id) => match host.with_backend_id(&id, reactions_du_backend) {
                    Ok(r) => (200, json!({ "ok": true, "result": r })),
                    Err(e) => (404, json!({ "ok": false, "error": e })),
                },
            };
            let _ = request.respond(json_(statut, &v));
        }
        // Le graphe d'un outil, d'une réaction ou d'un composant (C1) : le
        // JSON de l'outil `graph`, pour tout backend ouvert par son id.
        ("GET", "/ide/graphe") => {
            let cible = json!({ "tool": param("tool"), "reaction": param("reaction"), "component": param("component") });
            let (statut, v) = match (param("backend"), crate::backend::GraphTarget::from_args(&cible)) {
                (None, _) => (400, json!({ "ok": false, "error": "graphe : il faut backend=<id>" })),
                (_, Err(e)) => (400, json!({ "ok": false, "error": e })),
                (Some(id), Ok(cible)) => match host
                    .with_backend_id(&id, |b| match param("proposition").and_then(|v| v.parse::<u64>().ok()) {
                        None => b.prepared().graph_json(&cible),
                        // Le graphe tel qu'une proposition le propose : le chargement
                        // sur son recouvrement.
                        Some(pid) => {
                            let r = b.recouvrement_de_proposition(pid)?;
                            let apres = b.prepared().recharger(&r)?;
                            apres.graph_json_avec(&cible, &r)
                        }
                    })
                    .and_then(|r| r)
                {
                    Ok(g) => (200, json!({ "ok": true, "result": g })),
                    // **A tool the host serves itself has no declared graph** (the IDE in several windows, slice 4: a
                    // detached window of the graph follows every tool of a turn, propose and report included). It is
                    // a fact about the tool, said by `sans_graphe`, not a missing resource: a 404 there was a console
                    // error in every window that followed such a tool.
                    Err(e) if e.starts_with(crate::backend::GRAPH_PAS_D_OUTIL) => (200, json!({ "ok": false, "error": e, "sans_graphe": true })),
                    Err(e) => (404, json!({ "ok": false, "error": e })),
                },
            };
            let _ = request.respond(json_(statut, &v));
        }
        // Les propositions (page 08, P3) : la liste, le détail (diffs et
        // graphes avant/après), proposer, appliquer, rejeter. La page de
        // l'IDE est une personne : elle applique en mode approbation.
        ("GET", "/ide/propositions") => {
            let (statut, v) = match param("backend") {
                None => (400, json!({ "ok": false, "error": "propositions : il faut backend=<id>" })),
                Some(id) => {
                    let r = host.with_backend_id(&id, |b| match param("id").and_then(|v| v.parse::<u64>().ok()) {
                        None => Ok(json!(b.propositions().iter().map(crate::propositions::resume).collect::<Vec<_>>())),
                        Some(pid) => b.detail_de_proposition(pid),
                    });
                    match r.and_then(|r| r) {
                        Ok(v) => (200, json!({ "ok": true, "result": v })),
                        Err(e) => (404, json!({ "ok": false, "error": e })),
                    }
                }
            };
            let _ = request.respond(json_(statut, &v));
        }
        ("POST", "/ide/propositions" | "/ide/propositions/appliquer" | "/ide/propositions/rejeter") => {
            let mut corps = Vec::new();
            let (statut, v) = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => (400, json!({ "ok": false, "error": format!("corps illisible : {e}") })),
                Ok(c) => {
                    let id = c["backend"].as_str().unwrap_or_default().to_string();
                    let pid = c["id"].as_u64().unwrap_or(0);
                    let r = host.with_backend_id(&id, |b| match path.as_str() {
                        "/ide/propositions" => {
                            let files: Vec<(String, String)> = c["files"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .map(|f| (f["path"].as_str().unwrap_or_default().to_string(), f["content"].as_str().unwrap_or_default().to_string()))
                                .collect();
                            b.proposer(&files, c["reason"].as_str().unwrap_or_default(), "ide")
                        }
                        "/ide/propositions/appliquer" => b.appliquer(pid),
                        _ => b.rejeter(pid, c["why"].as_str().unwrap_or_default()),
                    });
                    match r.and_then(|r| r) {
                        Ok(p) => (200, json!({ "ok": true, "result": crate::propositions::resume(&p) })),
                        // Un refus est une réponse (200 ok:false), pas une ressource en échec.
                        Err(e) => (200, json!({ "ok": false, "error": e })),
                    }
                }
            };
            let _ = request.respond(json_(statut, &v));
        }
        // Les tâches de l'agent (page 09) : la liste, la relecture d'une tâche,
        // l'ouvrir, la fusionner, la rejeter. La page est une personne.
        // **The agents of the machine, as a tree** (21 Nov 2026): roots (Claude Code sessions by
        // name), their open tasks, and the tasks without a parent session.
        // **A run as a tree, read-only** (21 Nov 2026): the IDE's agent page, and a reviewer or a
        // judge that checks what a run really did.
        ("GET", "/ide/run") => {
            let (statut, v) = match (param("backend"), param("run")) {
                (Some(b), Some(r)) => match host.run_tree(&b, &r) {
                    Ok(t) => (200, json!({ "ok": true, "result": t })),
                    Err(e) => (404, json!({ "ok": false, "error": e })),
                },
                _ => (400, json!({ "ok": false, "error": "run: backend=<id> and run=<id> are required" })),
            };
            let _ = request.respond(json_(statut, &v));
        }
        ("GET", "/ide/agents") => {
            let _ = request.respond(json_(200, &json!({ "ok": true, "result": host.agents() })));
        }
        ("GET", "/ide/taches") => {
            let (statut, v) = match (param("backend"), param("nom")) {
                (None, _) => (400, json!({ "ok": false, "error": "tâches : il faut backend=<id>" })),
                (Some(id), None) => (200, json!({ "ok": true, "result": { "ouvertes": host.taches_de(&id), "fermees": host.taches_fermees_de(&id) } })),
                (Some(id), Some(nom)) => match host.relecture(&id, &nom) {
                    Ok(v) => (200, json!({ "ok": true, "result": v })),
                    Err(e) => (404, json!({ "ok": false, "error": e })),
                },
            };
            let _ = request.respond(json_(statut, &v));
        }
        ("GET", "/ide/chat") => {
            // **Le chat de l'atelier** (page 11) : allumé ou non, le run en
            // cours, le fil de la session, les sessions orphelines.
            let v = match param("backend") {
                None => json!({ "ok": false, "error": "chat : il faut backend=<id>" }),
                Some(id) => match host.chat_state(&id) {
                    Ok(v) => json!({ "ok": true, "result": v }),
                    Err(e) => json!({ "ok": false, "error": e }),
                },
            };
            let _ = request.respond(json_(200, &v));
        }
        ("POST", "/ide/chat") => {
            // Un tour : il publie sur le bus du backend (`agent`, `tokens`) et
            // rend à sa fin le texte, le run et ce qu'il a consommé. Un refus
            // (éteint, un tour déjà en cours) est une réponse. Qui l'envoie, l'hôte le dit
            // (`by_of`, le cookie ou l'en-tête de l'utilisateur) : le tour d'un humain passe
            // le plafond des dépenses et se compte.
            let utilisateur = entete(&request, "X-Rag3weaver-User").map(str::to_string);
            let client = if utilisateur.is_some() { "tty" } else { "ide" };
            let by = host.by_of(cookie(&request, COOKIE_USER).as_deref(), utilisateur.as_deref(), client);
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("corps illisible : {e}") }),
                Ok(c) => match host.chat_turn_by(c["backend"].as_str().unwrap_or_default(), c["message"].as_str().unwrap_or_default(), &by) {
                    Ok(v) => json!({ "ok": true, "result": v }),
                    // A model without a price: the refusal names it, and the page offers the gesture.
                    Err(e) => json!({ "ok": false, "missing_price": crate::usage_ledger::unpriced(&e), "error": e }),
                },
            };
            let _ = request.respond(json_(200, &v));
        }
        ("POST", "/ide/turns/read") => {
            // **The turns of a run** (`{run, from, to}`; first form: the chat file).
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("unreadable body: {e}") }),
                Ok(c) if c["apport"].is_string() => json!({ "ok": false, "error": "turns/read by apport comes with the memory's trace_turn; give {run, from, to}" }),
                Ok(c) => match host.lire_les_tours(c["run"].as_str().unwrap_or_default(), c["from"].as_u64().unwrap_or(1) as usize, c["to"].as_u64().unwrap_or(1) as usize) {
                    Ok(v) => json!({ "ok": true, "result": v }),
                    Err(e) => json!({ "ok": false, "error": e }),
                },
            };
            let _ = request.respond(json_(200, &v));
        }
        ("GET", "/ide/l1") => {
            // **The L1 of a run, in the order of its turns** (`backend`, `run`).
            let v = match (param("backend"), param("run")) {
                (Some(b), Some(r)) => match host.l1_du_run(&b, &r) {
                    Ok(v) => json!({ "ok": true, "result": v }),
                    Err(e) => json!({ "ok": false, "error": e }),
                },
                _ => json!({ "ok": false, "error": "l1: backend=<id>&run=<run>" }),
            };
            let _ = request.respond(json_(200, &v));
        }
        ("POST", "/ide/prices") => {
            // **A price, on the user's yes** (`{slug, input, output, …}`): written by the host only when the call
            // carries the user's title (`by_of`: the cookie or the header, never the body).
            let utilisateur = entete(&request, "X-Rag3weaver-User").map(str::to_string);
            let client = if utilisateur.is_some() { "tty" } else { "ide" };
            let by = host.by_of(cookie(&request, COOKIE_USER).as_deref(), utilisateur.as_deref(), client);
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("unreadable body: {e}") }),
                Ok(c) => match host.set_price(&by, &c) {
                    Ok(v) => json!({ "ok": true, "result": v }),
                    Err(e) => json!({ "ok": false, "error": e }),
                },
            };
            let _ = request.respond(json_(200, &v));
        }
        ("POST", "/ide/prices/cap") | ("POST", "/ide/prices/set-aside") => {
            // **The cap, or an unreadable prices file set aside, on the user's yes** (« À intervenir », 28 November
            // 2026): the host acts only when the call carries the user's title, as for a price.
            let utilisateur = entete(&request, "X-Rag3weaver-User").map(str::to_string);
            let client = if utilisateur.is_some() { "tty" } else { "ide" };
            let by = host.by_of(cookie(&request, COOKIE_USER).as_deref(), utilisateur.as_deref(), client);
            let aside = request.url().split('?').next() == Some("/ide/prices/set-aside");
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(if corps.is_empty() { b"{}" } else { &corps }).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("unreadable body: {e}") }),
                Ok(c) => match if aside { host.set_aside_prices(&by) } else { host.set_cap(&by, &c) } {
                    Ok(v) => json!({ "ok": true, "result": v }),
                    Err(e) => json!({ "ok": false, "error": e }),
                },
            };
            let _ = request.respond(json_(200, &v));
        }
        ("POST", "/ide/prices/lookup") => {
            // **A price looked up** (`{slug}`): a proposal with its sources, never written; the page asks the yes.
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("unreadable body: {e}") }),
                Ok(c) => match crate::daemon_host::prices::lookup_price(&crate::backend::WebSearchPolicy::default(), c["slug"].as_str().unwrap_or_default()) {
                    Ok(v) => json!({ "ok": true, "result": v }),
                    Err(e) => json!({ "ok": false, "error": e }),
                },
            };
            let _ = request.respond(json_(200, &v));
        }
        ("POST", "/ide/chat/compact") => {
            // **The compaction of a chat** (`/compact`, lot 2 of the session archived as it goes):
            // `{backend}`. A refusal (a turn or a filing in flight, no memory) is an answer.
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("unreadable body: {e}") }),
                Ok(c) => match host.compacter_le_chat(c["backend"].as_str().unwrap_or_default()) {
                    Ok(v) => json!({ "ok": true, "result": v }),
                    Err(e) => json!({ "ok": false, "error": e }),
                },
            };
            let _ = request.respond(json_(200, &v));
        }
        ("GET", "/ide/recherches") => {
            // **Mes recherches** (8 novembre 2026) : lues dans le journal des runs.
            let limite = param("limite").and_then(|l| l.parse().ok()).unwrap_or(50);
            let v = match host.recent_searches(&param("backend").unwrap_or_default(), &param("qui").unwrap_or_default(), limite, param("avant").as_deref()) {
                Ok(v) => json!({ "ok": true, "result": v }),
                Err(e) => json!({ "ok": false, "error": e }),
            };
            let _ = request.respond(json_(200, &v));
        }
        ("POST", "/ide/recherches/clic") => {
            // **Un clic sur un résultat**, envoyé par la vue recherche sans retarder
            // la navigation (`keepalive`). Un refus est une réponse.
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("corps illisible : {e}") }),
                Ok(c) => match host.record_click(c["backend"].as_str().unwrap_or_default(), c["run"].as_str().unwrap_or_default(), c["uuid"].as_str().unwrap_or_default(), c["rang"].as_u64()) {
                    Ok(v) => json!({ "ok": true, "result": v }),
                    Err(e) => json!({ "ok": false, "error": e }),
                },
            };
            let _ = request.respond(json_(200, &v));
        }
        // **Les accords de l'atelier** (ask_permission) : la carte sous l'appel décide
        // (`{id, decision: accorder|refuser}`) ; relue après un rechargement, elle lit son
        // état (`?id=`).
        #[cfg(feature = "code")]
        ("POST", "/ide/accords") => {
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("corps illisible : {e}") }),
                Ok(c) => match c["decision"].as_str() {
                    Some("accorder") if !humain => refus_humain(),
                    Some(d @ ("accorder" | "refuser")) => match host.decider_accord(c["id"].as_str().unwrap_or_default(), d == "accorder", "l'utilisatrice, dans l'IDE") {
                        Ok(v) => json!({ "ok": true, "result": v }),
                        Err(e) => json!({ "ok": false, "error": e }),
                    },
                    _ => json!({ "ok": false, "error": "« decision » vaut accorder ou refuser" }),
                },
            };
            let _ = request.respond(json_(200, &v));
        }
        #[cfg(feature = "code")]
        ("GET", "/ide/accords") => {
            let v = match host.etat_accord(&param("id").unwrap_or_default()) {
                Some(v) => json!({ "ok": true, "result": v }),
                None => json!({ "ok": false, "error": "aucune demande de ce nom (un redémarrage de l'hôte les oublie ; le journal des décisions les garde)" }),
            };
            let _ = request.respond(json_(200, &v));
        }
        ("POST", "/ide/chemins") => {
            // **The paths cited in the chat** (`daemon_host::chemins_cites`): a link only of a path that exists.
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("corps illisible : {e}") }),
                Ok(c) => {
                    let paths: Vec<String> = c["paths"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect();
                    match host.chemins_cites(c["backend"].as_str().unwrap_or_default(), &paths) {
                        Ok(v) => json!({ "ok": true, "result": v }),
                        Err(e) => json!({ "ok": false, "error": e }),
                    }
                }
            };
            let _ = request.respond(json_(200, &v));
        }
        ("POST", "/ide/symboles") => {
            // **Les symboles cités dans le chat** (`daemon_host::symboles_cites`) : la page
            // les envoie en lot, l'index dit où chacun est défini.
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("corps illisible : {e}") }),
                Ok(c) => {
                    let noms: Vec<String> = c["names"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect();
                    match host.symboles_cites(c["backend"].as_str().unwrap_or_default(), &noms) {
                        Ok(v) => json!({ "ok": true, "result": v }),
                        Err(e) => json!({ "ok": false, "error": e }),
                    }
                }
            };
            let _ = request.respond(json_(200, &v));
        }
        ("GET", "/ide/bilans") => {
            let v = match host.bilans_de(&param("backend").unwrap_or_default()) {
                Ok(v) => json!({ "ok": true, "result": v }),
                Err(e) => json!({ "ok": false, "error": e }),
            };
            let _ = request.respond(json_(200, &v));
        }
        ("POST", "/ide/bilans") => {
            // **Juger un run à la demande** (page 04) : le juge relit le run par
            // son journal et range sa fiche dans la mémoire que le backend
            // déclare. Un refus (pas de `bilans`, pas de journal) est une réponse.
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("corps illisible : {e}") }),
                Ok(c) => match host.juger(c["backend"].as_str().unwrap_or_default(), c["run"].as_str().unwrap_or_default()) {
                    Ok(v) => json!({ "ok": true, "result": v }),
                    Err(e) => json!({ "ok": false, "error": e }),
                },
            };
            let _ = request.respond(json_(200, &v));
        }
        ("GET", "/ide/teammate") => {
            // **L'équipier d'une tâche** (page 07) : sa boîte, ses plans vivants,
            // ses plans clos, et s'il tourne.
            let (backend, task) = (param("backend").unwrap_or_default(), param("task").unwrap_or_default());
            let v = match host.teammate_files(&backend, &task) {
                // `backend` : le manifeste de la tâche, celui des trames de `/ide/flux`.
                Ok(f) => json!({ "ok": true, "result": {
                    "backend": host.tache(&backend, &task).map(|t| t.manifest.display().to_string()),
                    "mailbox": f.mailbox(), "plans": f.read_plans(), "closed": f.closed(), "running": host.teammate_running(&backend, &task),
                    // **The task's mode** (23 November 2026): `in_place` (the agent writes in the user's folder, nothing
                    // to merge), `copy`, or null for a task without a code repository.
                    "workspace": host.tache(&backend, &task).and_then(|t| t.workspace.as_ref().and_then(|w| w.code.as_ref()).map(|c| if c.en_place { "in_place" } else { "copy" })),
                    // **What waits for the supervisor** (a run seen and reachable in the IDE, c7, 20 Nov. 2026): the
                    // question or approval the teammate waits on, computed by the host as for the teammate itself.
                    "waiting": crate::daemon_host::teammate::open_question(&f).map(|(_, l)| l),
                } }),
                Err(e) => json!({ "ok": false, "error": e }),
            };
            let _ = request.respond(json_(200, &v));
        }
        // **An entry code for the IDE** (21 Nov 2026), asked with the user's token.
        ("POST", "/ide/entry-code") => {
            let v = match host.issue_entry_code(entete(&request, "X-Rag3weaver-User")) {
                Ok(code) => json!({ "ok": true, "code": code, "expires_in_s": crate::daemon_host::user_title::ENTRY_CODE_TTL_MS / 1000 }),
                Err(e) => json!({ "ok": false, "error": e }),
            };
            let statut = if v["ok"] == true { 200 } else { 403 };
            let _ = request.respond(json_(statut, &v));
        }
        ("POST", "/ide/teammate/mailbox") => {
            let utilisateur = entete(&request, "X-Rag3weaver-User").map(str::to_string);
            let client = if utilisateur.is_some() { "tty" } else { "ide" };
            let by = host.by_of(cookie(&request, COOKIE_USER).as_deref(), utilisateur.as_deref(), client);
            // **Écrire à l'équipier** : `{backend, task, kind, item?, text, from?}`.
            // Une `request` ouvre la tâche et lance l'équipier ; un `stop`
            // arrête le pas en cours tout de suite.
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("corps illisible : {e}") }),
                Ok(c) => match host.post_to_teammate(
                    c["backend"].as_str().unwrap_or_default(),
                    c["task"].as_str().unwrap_or_default(),
                    c["from"].as_str().unwrap_or("supervisor"),
                    c["kind"].as_str().unwrap_or_default(),
                    c["item"].as_str(),
                    c["text"].as_str().unwrap_or_default(),
                    // L'adresse et la réponse (page 07-novembre-2026-everything-declarative/01, § 2).
                    // `by` is the host's, from who calls; a `by` of the body is never read.
                    // `gate` (a request's level of the gate: bac, judge, cards) goes to the host, which says it in the mailbox.
                    &json!({ "to": c["to"], "reply_to": c["reply_to"], "approved": c["approved"], "expects": c["expects"], "gate": c["gate"], "by": by }),
                ) {
                    Ok(v) => json!({ "ok": true, "result": v }),
                    Err(e) => json!({ "ok": false, "error": e }),
                },
            };
            let _ = request.respond(json_(200, &v));
        }
        ("GET", "/ide/teammate/pending") => {
            // **Ce qui attend une clé** (`claude-code:<id>`, `codex:<nom>`,
            // `teammate:<parent>/<task>`, `supervisor`…, sans liste fermée) :
            // lecture pure, un rejeu ou une sonde ne consomment rien.
            let who = param("for").unwrap_or_default();
            let v = if who.is_empty() { json!({ "ok": false, "error": "for manque : la clé du destinataire" }) } else { json!({ "ok": true, "result": host.teammate_pending(&who) }) };
            let _ = request.respond(json_(200, &v));
        }
        ("GET", "/ide/teammate/models") => {
            // **Les modèles d'une tâche** (`/model`, lot tty-14) : ceux que déclare son manifeste,
            // le défaut et celui de la tâche marqués.
            let v = match host.modeles_de_tache(&param("backend").unwrap_or_default(), &param("task").unwrap_or_default()) {
                Ok(v) => json!({ "ok": true, "result": v }),
                Err(e) => json!({ "ok": false, "error": e }),
            };
            let _ = request.respond(json_(200, &v));
        }
        ("POST", "/ide/teammate/model") => {
            // **Choisir le modèle d'une tâche** : `{backend, task, name}` ; un nom inconnu se refuse en
            // nommant les modèles déclarés.
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("corps illisible : {e}") }),
                Ok(c) => match host.choisir_le_modele(c["backend"].as_str().unwrap_or_default(), c["task"].as_str().unwrap_or_default(), c["name"].as_str().unwrap_or_default()) {
                    Ok(v) => json!({ "ok": true, "result": v }),
                    Err(e) => json!({ "ok": false, "error": e }),
                },
            };
            let _ = request.respond(json_(200, &v));
        }
        ("POST", "/ide/teammate/name") => {
            // **Renommer l'agent d'une tâche ouverte** (tty-18, `/nom <x>`) : `{backend, task, agent_name}` ;
            // `""` rend le nom du backend ; un refus nomme `agent_name`.
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("corps illisible : {e}") }),
                Ok(c) => match c["agent_name"].as_str() {
                    None => json!({ "ok": false, "error": "agent_name manque : le nom de l'agent, ou \"\" pour celui du backend" }),
                    Some(nom) => match host.renommer_l_agent_d_une_tache(c["backend"].as_str().unwrap_or_default(), c["task"].as_str().unwrap_or_default(), nom) {
                        Ok(v) => json!({ "ok": true, "result": v }),
                        Err(e) => json!({ "ok": false, "error": e }),
                    },
                },
            };
            let _ = request.respond(json_(200, &v));
        }
        ("GET", "/ide/teammate/list") => {
            let who = param("supervisor").unwrap_or_default();
            let v = if who.is_empty() { json!({ "ok": false, "error": "supervisor manque : la clé du superviseur" }) } else { json!({ "ok": true, "result": host.teammate_list(&who) }) };
            let _ = request.respond(json_(200, &v));
        }
        ("POST", "/ide/teammate/seen") => {
            // **Vu** : `{for, ids}` ; la sortie d'un compte rendu ou d'une fin
            // est un acte, jamais l'effet d'une lecture.
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("corps illisible : {e}") }),
                Ok(c) => {
                    let ids: Vec<String> = c["ids"].as_array().into_iter().flatten().filter_map(|i| i.as_str().map(str::to_string)).collect();
                    match host.teammate_seen(c["for"].as_str().unwrap_or_default(), &ids) {
                        Ok(v) => json!({ "ok": true, "result": v }),
                        Err(e) => json!({ "ok": false, "error": e }),
                    }
                }
            };
            let _ = request.respond(json_(200, &v));
        }
        ("POST", "/ide/runs/epingler") => {
            // **Épingler un run** (page 06) : `{backend, run, raison}` ; son
            // journal ne tombera jamais. Un refus est une réponse.
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("corps illisible : {e}") }),
                Ok(c) => match host.epingler_un_run(c["backend"].as_str().unwrap_or_default(), c["run"].as_str().unwrap_or_default(), c["raison"].as_str().unwrap_or_default()) {
                    Ok(v) => json!({ "ok": true, "result": v }),
                    Err(e) => json!({ "ok": false, "error": e }),
                },
            };
            let _ = request.respond(json_(200, &v));
        }
        ("POST", "/ide/runs/arreter") => {
            // **Arrêter un run depuis la page** (C4) : le même chemin d'arrêt
            // que tout run, au nom de l'IDE. Un refus est une réponse.
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("corps illisible : {e}") }),
                Ok(c) => match host.stop_run(c["backend"].as_str().unwrap_or_default(), c["run"].as_str().unwrap_or_default(), "ide") {
                    Ok(v) => json!({ "ok": true, "result": v }),
                    Err(e) => json!({ "ok": false, "error": e }),
                },
            };
            let _ = request.respond(json_(200, &v));
        }
        // **Fork an agent** (`--fork-agent`, 22 November 2026): `{backend, task, name?}`, the copy.
        ("POST", "/ide/teammate/fork") => {
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("corps illisible : {e}") }),
                Ok(c) => match host.fork_agent(c["backend"].as_str().unwrap_or_default(), c["task"].as_str().unwrap_or_default(), c["name"].as_str()) {
                    Ok(t) => json!({ "ok": true, "result": t }),
                    Err(e) => json!({ "ok": false, "error": e }),
                },
            };
            let _ = request.respond(json_(200, &v));
        }
        // **Reopen a closed agent** (`--pop-agent`, 22 November 2026): `{backend, name, new_name?}`, the new task.
        ("POST", "/ide/agents/reopen") => {
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("corps illisible : {e}") }),
                Ok(c) => match host.reopen_agent(c["backend"].as_str().unwrap_or_default(), c["name"].as_str().unwrap_or_default(), c["new_name"].as_str()) {
                    Ok(t) => json!({ "ok": true, "result": t }),
                    Err(e) => json!({ "ok": false, "error": e }),
                },
            };
            let _ = request.respond(json_(200, &v));
        }
        ("POST", "/ide/taches" | "/ide/taches/fusionner" | "/ide/taches/rejeter") => {
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("corps illisible : {e}") }),
                Ok(c) => {
                    let id = c["backend"].as_str().unwrap_or_default();
                    let nom = c["nom"].as_str().unwrap_or_default();
                    let r = match path.as_str() {
                        // **A child of a session** (the sub-agent with views): named `<session>-<name>`,
                        // its views given by the parent at its opening.
                        "/ide/taches" => match c["parent_session"].as_str().filter(|s| !s.is_empty()) {
                            None => host.ouvrir_une_tache_nommee(id, nom, c["agent_name"].as_str(), c["agent_persona"].as_str()).map(|t| json!(t)),
                            Some(session) => {
                                let enfant = crate::daemon_host::taches::nom_d_enfant(session, nom);
                                let vues: Vec<String> = c["views"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
                                host.ouvrir_une_tache_nommee(id, &enfant, c["agent_name"].as_str(), c["agent_persona"].as_str()).and_then(|t| {
                                    host.donner_les_vues(id, &enfant, session, vues, c["active"].as_str().map(str::to_string)).map(|()| json!(t)).or_else(|e| {
                                        let _ = host.rejeter_une_tache(id, &enfant, &format!("views refused at its opening: {e}"));
                                        Err(e)
                                    })
                                })
                            }
                        },
                        "/ide/taches/fusionner" => host.fusionner_une_tache(id, nom, c["message"].as_str().unwrap_or_default()),
                        _ => host.rejeter_une_tache(id, nom, c["why"].as_str().unwrap_or_default()),
                    };
                    match r {
                        Ok(v) => json!({ "ok": true, "result": v }),
                        Err(e) => json!({ "ok": false, "error": e }),
                    }
                }
            };
            let _ = request.respond(json_(200, &v));
        }
        // **The parent changes a child's views** (the sub-agent with views): add, remove, switch.
        ("POST", "/ide/teammate/views") => {
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("corps illisible : {e}") }),
                Ok(c) => {
                    let liste = |k: &str| -> Vec<String> { c[k].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default() };
                    match host.changer_les_vues(c["backend"].as_str().unwrap_or_default(), c["task"].as_str().unwrap_or_default(), &liste("add"), &liste("remove"), c["active"].as_str()) {
                        Ok(v) => json!({ "ok": true, "result": v }),
                        Err(e) => json!({ "ok": false, "error": e }),
                    }
                }
            };
            let _ = request.respond(json_(200, &v));
        }
        // La coloration pour la page (coloration B) : la même fonction que
        // `POST /highlight` sous le jeton porteur, ici sous le cookie de l'IDE.
        ("POST", "/ide/highlight") => {
            let mut corps = Vec::new();
            let (statut, v) = match request.as_reader().read_to_end(&mut corps) {
                Err(e) => (400, json!({ "ok": false, "error": format!("corps illisible : {e}") })),
                Ok(_) => host.handle("POST", "/highlight", None, &corps),
            };
            let _ = request.respond(json_(statut, &v));
        }
        // **Open a folder** (Lucie's first step in the IDE, 21 November 2026): the field of the welcome page, on the
        // host's `/dossiers/ouvrir` under the IDE's session — created and `git init`, or attached, or refused saying why.
        ("POST", "/ide/ouvrir") => {
            let mut corps = Vec::new();
            let (statut, v) = match request.as_reader().read_to_end(&mut corps) {
                Err(e) => (400, json!({ "ok": false, "error": format!("corps illisible : {e}") })),
                Ok(_) => host.handle("POST", "/dossiers/ouvrir", Some(&host.ide_session()), &corps),
            };
            let _ = request.respond(json_(statut, &v));
        }
        // **The machine at a glance** (scenario 1 of the night of 20 November 2026): the page, and what it reads, the
        // host's `GET /poste` (sessions, folders, views, repositories, tasks).
        ("GET", "/ide/poste") | ("GET", "/ide/poste/") => {
            let _ = request.respond(reponse_octets(200, "text/html; charset=utf-8", bundle("index.html")));
        }
        ("GET", "/ide/poste/etat") => {
            let (statut, v) = host.handle("GET", "/poste", None, &[]);
            let _ = request.respond(json_(statut, &v));
        }
        ("GET", "/ide/etat") => {
            let (statut, mut v) = host.handle("GET", "/etat", None, &[]);
            // **Whether the page is entered as the user** (`human-yes-only`): the page says it at its head when not.
            v["human"] = json!(humain);
            let _ = request.respond(json_(statut, &v));
        }
        ("POST", "/ide/call" | "/ide/describe") => {
            let mut corps = Vec::new();
            let (statut, v) = match request.as_reader().read_to_end(&mut corps) {
                Err(e) => (400, json!({ "ok": false, "error": format!("corps illisible : {e}") })),
                Ok(_) => {
                    let route = if path == "/ide/call" { "/call" } else { "/describe" };
                    let session = host.ide_session();
                    host.handle("POST", route, Some(&session), &corps)
                }
            };
            // **Un refus d'outil est une réponse, pas une panne** : la page lit
            // `ok: false` (le conflit d'un enregistrement sur un fichier changé
            // en est un, attendu). En 422, le navigateur le journalisait comme une
            // ressource en échec, à chaque conflit (vu par le témoin navigateur,
            // 19 octobre 2026). Le statut de l'hôte reste dans le corps.
            let (statut, v) = match statut {
                422 => {
                    let mut v = v;
                    v["status"] = json!(422);
                    (200, v)
                }
                autre => (autre, v),
            };
            let _ = request.respond(json_(statut, &v));
        }
        ("GET", _) => {
            let _ = request.respond(reponse(200, "text/html; charset=utf-8", PAGE.to_string()));
        }
        _ => {
            let _ = request.respond(json_(404, &json!({ "ok": false, "error": format!("route inconnue : {method} {path}") })));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un écrivain qui garde ce qu'on lui écrit.
    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<u8>>>);

    impl Write for Capture {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Capture {
        /// What the window received, **once its writer has delivered**: each window has its own writer thread
        /// (ticket of the 14th), so a frame arrives just after `publier` returns. Read when the capture has stopped
        /// growing for 20 ms, 2 s at most.
        fn texte(&self) -> String {
            let mut avant = usize::MAX;
            for _ in 0..100 {
                let n = self.0.lock().unwrap().len();
                if n == avant {
                    break;
                }
                avant = n;
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
        /// The ids received **once at least `n` are there** (21 November 2026, a pass at full load read `[3, 4]` for
        /// `[3, 4, 5]`): the wait is bounded on the event — the n-th id delivered — and by 5 s, never a bare sleep;
        /// then the stable read, which would still show a duplicate.
        fn ids_attendus(&self, n: usize) -> Vec<u64> {
            let fin = std::time::Instant::now() + std::time::Duration::from_secs(5);
            let compte = || String::from_utf8_lossy(&self.0.lock().unwrap()).lines().filter(|l| l.starts_with("id: ")).count();
            while compte() < n && std::time::Instant::now() < fin {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            self.ids()
        }
        /// Les ids des événements reçus, dans l'ordre.
        fn ids(&self) -> Vec<u64> {
            self.texte().lines().filter_map(|l| l.strip_prefix("id: ")).map(|v| v.parse().unwrap()).collect()
        }
    }

    fn fichier(path: &str) -> Value {
        json!({ "kind": "FileChanged", "path": path })
    }

    /// A window whose writer delivers late (a loaded machine: the writer thread scheduled after `publier` returned).
    struct Lente(Capture, std::time::Duration);

    impl Write for Lente {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            std::thread::sleep(self.1);
            self.0.write(b)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// **A capture is read once its writer has delivered, even a late one** (the pass of `ed-exact-name-by-name-4`,
    /// 21 November 2026, 06 h 57, at full load: `two_windows_share_ids_and_a_returning_one_gets_what_it_missed` read
    /// `[3, 4]` for `[3, 4, 5]`). The capture was read « stable » after 20 ms without growth, before a late writer
    /// delivered anything.
    #[test]
    fn a_capture_is_read_once_a_late_writer_has_delivered() {
        let flux = Flux::new();
        let lente = Capture::default();
        flux.abonner(Box::new(Lente(lente.clone(), std::time::Duration::from_millis(60))), None, None);
        flux.publier("files", "b", fichier("a.rs"));
        assert_eq!(lente.ids_attendus(1), vec![1], "the late window read before its writer delivered");
    }

    /// **Des relais qui publient ensemble ne perdent rien** (ticket du 7 novembre
    /// 2026, « l'affichage en direct du chat peut manquer le premier jeton »).
    /// Quatre fils publient chacun sur son sujet, en boucle, en même temps : la
    /// fenêtre doit recevoir chaque id une fois exactement, dans l'ordre, sans
    /// trou, et le tampon doit les garder dans le même ordre (le rejeu par
    /// `Last-Event-ID` le suit). Avant, un id tiré hors du verrou laissait un
    /// fil envoyer N+1 avant N, et `envoyer` sautait N en silence.
    #[test]
    fn relays_publishing_together_lose_nothing_and_keep_the_order() {
        let flux = Flux::new();
        let fenetre = Capture::default();
        flux.abonner(Box::new(fenetre.clone()), None, None);
        const PAR_FIL: u64 = 2_000;
        std::thread::scope(|s| {
            for sujet in ["tokens", "agent", "dataflow", "catalog"] {
                let flux = &flux;
                s.spawn(move || {
                    for n in 0..PAR_FIL {
                        flux.publier(sujet, "b", json!({ "kind": "Token", "seq": n }));
                    }
                });
            }
        });
        let ids = fenetre.ids_attendus(4 * PAR_FIL as usize);
        let attendus: Vec<u64> = (1..=4 * PAR_FIL).collect();
        if ids != attendus {
            let manquants: Vec<u64> = attendus.iter().copied().filter(|i| !ids.contains(i)).take(10).collect();
            let desordre = ids.windows(2).filter(|w| w[0] >= w[1]).count();
            panic!("{} reçus sur {} ; manquants (10 premiers) : {manquants:?} ; hors d'ordre : {desordre}", ids.len(), attendus.len());
        }
        let tampon: Vec<u64> = flux.tampon.lock().unwrap().iter().map(|e| e.id).collect();
        assert!(tampon.windows(2).all(|w| w[0] < w[1]), "le tampon n'est pas dans l'ordre des ids");
    }

    /// **Deux fenêtres reçoivent le même événement sous le même id** ; une
    /// fenêtre neuve commence à maintenant ; une fenêtre qui revient avec le
    /// dernier id reçu (`Last-Event-ID`) reçoit ce qu'elle a manqué, une fois.
    #[test]
    fn two_windows_share_ids_and_a_returning_one_gets_what_it_missed() {
        let flux = Flux::new();
        flux.publier("files", "b", fichier("a.rs"));
        let (un, deux) = (Capture::default(), Capture::default());
        flux.abonner(Box::new(un.clone()), None, None);
        flux.abonner(Box::new(deux.clone()), None, None);
        flux.publier("files", "b", fichier("b.rs"));
        assert_eq!(un.ids_attendus(1), vec![2], "une fenêtre neuve commence à maintenant");
        assert_eq!(un.ids_attendus(1), deux.ids_attendus(1), "le même événement, le même id");
        flux.publier("files", "b", fichier("c.rs"));
        flux.publier("dataflow", "b", json!({ "kind": "RunStarted" }));
        let revenue = Capture::default();
        flux.abonner(Box::new(revenue.clone()), None, Some(2));
        assert_eq!(revenue.ids_attendus(2), vec![3, 4], "ce qu'elle a manqué depuis 2");
        flux.publier("files", "b", fichier("d.rs"));
        assert_eq!(revenue.ids_attendus(3), vec![3, 4, 5], "puis la suite, sans doublon");
        assert!(revenue.texte().contains("\"backend\":\"b\"") && revenue.texte().contains("d.rs"));
    }

    /// **Les sujets filtrent** ; un retour plus vieux que le tampon dit sa lacune.
    #[test]
    fn subjects_filter_and_a_too_old_return_says_its_gap() {
        let flux = Flux::new();
        let fichiers = Capture::default();
        flux.abonner(Box::new(fichiers.clone()), Some(HashSet::from(["files".to_string()])), None);
        flux.publier("dataflow", "b", json!({ "kind": "RunStarted" }));
        flux.publier("files", "b", fichier("a.rs"));
        assert_eq!(fichiers.ids_attendus(1), vec![2]);
        for i in 0..TAMPON + 10 {
            flux.publier("files", "b", fichier(&format!("f{i}.rs")));
        }
        let vieille = Capture::default();
        flux.abonner(Box::new(vieille.clone()), None, Some(1));
        let t = vieille.texte();
        assert!(t.contains("event: lacune"), "{}", &t[..t.len().min(300)]);
        assert_eq!(vieille.ids_attendus(TAMPON).len(), TAMPON, "puis tout ce que le tampon garde");
    }

    /// **Le relais ne dépend pas du runtime des graphes** : ses deux fils tenus
    /// par des nœuds qui bloquent (comme sous la lib entière), un événement du
    /// bus arrive quand même à la fenêtre, tout de suite. Avant, il attendait
    /// qu'un fil se libère.
    #[test]
    fn the_relay_does_not_wait_for_the_graph_runtime() {
        let flux = Flux::new();
        let bus = EventBus::new(64);
        flux.relayer("/x/backend.json", &bus);
        let fenetre = Capture::default();
        flux.abonner(Box::new(fenetre.clone()), None, None);
        // Les deux fils du runtime tenus 1,5 s (court : les autres témoins du
        // processus le partagent), et la preuve qu'ils le sont.
        let tenus = Arc::new(std::sync::Barrier::new(3));
        for _ in 0..2 {
            let tenus = tenus.clone();
            crate::dataflow::rt::runtime().spawn(async move {
                tenus.wait();
                std::thread::sleep(std::time::Duration::from_millis(1500));
            });
        }
        tenus.wait();
        let envoi = std::time::Instant::now();
        bus.emit(crate::events::Event::FileChanged {
            watch: "code".into(),
            path: "src/lib.rs".into(),
            change: "modified".into(),
            hash: "h".into(),
            at_ms: 1,
        });
        while fenetre.ids().is_empty() && envoi.elapsed() < std::time::Duration::from_millis(500) {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(!fenetre.ids().is_empty(), "l'événement a attendu les fils du runtime des graphes : rien en {:?}", envoi.elapsed());
    }

    /// **Le bus d'un backend est relayé** : un `FileChanged` émis sur le bus
    /// arrive à la fenêtre, avec son sujet et son backend.
    /// **A run's words are relayed between its start and its end** (an end-to-end witness on 23 November 2026: 132 to
    /// 162 of 300 words of a teammate step reached the terminal after the `RunFinished` of their run, `agent` and
    /// `tokens` being relayed by two threads; the ticket of 7 November had seen a `Token` pass its `RunStarted`). A
    /// window sees the `RunStarted` of a run before its first `Token`, and its `RunFinished` after its last.
    #[test]
    fn a_runs_words_are_relayed_between_its_start_and_its_end() {
        use crate::events::Event;
        for essai in 0..5 {
            let flux = Flux::new();
            let bus = EventBus::new(4096);
            flux.relayer("/x/backend.json", &bus);
            let fenetre = Capture::default();
            flux.abonner(Box::new(fenetre.clone()), None, None);
            let run = format!("agent-{essai}");
            bus.emit(Event::RunStarted { run: run.clone(), parent: None, kind: "agent".into(), name: "chat".into(), domain: String::new(), scope: None, target: None, role: None, step: None });
            for i in 0..300u64 {
                bus.emit(Event::Token { run: run.clone(), node: "chat".into(), seq: i, text: format!("mot{i} "), count: 1 });
            }
            bus.emit(Event::RunFinished { run: run.clone(), kind: "agent".into(), ms: 1, ok: true, cause: None, cancelled_by: None });
            fenetre.ids_attendus(302);
            let t = fenetre.texte();
            let genres: Vec<String> = t
                .lines()
                .filter_map(|l| l.strip_prefix("data: "))
                .filter_map(|d| serde_json::from_str::<Value>(d).ok())
                .filter(|v| v["evenement"]["run"] == run.as_str())
                .map(|v| v["evenement"]["kind"].as_str().unwrap_or_default().to_string())
                .collect();
            let debut = genres.iter().position(|k| k == "RunStarted");
            let fin = genres.iter().position(|k| k == "RunFinished");
            let premier = genres.iter().position(|k| k == "Token");
            let dernier = genres.iter().rposition(|k| k == "Token");
            assert_eq!(genres.iter().filter(|k| *k == "Token").count(), 300, "essai {essai} : tous les mots");
            assert!(debut < premier, "essai {essai} : RunStarted ({debut:?}) avant le premier mot ({premier:?})");
            assert!(dernier < fin, "essai {essai} : RunFinished ({fin:?}) après le dernier mot ({dernier:?})");
        }
    }

    #[test]
    fn a_backend_bus_is_relayed_to_the_windows() {
        let flux = Flux::new();
        let bus = EventBus::new(64);
        flux.relayer("/x/backend.json", &bus);
        let fenetre = Capture::default();
        flux.abonner(Box::new(fenetre.clone()), None, None);
        bus.emit(crate::events::Event::FileChanged {
            watch: "code".into(),
            path: "src/lib.rs".into(),
            change: "modified".into(),
            hash: "h".into(),
            at_ms: 1,
        });
        let fin = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while fenetre.ids().is_empty() && std::time::Instant::now() < fin {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let t = fenetre.texte();
        assert!(t.contains("FileChanged") && t.contains("src/lib.rs") && t.contains("\"sujet\":\"files\"") && t.contains("/x/backend.json"), "{t}");
    }

    /// **La page n'appelle que ce que le gabarit `code` sert** (lot A3) : le
    /// bloc `outils` de la page nomme chaque outil appelé et ses paramètres ;
    /// chacun existe dans le manifeste du gabarit, et chaque paramètre est
    /// déclaré par son graphe (`%% param:`) sans être lié par le manifeste. Et
    /// chaque `appeler(…, "outil", …)` de la page est dans le bloc, comme chaque
    /// `appelerEntier(…, "outil", …)` (la réponse entière, pour la recherche) :
    /// une forme oubliée sortirait ses appels de la garde sans bruit.
    #[test]
    fn the_page_only_calls_what_the_code_template_serves() {
        let debut = PAGE.find(r#"<script type="application/json" id="outils">"#).expect("le bloc outils");
        let bloc = &PAGE[debut..];
        let bloc = &bloc[bloc.find('>').unwrap() + 1..bloc.find("</script>").unwrap()];
        let outils: serde_json::Map<String, Value> = serde_json::from_str(bloc).expect("le bloc outils est du JSON");
        let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
        let manifeste: Value = serde_json::from_slice(&std::fs::read(racine.join("backend.json")).unwrap()).unwrap();
        for (outil, params) in &outils {
            let attache = &manifeste["tools"][outil];
            assert!(attache.is_object(), "le gabarit code ne sert pas « {outil} »");
            let graphe = std::fs::read_to_string(racine.join(attache["graph"].as_str().unwrap())).unwrap();
            let declares: HashSet<&str> = graphe
                .lines()
                .filter_map(|l| l.strip_prefix("%% param: "))
                .filter_map(|l| l.split_whitespace().next())
                .collect();
            for p in params.as_array().unwrap() {
                let p = p.as_str().unwrap();
                assert!(declares.contains(p), "{outil} : « {p} » n'est pas un paramètre de son graphe ({declares:?})");
                assert!(attache["bindings"].get(p).is_none(), "{outil} : « {p} » est lié par le manifeste, la page ne peut pas le passer");
            }
        }
        let appeles: HashSet<String> = ["appeler(id, \"", "appelerEntier(id, \""]
            .iter()
            .flat_map(|forme| PAGE_JS.split(*forme).skip(1))
            .filter_map(|r| r.split('"').next().map(str::to_string))
            .collect();
        assert!(appeles.contains("search_code"), "la recherche, appelée par appelerEntier, est vue par la garde");
        assert!(!appeles.is_empty());
        for a in &appeles {
            assert!(outils.contains_key(a), "la page appelle « {a} », absent du bloc outils");
        }
    }

    /// **La page n'a pas de code en ligne** : ses scripts sont des fichiers
    /// servis par l'hôte (sauf le bloc `outils`, des données), que les outils de
    /// code lisent comme le reste. La page charge `/ide/page.js`, et cette route
    /// le sert avant la page de repli.
    #[test]
    fn the_page_code_is_a_served_file() {
        let mut reste = PAGE;
        while let Some(i) = reste.find("<script") {
            let balise = &reste[i..i + reste[i..].find('>').unwrap()];
            assert!(balise.contains(" src=") || balise.contains("application/json"), "du code en ligne dans la page : {balise}");
            reste = &reste[i + 1..];
        }
        assert!(PAGE.contains(r#"<script type="module" src="/ide/page.js"></script>"#));
    }

    /// **Le bundle servi est celui que `bundle.json` décrit** (page 08) : les
    /// octets embarqués d'`app.js` et d'`app.css` ont l'empreinte sha256 que le
    /// bâti a écrite, et les versions épinglées sont celles de `ui/ide/package.json`.
    /// Un bundle rebâti sans son manifeste, ou un `package.json` monté de version
    /// sans rebâti, rougit ici — avant même l'étape `ide_bundle.sh --check`.
    #[test]
    fn le_bundle_de_l_ide_est_celui_que_son_manifeste_decrit() {
        use sha2::Digest;
        let manifeste: Value = serde_json::from_slice(crate::builtin_templates::get("ide/app/bundle.json").expect("templates/ide/app/bundle.json")).unwrap();
        for nom in ["app.js", "app.css"] {
            let octets = crate::builtin_templates::get(&format!("ide/app/{nom}")).unwrap_or_else(|| panic!("templates/ide/app/{nom} embarqué"));
            let empreinte = format!("{:x}", sha2::Sha256::digest(octets));
            assert_eq!(manifeste["sha256"][nom], empreinte, "{nom} : le bundle servi n'est pas celui du manifeste — relancer scripts/ide_bundle.sh");
        }
        let pkg: Value = serde_json::from_str(include_str!("../ui/ide/package.json")).unwrap();
        for (nom, version) in pkg["dependencies"].as_object().unwrap().iter().chain(pkg["devDependencies"].as_object().unwrap()) {
            assert_eq!(&manifeste["versions"][nom], version, "{nom} : package.json a bougé sans rebâti");
        }
        let index = String::from_utf8_lossy(crate::builtin_templates::get("ide/app/index.html").unwrap());
        assert!(index.contains(r#"<script type="module" src="/ide/app.js"></script>"#) && index.contains(r#"href="/ide/app.css""#), "{index}");
        assert!(PAGE_JS.contains("function route()"), "le code de la page est dans page.js");
        let atelier = include_str!("atelier/page.html");
        assert!(!atelier.contains("<script type=\"module\">"), "du code en ligne dans la page de l'atelier");
        assert!(atelier.contains(r#"<script type="module" src="/_atelier/page.js"></script>"#));
    }

    /// **Les routes de l'aperçu** (lot 3) : la page pour un composant ; ses
    /// modules d'un backend inconnu, refusés en le nommant ; un chemin
    /// incomplet, 404.
    #[test]
    fn the_preview_routes_answer() {
        let host = DaemonHost::new().with_background(Arc::new(crate::dataflow::background::Registry::new()));
        let (s, t, c) = apercu(&host, "/ide/apercu/abc/Fiche", None, false);
        assert_eq!(s, 200);
        assert!(t.starts_with("text/html") && c.contains("/ide/registre.js") && c.contains("modules.js"), "{c}");
        let (s, _, c) = apercu(&host, "/ide/apercu/abc/Fiche/modules.js", None, false);
        assert_eq!(s, 422, "{c}");
        assert!(c.contains("abc"), "{c}");
        assert_eq!(apercu(&host, "/ide/apercu/abc", None, false).0, 404);
    }

    /// **Les modules de l'éditeur se rendent**, depuis le magasin embarqué : le
    /// graphe de CodeMirror sans import non résolu, chaque lib du démarrage
    /// présente, et leur code exécutable par le registre (rquickjs, sans DOM :
    /// on ne fait que charger `@codemirror/state`, qui n'en a pas besoin).
    #[test]
    fn the_editor_modules_render_from_the_embedded_store() {
        let js = editeur_modules().as_ref().unwrap_or_else(|e| panic!("{e}"));
        for spec in EDITEUR {
            assert!(js.contains(&format!("{spec:?}")) || js.contains(spec), "{spec} au démarrage");
        }
        let programme = format!(
            "{}{js}\nconst S = __rag3weaver.require(__rag3weaver.demarrage[\"@codemirror/state\"]);\nS.EditorState.create({{ doc: \"ab\" }}).doc.toString();",
            crate::components::modules::REGISTRE
        );
        let rt = rquickjs::Runtime::new().unwrap();
        let ctx = rquickjs::Context::full(&rt).unwrap();
        let doc: String = ctx.with(|ctx| match ctx.eval::<String, _>(programme.clone()) {
            Ok(s) => s,
            Err(e) => panic!("{e} — {:?}", ctx.catch()),
        });
        assert_eq!(doc, "ab");
    }

    /// **Le script qui range les libs de l'éditeur réduit le magasin aux mêmes
    /// entrées que la page** : une seule liste, `EDITEUR`, dite deux fois.
    #[test]
    fn the_ide_libs_script_prunes_to_the_editor_entries() {
        let script = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/ide_libs.sh")).unwrap();
        let ligne = script.lines().find(|l| l.contains("libs prune")).expect("ide_libs.sh réduit le magasin");
        let entrees: Vec<&str> = ligne.split_whitespace().filter(|m| m.starts_with('@') || m.starts_with("codemirror")).collect();
        assert_eq!(entrees, EDITEUR, "{ligne}");
    }

    #[test]
    fn url_parameters_are_decoded() {
        assert_eq!(parametres("/ide/flux?sujets=files%2Cdataflow&depuis=12"), vec![("sujets".to_string(), "files,dataflow".to_string()), ("depuis".to_string(), "12".to_string())]);
        assert_eq!(decoder("a%2"), "a%2", "un % tronqué reste tel quel");
    }
}
