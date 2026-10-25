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
//! | `GET /ide/entrer?jeton=…` | le jeton local posé en cookie (`HttpOnly`, `SameSite=Strict`), puis la page |
//! | `GET /ide`, `GET /ide/…` | la page (une seule, qui lit son URL) |
//! | `GET /ide/graphe?backend=<id>&tool=…` | le graphe d'un outil (ou `reaction=`, `component=`), en JSON |
//! | `GET /ide/reactions?backend=<id>` | les réactions du backend, ses runs, l'état de ses montres |
//! | `GET /ide/propositions?backend=<id>[&id=<n>]` | les propositions, ou le détail d'une (diffs, graphes avant/après) |
//! | `POST /ide/propositions[/appliquer\|/rejeter]` | proposer `{backend, files, reason}`, appliquer `{backend, id}`, rejeter `{backend, id, why}` |
//! | `GET /ide/taches?backend=<id>[&nom=<n>]` | les tâches de l'agent, ou la relecture d'une (fichiers, graphes, témoins) |
//! | `POST /ide/taches[/fusionner\|/rejeter]` | ouvrir `{backend, nom}`, fusionner `{backend, nom, message}`, rejeter `{backend, nom, why}` |
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
//! porte `Authorization: Bearer` passe aussi.
//!
//! **Le flux** : chaque backend ouvert par l'hôte voit ses sujets relayés
//! ([`SUJETS`]). Un événement porte un `id:` croissant ; une fenêtre qui se
//! reconnecte avec `Last-Event-ID` (ou `?depuis=`) reçoit ce qu'elle a
//! manqué tant que le tampon le garde, et une `lacune` dite sinon.

use std::collections::{HashSet, VecDeque};
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
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
/// Le pas du battement qui fait tomber une fenêtre fermée.
const BATTEMENT: std::time::Duration = std::time::Duration::from_secs(15);

const PAGE: &str = include_str!("ide/page.html");

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

/// Une fenêtre abonnée : ses sujets (tous si `None`), le dernier id envoyé.
struct Abonne {
    sujets: Option<HashSet<String>>,
    dernier: u64,
    ecrivain: Box<dyn Write + Send>,
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
        self.ecrivain.write_all(e.trame.as_bytes()).and_then(|_| self.ecrivain.flush()).is_ok()
    }
}

/// **Le flux de l'IDE** : le bus des backends, numéroté, rejouable.
pub struct Flux {
    prochain: AtomicU64,
    tampon: Mutex<VecDeque<Arc<Entree>>>,
    abonnes: Mutex<Vec<Abonne>>,
}

impl Flux {
    /// Un flux vide, et son battement (un fil qui s'arrête avec lui).
    pub fn new() -> Arc<Self> {
        let flux = Arc::new(Self { prochain: AtomicU64::new(1), tampon: Mutex::new(VecDeque::new()), abonnes: Mutex::new(Vec::new()) });
        let faible: Weak<Self> = Arc::downgrade(&flux);
        let _ = std::thread::Builder::new().name("ide-battement".into()).spawn(move || loop {
            std::thread::sleep(BATTEMENT);
            let Some(flux) = faible.upgrade() else { break };
            flux.battre();
        });
        flux
    }

    /// Publie `evenement` (déjà en JSON) sous `sujet`, pour le backend `backend`.
    pub fn publier(&self, sujet: &str, backend: &str, evenement: Value) {
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
        self.abonnes.lock().unwrap_or_else(|p| p.into_inner()).retain_mut(|a| a.envoyer(&entree));
    }

