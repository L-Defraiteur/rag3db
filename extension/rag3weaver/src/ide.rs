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
pub const SUJETS: [&str; 7] = ["files", "dataflow", "agent", "tokens", "catalog", "components", "reactions"];
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

    /// **Relaie le bus d'un backend** qui vient de s'ouvrir : une tâche par
    /// sujet, qui finit quand le bus se ferme (le backend fermé ou rouvert,
    /// dont le nouveau bus est relayé à son tour).
    pub fn relayer(self: &Arc<Self>, backend: &str, bus: &EventBus) {
        for sujet in SUJETS {
            let mut rx = bus.subscribe(sujet);
            let (flux, backend, sujet) = (Arc::downgrade(self), backend.to_string(), sujet.to_string());
            crate::dataflow::rt::runtime().spawn(async move {
                loop {
                    let evenement = match rx.recv().await {
                        Ok(e) => e.to_json(),
                        Err(async_broadcast::RecvError::Overflowed(n)) => json!({ "kind": "EventsMissed", "count": n }),
                        Err(async_broadcast::RecvError::Closed) => break,
                    };
                    let Some(flux) = flux.upgrade() else { break };
                    flux.publier(&sujet, &backend, evenement);
                }
            });
        }
    }
}

// ─── L'aperçu d'un composant (marche 3, lot 3) ──────────────────────────────

const APERCU: &str = include_str!("ide/apercu.html");

/// `/ide/apercu/<backend>/<Composant>` : la page ; `…/modules.js` : ses
/// modules, enveloppés, puis l'entrée et le démarrage pour le registre.
fn apercu(host: &DaemonHost, path: &str) -> (u16, &'static str, String) {
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
            let rendu = host.with_backend_id(id, |b| {
                let prepared = b.prepared();
                let graphe = prepared.components_on_disk()?.module_graph(&prepared.directory, &nom, crate::components::Mode::Development)?;
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

/// Le script des modules d'un graphe, puis son entrée, son démarrage et ce qui
/// ne s'est pas résolu (dit à la console, refusé par le registre s'il s'exécute).
pub fn modules_js(g: &crate::components::modules::Graph) -> String {
    let mut js = g.script();
    js.push_str(&format!(
        "__rag3weaver.entree = {};
__rag3weaver.demarrage = {};
__rag3weaver.frontieres = {};
",
        serde_json::to_string(&g.entry).unwrap(),
        serde_json::to_string(&g.boot).unwrap(),
        serde_json::to_string(&g.modules.values().filter(|m| m.exports_only_components).map(|m| m.id.as_str()).collect::<Vec<_>>()).unwrap()
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
            let (statut, type_, corps) = apercu(host, p);
            let _ = request.respond(reponse(statut, type_, corps));
        }
        // Le graphe d'un outil, d'une réaction ou d'un composant (C1) : le
        // JSON de l'outil `graph`, pour tout backend ouvert par son id.
        ("GET", "/ide/graphe") => {
            let cible = json!({ "tool": param("tool"), "reaction": param("reaction"), "component": param("component") });
            let (statut, v) = match (param("backend"), crate::backend::GraphTarget::from_args(&cible)) {
                (None, _) => (400, json!({ "ok": false, "error": "graphe : il faut backend=<id>" })),
                (_, Err(e)) => (400, json!({ "ok": false, "error": e })),
                (Some(id), Ok(cible)) => match host.with_backend_id(&id, |b| b.prepared().graph_json(&cible)).and_then(|r| r) {
                    Ok(g) => (200, json!({ "ok": true, "result": g })),
                    Err(e) => (404, json!({ "ok": false, "error": e })),
                },
            };
            let _ = request.respond(json_(statut, &v));
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
        let (s, t, c) = apercu(&host, "/ide/apercu/abc/Fiche");
        assert_eq!(s, 200);
        assert!(t.starts_with("text/html") && c.contains("/ide/registre.js") && c.contains("modules.js"), "{c}");
        let (s, _, c) = apercu(&host, "/ide/apercu/abc/Fiche/modules.js");
        assert_eq!(s, 422, "{c}");
        assert!(c.contains("abc"), "{c}");
        assert_eq!(apercu(&host, "/ide/apercu/abc").0, 404);
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

    #[test]
    fn url_parameters_are_decoded() {
        assert_eq!(parametres("/ide/flux?sujets=files%2Cdataflow&depuis=12"), vec![("sujets".to_string(), "files,dataflow".to_string()), ("depuis".to_string(), "12".to_string())]);
        assert_eq!(decoder("a%2"), "a%2", "un % tronqué reste tel quel");
    }
}