    /// **Abonne une fenêtre** : l'en-tête SSE, puis ce qu'elle a manqué depuis
    /// `depuis` (une `lacune` dite si le tampon ne le garde plus), puis la suite.
    pub fn abonner(&self, mut ecrivain: Box<dyn Write + Send>, sujets: Option<HashSet<String>>, depuis: Option<u64>) {
        // Sous le verrou des abonnés : une publication qui tombe pendant le
        // rejeu attend, puis passe ; l'abonné saute ce qu'il a déjà eu.
        let mut abonnes = self.abonnes.lock().unwrap_or_else(|p| p.into_inner());
        let entete = b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\n\
            Connection: keep-alive\r\n\r\n: ide\n\n";
        if ecrivain.write_all(entete).and_then(|_| ecrivain.flush()).is_err() {
            return;
        }
        let mut abonne = Abonne { sujets, dernier: 0, ecrivain };
        if let Some(depuis) = depuis {
            abonne.dernier = depuis;
            let tampon: Vec<Arc<Entree>> = self.tampon.lock().unwrap_or_else(|p| p.into_inner()).iter().cloned().collect();
            let premier = tampon.first().map(|e| e.id).unwrap_or_else(|| self.prochain.load(Ordering::SeqCst));
            if depuis + 1 < premier {
                let trame = format!(
                    "event: lacune\ndata: {}\n\n",
                    json!({ "depuis": depuis, "premier": premier, "raison": "le tampon du flux ne garde plus ces événements : relire l'état" })
                );
                if abonne.ecrivain.write_all(trame.as_bytes()).and_then(|_| abonne.ecrivain.flush()).is_err() {
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
            .retain_mut(|a| a.ecrivain.write_all(b": battement\n\n").and_then(|_| a.ecrivain.flush()).is_ok());
    }

    /// Combien de fenêtres écoutent.
    pub fn fenetres(&self) -> usize {
        self.abonnes.lock().unwrap_or_else(|p| p.into_inner()).len()
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
            let mut rx = bus.subscribe(sujet);
            let (flux, backend, sujet) = (Arc::downgrade(self), backend.to_string(), sujet.to_string());
            let lance = std::thread::Builder::new().name(format!("ide-relais-{sujet}")).spawn(move || loop {
                let evenement = match rx.recv_blocking() {
                    Ok(e) => e.to_json(),
                    Err(async_broadcast::RecvError::Overflowed(n)) => json!({ "kind": "EventsMissed", "count": n }),
                    Err(async_broadcast::RecvError::Closed) => break,
                };
                let Some(flux) = flux.upgrade() else { break };
                flux.publier(&sujet, &backend, evenement);
            });
            if let Err(e) = lance {
                eprintln!("[ide] le relais de « {sujet} » ne part pas : {e}");
            }
        }
    }
}

// ─── L'aperçu d'un composant (marche 3, lot 3) ──────────────────────────────

const APERCU: &str = include_str!("ide/apercu.html");

/// `/ide/apercu/<backend>/<Composant>` : la page ; `…/modules.js` : ses
/// modules, enveloppés, puis l'entrée et le démarrage pour le registre.
fn apercu(host: &DaemonHost, path: &str, proposition: Option<u64>) -> (u16, &'static str, String) {
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
fn jeton_du_cookie(request: &tiny_http::Request) -> Option<String> {
    entete(request, "Cookie")?
        .split(';')
        .filter_map(|c| c.trim().split_once('='))
        .find(|(k, _)| *k == COOKIE)
        .map(|(_, v)| v.to_string())
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

    // L'entrée : le jeton de l'URL devient le cookie, et l'URL le perd.
    if method == "GET" && path == "/ide/entrer" {
        let jeton = param("jeton").unwrap_or_default();
        let r = if juste(&jeton) {
            let cookie = format!("{COOKIE}={jeton}; HttpOnly; SameSite=Strict; Path={PREFIX}");
            tiny_http::Response::from_string(String::new())
                .with_status_code(303)
                .with_header(tiny_http::Header::from_bytes(&b"Location"[..], &b"/ide/"[..]).expect("en-tête littéral"))
                .with_header(tiny_http::Header::from_bytes(&b"Set-Cookie"[..], cookie.as_bytes()).expect("cookie ASCII"))
        } else {
            json_(403, &json!({ "ok": false, crate::daemon_host::BAD_TOKEN: true, "error": "jeton absent ou faux : l'URL d'entrée est donnée par `rag3weaver-backend ide`" }))
        };
        let _ = request.respond(r);
        return;
    }

    let porte = attendu.is_none()
        || entete(&request, "Authorization").is_some_and(|h| attendu.is_some_and(|a| crate::daemon_host::same_token(h, a)))
        || jeton_du_cookie(&request).is_some_and(|j| juste(&j));
    if !porte {
        let _ = request.respond(json_(
            403,
            &json!({ "ok": false, crate::daemon_host::BAD_TOKEN: true, "error": "pas de jeton : entrer par l'URL que donne `rag3weaver-backend ide`" }),
        ));
        return;
    }

    match (method.as_str(), path.as_str()) {
        ("GET", "/ide/flux") => {
            let sujets = param("sujets").map(|s| s.split(',').filter(|x| !x.is_empty()).map(str::to_string).collect::<HashSet<_>>());
            let depuis = entete(&request, "Last-Event-ID").map(str::to_string).or_else(|| param("depuis")).and_then(|v| v.trim().parse::<u64>().ok());
            let ecrivain = request.into_writer();
            host.ide_flux().abonner(ecrivain, sujets, depuis);
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
            let (statut, type_, corps) = apercu(host, p, param("proposition").and_then(|v| v.parse().ok()));
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
            // (éteint, un tour déjà en cours) est une réponse.
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("corps illisible : {e}") }),
                Ok(c) => match host.chat_turn(c["backend"].as_str().unwrap_or_default(), c["message"].as_str().unwrap_or_default()) {
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
        ("POST", "/ide/taches" | "/ide/taches/fusionner" | "/ide/taches/rejeter") => {
            let mut corps = Vec::new();
            let v = match request.as_reader().read_to_end(&mut corps).map_err(|e| e.to_string()).and_then(|_| serde_json::from_slice::<Value>(&corps).map_err(|e| e.to_string())) {
                Err(e) => json!({ "ok": false, "error": format!("corps illisible : {e}") }),
                Ok(c) => {
                    let id = c["backend"].as_str().unwrap_or_default();
                    let nom = c["nom"].as_str().unwrap_or_default();
                    let r = match path.as_str() {
                        "/ide/taches" => host.ouvrir_une_tache(id, nom).map(|t| json!(t)),
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
        ("GET", "/ide/etat") => {
            let (statut, v) = host.handle("GET", "/etat", None, &[]);
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
        fn texte(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
        /// Les ids des événements reçus, dans l'ordre.
        fn ids(&self) -> Vec<u64> {
            self.texte().lines().filter_map(|l| l.strip_prefix("id: ")).map(|v| v.parse().unwrap()).collect()
        }
    }

    fn fichier(path: &str) -> Value {
        json!({ "kind": "FileChanged", "path": path })
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
        assert_eq!(un.ids(), vec![2], "une fenêtre neuve commence à maintenant");
        assert_eq!(un.ids(), deux.ids(), "le même événement, le même id");
        flux.publier("files", "b", fichier("c.rs"));
        flux.publier("dataflow", "b", json!({ "kind": "RunStarted" }));
        let revenue = Capture::default();
        flux.abonner(Box::new(revenue.clone()), None, Some(2));
        assert_eq!(revenue.ids(), vec![3, 4], "ce qu'elle a manqué depuis 2");
        flux.publier("files", "b", fichier("d.rs"));
        assert_eq!(revenue.ids(), vec![3, 4, 5], "puis la suite, sans doublon");
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
        assert_eq!(fichiers.ids(), vec![2]);
        for i in 0..TAMPON + 10 {
            flux.publier("files", "b", fichier(&format!("f{i}.rs")));
        }
        let vieille = Capture::default();
        flux.abonner(Box::new(vieille.clone()), None, Some(1));
        let t = vieille.texte();
        assert!(t.contains("event: lacune"), "{}", &t[..t.len().min(300)]);
        assert_eq!(vieille.ids().len(), TAMPON, "puis tout ce que le tampon garde");
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
    /// chaque `appeler(…, "outil", …)` de la page est dans le bloc.
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
        let appeles: HashSet<String> = PAGE
            .split("appeler(id, \"")
            .skip(1)
            .filter_map(|r| r.split('"').next().map(str::to_string))
            .collect();
        assert!(!appeles.is_empty());
        for a in &appeles {
            assert!(outils.contains_key(a), "la page appelle « {a} », absent du bloc outils");
        }
    }

    /// **Les routes de l'aperçu** (lot 3) : la page pour un composant ; ses
    /// modules d'un backend inconnu, refusés en le nommant ; un chemin
    /// incomplet, 404.
    #[test]
    fn the_preview_routes_answer() {
        let host = DaemonHost::new().with_background(Arc::new(crate::dataflow::background::Registry::new()));
        let (s, t, c) = apercu(&host, "/ide/apercu/abc/Fiche", None);
        assert_eq!(s, 200);
        assert!(t.starts_with("text/html") && c.contains("/ide/registre.js") && c.contains("modules.js"), "{c}");
        let (s, _, c) = apercu(&host, "/ide/apercu/abc/Fiche/modules.js", None);
        assert_eq!(s, 422, "{c}");
        assert!(c.contains("abc"), "{c}");
        assert_eq!(apercu(&host, "/ide/apercu/abc", None).0, 404);
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
