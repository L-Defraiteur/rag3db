//! **Le témoin de la page de l'IDE dans un vrai navigateur** (18 octobre 2026).
//!
//! Les autres témoins voient la page par ses appels, jamais rendue : c'est ce
//! qui a laissé toutes les vues cassées sans que personne le voie (`appeler()`
//! lisait la réponse un cran trop haut). Celui-ci ouvre les vues dans un
//! Chromium sans écran, sur un vrai hôte qui sert le gabarit `code`. Il exige
//! zéro erreur JavaScript à la console et un contenu attendu dans le DOM, puis
//! il édite et enregistre un fichier par la page.
//!
//! Le navigateur se pilote par le protocole DevTools sur un tube
//! (`--remote-debugging-pipe` : fd 3 en entrée, fd 4 en sortie, un message JSON
//! par fin `\0`) : ni websocket, ni dépendance.
//!
//! **Sans navigateur, le témoin se saute nommément** (une ligne `SAUTÉ` sur la
//! sortie d'erreur). Là où il doit jouer (la CI, luciepc),
//! `RAG3WEAVER_EXIGER_NAVIGATEUR=1` fait de cette absence un rouge : jamais
//! vert pour rien. `RAG3WEAVER_NAVIGATEUR` désigne un binaire précis.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

const CANDIDATS: &[&str] = &[
    "chromium",
    "chromium-browser",
    "google-chrome-stable",
    "google-chrome",
    "chrome",
    "headless_shell",
];

/// Le binaire du navigateur : `RAG3WEAVER_NAVIGATEUR`, sinon le premier des
/// candidats dans le PATH, sinon un Chromium de Playwright
/// (`~/.cache/ms-playwright/<version>/<dossier>/chrome`, le plus récent).
pub(crate) fn trouver_navigateur() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("RAG3WEAVER_NAVIGATEUR").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(p));
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    if let Some(p) = CANDIDATS
        .iter()
        .flat_map(|nom| std::env::split_paths(&path).map(move |d| d.join(nom)))
        .find(|p| p.is_file())
    {
        return Some(p);
    }
    let playwright = PathBuf::from(std::env::var_os("HOME")?).join(".cache/ms-playwright");
    let mut trouves: Vec<PathBuf> = std::fs::read_dir(playwright)
        .ok()?
        .flatten()
        .flat_map(|v| std::fs::read_dir(v.path()).into_iter().flatten().flatten())
        .flat_map(|d| ["chrome", "chrome-headless-shell", "headless_shell"].map(|n| d.path().join(n)))
        .filter(|p| p.is_file())
        .collect();
    trouves.sort();
    trouves.pop()
}

/// Le navigateur, ou `None` après la ligne `SAUTÉ` ; un rouge si
/// `RAG3WEAVER_EXIGER_NAVIGATEUR=1`. Un témoin navigateur se joue dans son
/// étape, sous la feature `navigateur` (`scripts/temoins_navigateur.sh`) ; joué
/// sans elle, il le dit (un rouge, pas un saut).
pub(crate) fn navigateur_ou_saut(temoin: &str) -> Option<PathBuf> {
    // Un témoin navigateur se joue dans son étape, pas dans la lib : sans la
    // feature, il est ignoré ; joué quand même (--ignored), il le dit.
    if !cfg!(feature = "navigateur") {
        panic!("{temoin} est un témoin navigateur : il se joue dans son étape (--features navigateur, scripts/temoins_navigateur.sh), pas dans la lib");
    }
    if let Some(p) = trouver_navigateur() {
        return Some(p);
    }
    let raison = format!(
        "aucun navigateur (RAG3WEAVER_NAVIGATEUR vide, aucun de {} dans le PATH, aucun Chromium de Playwright)",
        CANDIDATS.join(", ")
    );
    if std::env::var("RAG3WEAVER_EXIGER_NAVIGATEUR").as_deref() == Ok("1") {
        panic!("{temoin} : {raison}, et RAG3WEAVER_EXIGER_NAVIGATEUR=1 l'exige");
    }
    eprintln!("SAUTÉ {temoin} : {raison}");
    None
}

/// Un Chromium sans écran, attaché à un onglet, qui garde ce que la page dit à
/// la console.
pub(crate) struct Navigateur {
    enfant: Child,
    ecrire: std::fs::File,
    recus: Receiver<Value>,
    suivant: u64,
    session: String,
    /// Les erreurs vues : exceptions, `console.error`/`warn`, journal en erreur.
    pub(crate) erreurs: Vec<String>,
    _profil: tempfile::TempDir,
}

fn tube() -> (std::fs::File, std::fs::File) {
    use std::os::fd::FromRawFd;
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) }, 0, "pipe2");
    unsafe { (std::fs::File::from_raw_fd(fds[0]), std::fs::File::from_raw_fd(fds[1])) }
}

impl Navigateur {
    pub(crate) fn lancer(binaire: &std::path::Path) -> Navigateur {
        use std::os::fd::AsRawFd;
        let profil = tempfile::tempdir().unwrap();
        // Vers le navigateur (son fd 3), et depuis lui (son fd 4).
        let (entree_lue, entree) = tube();
        let (sortie, sortie_ecrite) = tube();
        let (a, b) = (entree_lue.as_raw_fd(), sortie_ecrite.as_raw_fd());
        let mut commande = Command::new(binaire);
        commande
            .args([
                "--headless",
                "--remote-debugging-pipe",
                "--no-first-run",
                "--no-default-browser-check",
                "--disable-gpu",
                "--disable-extensions",
                "--disable-background-networking",
                "--disable-sync",
                // Les runners de la CI refusent le bac à sable sans espace de noms.
                "--no-sandbox",
            ])
            .arg(format!("--user-data-dir={}", profil.path().display()))
            .arg("about:blank")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // dup2 rend les fd 3 et 4 au fils sans CLOEXEC. Les tubes passent d'abord
        // au-dessus de 10 : un tube qui serait déjà le fd 3 (ou 4) resterait sinon
        // en CLOEXEC (dup2 sur lui-même ne fait rien), ou serait écrasé par l'autre
        // — le binaire de test lancé seul l'a montré, le 19 octobre 2026.
        unsafe {
            commande.pre_exec(move || {
                let (a, b) = (libc::fcntl(a, libc::F_DUPFD, 10), libc::fcntl(b, libc::F_DUPFD, 10));
                if a < 0 || b < 0 || libc::dup2(a, 3) < 0 || libc::dup2(b, 4) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let enfant = commande.spawn().unwrap_or_else(|e| panic!("lancer {} : {e}", binaire.display()));
        drop((entree_lue, sortie_ecrite));
        // Un fil lit les messages : une attente se borne par recv_timeout.
        let (envoi, recus) = channel();
        std::thread::spawn(move || {
            let mut lecteur = BufReader::new(sortie);
            loop {
                let mut tampon = Vec::new();
                match lecteur.read_until(0, &mut tampon) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        tampon.pop();
                        if let Ok(v) = serde_json::from_slice::<Value>(&tampon) {
                            if envoi.send(v).is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });
        let mut n = Navigateur {
            enfant,
            ecrire: entree,
            recus,
            suivant: 0,
            session: String::new(),
            erreurs: Vec::new(),
            _profil: profil,
        };
        let cibles = n.commande("Target.getTargets", json!({}));
        let onglet = cibles["targetInfos"]
            .as_array()
            .and_then(|t| t.iter().find(|c| c["type"] == "page"))
            .unwrap_or_else(|| panic!("aucun onglet : {cibles}"))["targetId"]
            .clone();
        let attache = n.commande("Target.attachToTarget", json!({ "targetId": onglet, "flatten": true }));
        n.session = attache["sessionId"].as_str().unwrap().to_string();
        for domaine in ["Runtime.enable", "Log.enable", "Page.enable"] {
            n.commande(domaine, json!({}));
        }
        // Les appels en vol, relevés dès la création de chaque document : une
        // attente dépassée dit lesquels n'ont jamais répondu.
        n.commande(
            "Page.addScriptToEvaluateOnNewDocument",
            json!({ "source": "(() => { const vol = new Map(); let n = 0; window.__temoin_en_vol = vol; \
                const f = window.fetch; window.fetch = (...a) => { const k = ++n; \
                vol.set(k, `${a[0]} ${typeof a[1]?.body === 'string' ? a[1].body.slice(0, 300) : ''}`); \
                return f(...a).finally(() => vol.delete(k)); }; })()" }),
        );
        n
    }

    /// Une commande à l'onglet (au navigateur tant qu'aucune session), et sa
    /// réponse ; les événements reçus en route sont relevés.
    pub(crate) fn commande(&mut self, methode: &str, params: Value) -> Value {
        self.suivant += 1;
        let id = self.suivant;
        let mut m = json!({ "id": id, "method": methode, "params": params });
        if !self.session.is_empty() {
            m["sessionId"] = json!(self.session);
        }
        let mut octets = serde_json::to_vec(&m).unwrap();
        octets.push(0);
        self.ecrire.write_all(&octets).unwrap();
        let fin = Instant::now() + Duration::from_secs(30);
        loop {
            let v = self.recevoir(fin.saturating_duration_since(Instant::now()))
                .unwrap_or_else(|| panic!("{methode} : pas de réponse du navigateur en 30 s"));
            if v["id"] == id {
                if let Some(e) = v.get("error") {
                    panic!("{methode} : {e}");
                }
                return v["result"].clone();
            }
        }
    }

    fn recevoir(&mut self, delai: Duration) -> Option<Value> {
        match self.recus.recv_timeout(delai) {
            Ok(v) => {
                self.relever(&v);
                Some(v)
            }
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => panic!("le navigateur s'est fermé"),
        }
    }

    fn relever(&mut self, v: &Value) {
        let p = &v["params"];
        match v["method"].as_str() {
            Some("Runtime.exceptionThrown") => {
                let d = &p["exceptionDetails"];
                let texte = d["exception"]["description"].as_str().or(d["text"].as_str()).unwrap_or("?");
                self.erreurs.push(format!("exception : {texte} ({}:{})", d["url"].as_str().unwrap_or(""), d["lineNumber"]));
            }
            Some("Runtime.consoleAPICalled") if matches!(p["type"].as_str(), Some("error" | "warning" | "assert")) => {
                let args: Vec<String> = p["args"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|a| a["value"].as_str().map(str::to_string).or(a["description"].as_str().map(str::to_string)).unwrap_or_else(|| a.to_string()))
                    .collect();
                self.erreurs.push(format!("console.{} : {}", p["type"].as_str().unwrap(), args.join(" ")));
            }
            Some("Log.entryAdded") if p["entry"]["level"] == "error" => {
                let e = &p["entry"];
                self.erreurs.push(format!("journal : {} {}", e["text"].as_str().unwrap_or("?"), e["url"].as_str().unwrap_or("")));
            }
            _ => {}
        }
    }

    /// **Une touche au clavier**, par le protocole (keyDown puis keyUp) :
    /// `modificateurs` 2 = Ctrl. La page reçoit de vrais événements clavier.
    pub(crate) fn touche(&mut self, cle: &str, code: &str, virtuelle: u32, modificateurs: u32) {
        for genre in ["keyDown", "keyUp"] {
            self.commande(
                "Input.dispatchKeyEvent",
                json!({ "type": genre, "key": cle, "code": code, "windowsVirtualKeyCode": virtuelle, "modifiers": modificateurs }),
            );
        }
    }

    /// Le résultat d'une expression JavaScript (les promesses attendues).
    pub(crate) fn evaluer(&mut self, expression: &str) -> Value {
        let r = self.commande(
            "Runtime.evaluate",
            json!({ "expression": expression, "returnByValue": true, "awaitPromise": true }),
        );
        if let Some(e) = r.get("exceptionDetails") {
            panic!("évaluer {expression} : {e}");
        }
        r["result"]["value"].clone()
    }

    /// Va à une URL, et attend un nouveau document où `condition` est vraie.
    pub(crate) fn aller(&mut self, url: &str, condition: &str) {
        // La marque de l'ancien document : la condition ne se juge que sur le neuf.
        if !self.session.is_empty() {
            let _ = self.commande("Runtime.evaluate", json!({ "expression": "window.__temoin_ancien = true" }));
        }
        self.commande("Page.navigate", json!({ "url": url }));
        self.attendre(&format!("!window.__temoin_ancien && document.readyState !== 'loading' && ({condition})"), url);
    }

    /// Attend que `condition` soit vraie, 20 s au plus.
    pub(crate) fn attendre(&mut self, condition: &str, quoi: &str) {
        let fin = Instant::now() + Duration::from_secs(20);
        loop {
            let r = self.commande(
                "Runtime.evaluate",
                json!({ "expression": format!("(() => {{ try {{ return !!({condition}); }} catch (e) {{ return false; }} }})()"), "returnByValue": true }),
            );
            if r["result"]["value"] == true {
                return;
            }
            if Instant::now() > fin {
                let vue = self.evaluer("document.getElementById('vue')?.innerText ?? document.body?.innerText ?? ''");
                let en_vol = self.evaluer("[...(window.__temoin_en_vol ?? new Map()).values()]");
                panic!("{quoi} : « {condition} » jamais vraie en 20 s ; la vue : {vue} ; en vol : {en_vol} ; erreurs : {:?}", self.erreurs);
            }
            while self.recevoir(Duration::from_millis(100)).is_some() {}
        }
    }
}

impl Drop for Navigateur {
    fn drop(&mut self) {
        // Notre propre fils, par son pid.
        let _ = self.enfant.kill();
        let _ = self.enfant.wait();
    }
}

/// **Un composant à prévisualiser**, rangé dans le dossier du manifeste : un
/// faux React en CommonJS (le magasin n'a pas de vrai React hors réseau) dont
/// `react-dom/client` rend vraiment dans le DOM, et deux composants TSX, `Fiche`
/// qui importe `Bouton` et tient un `useState`.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
fn ranger_un_composant(dossier: &std::path::Path, paquets: &std::path::Path) {
    let paquet = |nom: &str, pj: Value, fichiers: &[(&str, &str)]| -> PathBuf {
        let d = paquets.join(nom);
        for (chemin, texte) in fichiers {
            let f = d.join(chemin);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, texte).unwrap();
        }
        std::fs::write(d.join("package.json"), pj.to_string()).unwrap();
        d
    };
    let react = paquet(
        "react",
        json!({ "name": "react", "version": "19.2.0", "exports": { ".": "./index.js", "./jsx-dev-runtime": "./jsx.js", "./jsx-runtime": "./jsx.js" } }),
        &[
            ("index.js", "exports.useState = function (v) { return [v, function () {}]; }; exports.createElement = function (type, props) { return { type: type, props: props || {} }; };"),
            ("jsx.js", "require('react'); exports.jsxDEV = function (type, props) { return { type: type, props: props || {} }; }; exports.jsx = exports.jsxDEV; exports.jsxs = exports.jsxDEV; exports.Fragment = 'F';"),
        ],
    );
    let dom = paquet(
        "react-dom",
        json!({ "name": "react-dom", "version": "19.2.0", "exports": { ".": "./index.js", "./client": "./client.js" }, "dependencies": { "react": "19.2.0" } }),
        &[
            ("index.js", "module.exports = {};"),
            ("client.js", "require('react');
function noeud(e) {
  if (e === null || e === undefined || e === false) return document.createTextNode('');
  if (typeof e !== 'object') return document.createTextNode(String(e));
  if (Array.isArray(e)) { var f = document.createDocumentFragment(); e.forEach(function (x) { f.append(noeud(x)); }); return f; }
  if (typeof e.type === 'function') return noeud(e.type(e.props));
  var el = e.type === 'F' ? document.createDocumentFragment() : document.createElement(e.type);
  if (e.props.className) el.className = e.props.className;
  var enfants = e.props.children;
  (Array.isArray(enfants) ? enfants : [enfants]).forEach(function (x) { el.append(noeud(x)); });
  return el;
}
exports.createRoot = function (racine) { return { render: function (e) { racine.replaceChildren(noeud(e)); } }; };"),
        ],
    );
    crate::components::add_lib(dossier, &react).unwrap();
    crate::components::add_lib(dossier, &dom).unwrap();
    let composants = dossier.join("components");
    std::fs::create_dir_all(&composants).unwrap();
    for (nom, fichier, source) in [
        ("Bouton", "bouton.tsx", "export function Bouton({ libelle }: { libelle: string }) { return <button>{libelle}</button>; }\n"),
        (
            "Fiche",
            "fiche.tsx",
            "import { useState } from \"react\";\nimport { Bouton } from \"./bouton\";\nexport function Fiche({ titre }: { titre: string }) {\n  const [n] = useState(3);\n  return <div className=\"fiche\"><h2>{titre}</h2><span>{n}</span><Bouton libelle=\"ok\" /></div>;\n}\n",
        ),
    ] {
        std::fs::write(composants.join(fichier), source).unwrap();
        std::fs::write(
            composants.join(format!("{}{}", fichier.split('.').next().unwrap(), crate::components::SUFFIX)),
            json!({ "name": nom, "source": fichier, "libs": { "react": "19.2.0" } }).to_string(),
        )
        .unwrap();
    }
}

/// **Les vues de l'IDE, rendues** : l'accueil, l'arbre, un fichier, un symbole,
/// la recherche et le graphe, chacune avec zéro erreur à la console et son
/// contenu attendu ; puis la vue d'un fichier se modifie et s'enregistre, et le
/// disque porte le texte tapé.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
#[test]
fn les_vues_de_l_ide_se_rendent_dans_un_vrai_navigateur() {
    let Some(binaire) = navigateur_ou_saut("les_vues_de_l_ide_se_rendent_dans_un_vrai_navigateur") else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let (base, arbre) = (d.path().join("base"), d.path().join("arbre"));
    std::fs::create_dir_all(&base).unwrap();
    std::fs::create_dir_all(arbre.join("src")).unwrap();
    std::fs::write(arbre.join("main.rs"), "fn main() {\n    depart();\n}\n\nfn depart() {\n    arrivee();\n}\n").unwrap();
    std::fs::write(arbre.join("src/arrivee.rs"), "pub fn arrivee() {\n    let _ = 1;\n}\n").unwrap();
    // Un scope dans un autre, pour le plan du fichier (outline) : la méthode sous son impl.
    std::fs::write(arbre.join("src/compteur.rs"), "pub struct Compteur;\n\nimpl Compteur {\n    pub fn inc(&self) {}\n}\n").unwrap();
    let gabarit = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
    let manifeste = crate::gabarits::jouable::manifeste_jouable(&gabarit, &base).unwrap();
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["workspace"]["root"] = json!(arbre.to_string_lossy());
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    ranger_un_composant(manifeste.parent().unwrap(), &d.path().join("paquets"));
    let manifeste = manifeste.to_string_lossy().to_string();

    let (adresse, fil) = super::tests::hote_garde("j");
    let appeler = |nom: &str, args: Value| -> Value {
        let mut r = crate::daemon::agent()
            .post(format!("http://{adresse}/ide/call"))
            .header("Cookie", "rag3weaver_ide=j")
            .header("content-type", "application/json")
            .send(json!({ "manifest": manifeste, "name": nom, "arguments": args }).to_string())
            .unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    // Le premier appel ouvre le backend et lance l'index (en plein texte seul
    // sans service d'embarquement : les vecteurs restent en dette) ; on attend
    // les définitions de `depart` pour que le symbole et le graphe aient de
    // quoi se montrer.
    let index = appeler("index", json!({ "confirm": true }));
    assert_eq!(index["ok"], true, "{index}");
    // La sortie d'`usages` porte son propre `result` (et un `status` quand
    // il n'y a pas d'index) : un cran sous la réponse du graphe.
    let debut = Instant::now();
    loop {
        let u = appeler("usages", json!({ "name": "depart", "format": "json" }));
        if u["ok"] == true && u["result"]["result"]["result"]["definitions"].as_array().is_some_and(|d| !d.is_empty()) {
            break;
        }
        assert!(debut.elapsed() < Duration::from_secs(60), "depart n'est pas indexé en 60 s : {u}");
        std::thread::sleep(Duration::from_millis(200));
    }
    let etat: Value = {
        let mut r = crate::daemon::agent()
            .get(format!("http://{adresse}/ide/etat"))
            .header("Cookie", "rag3weaver_ide=j")
            .call()
            .unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    let id = etat["backends"]
        .as_array()
        .and_then(|b| b.iter().find(|b| b["manifest"] == manifeste.as_str()))
        .unwrap_or_else(|| panic!("le backend n'est pas dans l'état : {etat}"))["id"]
        .as_str()
        .unwrap()
        .to_string();

    let mut n = Navigateur::lancer(&binaire);
    let racine = format!("http://{adresse}");
    // L'entrée par le jeton pose le cookie, et mène à l'accueil.
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), &format!("document.querySelector('#vue a[href=\"/ide/code/{id}/arbre/\"]')"));
    let vues: &[(&str, String, &str)] = &[
        ("l'accueil", format!("{racine}/ide/"), "document.querySelector('#vue h1')?.textContent === 'Backends'"),
        (
            "l'arbre",
            format!("{racine}/ide/code/{id}/arbre/"),
            "[...document.querySelectorAll('#vue li a')].map(a => a.textContent).join(',') === 'src/,main.rs'",
        ),
        (
            "un fichier",
            format!("{racine}/ide/code/{id}/fichier/main.rs"),
            "document.querySelector('#L5')?.innerText.includes('fn depart()') && document.querySelector('#vue a.id[href$=\"/symbole/arrivee\"]') && [...document.querySelectorAll('#vue aside a')].some(a => a.textContent === 'depart')",
        ),
        (
            "le plan d'un fichier",
            format!("{racine}/ide/code/{id}/fichier/src/compteur.rs"),
            // Le plan d'outline en json : `inc` sous son impl (une profondeur), vers sa ligne et
            // son symbole, et la provenance que l'outil dit.
            r##"(() => { const d = [...document.querySelectorAll('#plan .scope')].find(s => s.querySelector('a')?.textContent === 'inc'); return d && d.style.getPropertyValue('--profondeur').trim() === '1' && d.querySelector('a[href="#L4"]') && d.querySelector('a[href$="/symbole/inc"]') && document.querySelector('#plan-provenance')?.textContent.includes('pas par un compilateur'); })()"##,
        ),
        (
            "un symbole",
            format!("{racine}/ide/code/{id}/symbole/depart"),
            "[...document.querySelectorAll('#vue h2')].some(h => h.textContent === 'Définitions (1)') && document.querySelector('#vue a[href$=\"/fichier/main.rs#L5\"]')",
        ),
        (
            // La recherche dit ce qu'elle sait (pièce 3, docs/12-octobre-2026-recherche/06) :
            // le score et les scores par signal de chaque résultat, l'état de l'index en
            // tête, et la case « reclasser ». Le repli dense n'est pas exigé ici : il dépend
            // du service d'embarquement de l'environnement (vu le 25 octobre : la passe le
            // pose, et l'index avait ses vecteurs).
            "la recherche",
            format!("{racine}/ide/code/{id}/recherche?q=arrivee"),
            "[...document.querySelectorAll('#vue .resultat a')].some(a => a.textContent.startsWith('src/arrivee.rs')) && document.querySelector('#vue .resultat .score') && document.querySelector('#vue .resultat .signaux') && document.querySelector('#recherche-etat') && document.querySelector('#reclasser') && !document.querySelector('#reclasser').checked",
        ),
        (
            // Reclasser sans relecteur configuré : la page le dit en pied (les avertissements),
            // la case est cochée. Aucun relecteur n'est déclaré ici, quel que soit
            // l'environnement.
            "la recherche à reclasser",
            format!("{racine}/ide/code/{id}/recherche?q=arrivee&reclasser=1"),
            "document.querySelector('#reclasser')?.checked && document.querySelector('#vue .avertissements')?.textContent.includes('aucun reranker configuré')",
        ),
        (
            "la liste des graphes",
            format!("{racine}/ide/code/{id}/dag/"),
            "[...document.querySelectorAll('#vue li a')].some(a => a.textContent === 'usages')",
        ),
        (
            "le DAG d'un outil",
            format!("{racine}/ide/code/{id}/dag/tool/usages"),
            "document.querySelectorAll('#vue svg g.etape').length >= 1 && document.querySelector('#dag-etat')",
        ),
        (
            "le graphe",
            format!("{racine}/ide/code/{id}/graphe/depart"),
            "document.querySelectorAll('#vue svg g.noeud').length >= 2 && document.querySelector('#vue svg g.noeud.centre')?.textContent.includes('depart')",
        ),
    ];
    // L'aperçu d'un composant (marche 3, lot 3) : le registre, les modules, le
    // rendu dans le DOM, avec ses props prises à l'URL.
    let vues = [
        vues,
        &[(
            "l'aperçu d'un composant",
            format!("{racine}/ide/apercu/{id}/Fiche?props=%7B%22titre%22%3A%22Bonjour%22%7D"),
            "document.querySelector('#racine .fiche h2')?.textContent === 'Bonjour' && document.querySelector('#racine .fiche span')?.textContent === '3' && document.querySelector('#racine .fiche button')?.textContent === 'ok' && !document.querySelector('pre.erreur')",
        )],
    ]
    .concat();
    for (quoi, url, attendu) in &vues {
        n.aller(url, attendu);
        let alertes = n.evaluer("[...document.querySelectorAll('.alerte')].map(e => e.textContent)");
        assert_eq!(alertes, json!([]), "{quoi} montre une alerte");
        assert!(n.erreurs.is_empty(), "{quoi} : erreurs à la console : {:#?}", n.erreurs);
    }

    // L'édition (B1, page 05 §4), dans CodeMirror : Modifier, tout sélectionner et
    // taper au clavier, Ctrl-S ; le disque porte le texte tapé.
    n.aller(&format!("{racine}/ide/code/{id}/fichier/main.rs"), "document.querySelector('#L1')");
    n.evaluer("[...document.querySelectorAll('button')].find(b => b.textContent === 'Modifier').click()");
    n.attendre("document.querySelector('.edition .cm-content')?.innerText.includes('fn depart()')", "l'éditeur CodeMirror");
    let avis = |texte: &str| format!("document.querySelector('.barre .note')?.textContent === {}", json!(texte));
    let neuf = "fn main() {\n    depart();\n}\n\nfn depart() {}\n";
    n.evaluer("document.querySelector('.edition .cm-content').focus()");
    n.touche("a", "KeyA", 65, 2);
    n.commande("Input.insertText", json!({ "text": neuf }));
    n.attendre(&avis("modifié, non enregistré"), "l'avis de modification");
    n.touche("s", "KeyS", 83, 2);
    n.attendre(&avis("enregistré"), "l'enregistrement par Ctrl-S");
    assert_eq!(std::fs::read_to_string(arbre.join("main.rs")).unwrap(), neuf);

    // Le tampon n'est jamais remplacé : on tape encore, le fichier change sur le
    // disque (un autre éditeur) ; la page le dit et garde le tampon. Ctrl-S montre
    // la différence sans rien écrire ; « garder ma version » écrit le tampon.
    n.touche("End", "End", 35, 2);
    n.commande("Input.insertText", json!({ "text": "// tampon\n" }));
    n.attendre(&avis("modifié, non enregistré"), "la seconde modification");
    let disque = "fn main() {}\n// disque\n";
    std::fs::write(arbre.join("main.rs"), disque).unwrap();
    n.attendre("document.querySelector('.barre .note')?.textContent.startsWith('changé sur le disque')", "l'avis du changement sur le disque");
    n.attendre("document.querySelector('.edition .cm-content')?.innerText.includes('// tampon')", "le tampon gardé");
    n.touche("s", "KeyS", 83, 2);
    n.attendre("document.querySelector('.diff .moins') && document.querySelector('.diff .plus')", "la différence disque → tampon");
    assert_eq!(std::fs::read_to_string(arbre.join("main.rs")).unwrap(), disque, "rien n'est écrit sur un fichier changé");
    n.evaluer("[...document.querySelectorAll('button')].find(b => b.textContent.startsWith('Garder ma version')).click()");
    n.attendre(&avis("enregistré"), "l'écriture en connaissance de cause");
    assert_eq!(std::fs::read_to_string(arbre.join("main.rs")).unwrap(), format!("{neuf}// tampon\n"));
    assert!(n.erreurs.is_empty(), "l'édition : erreurs à la console : {:#?}", n.erreurs);

    // Le DAG vivant (C2) : la page montre le graphe de `usages` ; un appel de
    // l'outil s'annonce par sa cible, et ses nœuds s'allument à leur NodeRun.
    n.aller(
        &format!("{racine}/ide/code/{id}/dag/tool/usages"),
        "document.querySelectorAll('#vue svg g.etape').length >= 1 && document.getElementById('lien')?.textContent === 'flux : ouvert'",
    );
    let u = appeler("usages", json!({ "name": "depart", "format": "json" }));
    assert_eq!(u["ok"], true, "{u}");
    n.attendre(
        "document.querySelector('#dag-etat')?.textContent.includes(': fini en') \
         && document.querySelectorAll('#vue g.etape.passe').length >= 1 \
         && document.querySelectorAll('#vue g.etape.echec').length === 0",
        "le DAG allumé par le run de l'outil",
    );
    assert!(n.erreurs.is_empty(), "le DAG vivant : erreurs à la console : {:#?}", n.erreurs);

    // Les réactions ((b)) : la vue liste `reindex_on_change` ; un fichier
    // réécrit sur le disque est vu par la montre, et le run de la réaction
    // s'inscrit dans les runs récents, finit, et dit ce qu'il a écrit.
    n.aller(
        &format!("{racine}/ide/code/{id}/reactions/"),
        "[...document.querySelectorAll('#vue li a')].some(a => a.textContent === 'reindex_on_change') && document.getElementById('lien')?.textContent === 'flux : ouvert'",
    );
    std::fs::write(arbre.join("src/arrivee.rs"), "pub fn arrivee() {\n    let _ = 2;\n}\n").unwrap();
    n.attendre(
        "document.querySelector('#montre-vue')?.textContent.includes('src/arrivee.rs') \
         && [...document.querySelectorAll('#runs-recents tr[data-reaction=\"reindex_on_change\"] td.fin')].some(t => t.textContent.startsWith('fini'))",
        "la montre et le run de la réaction, vus dans la vue des réactions",
    );
    // Ce que le run a écrit, sous son run : la ré-indexation du fichier publie
    // sur `catalog` des écritures qui portent le run de la réaction (la pièce
    // 1 de la mémoire), et la vue les range sous lui.
    n.attendre(
        "[...document.querySelectorAll('#runs-recents tr[data-reaction=\"reindex_on_change\"]')].some(tr => tr.querySelector('td.fin')?.textContent.startsWith('fini') && tr.querySelector('td.ecrit li[data-entity]'))",
        "ce que le run de la réaction a écrit, rangé sous lui",
    );
    let ecrit = n.evaluer("[...document.querySelectorAll('#runs-recents td.ecrit li')].map(li => li.textContent)");
    eprintln!("le run de reindex_on_change a écrit : {ecrit}");
    assert!(n.erreurs.is_empty(), "la vue des réactions : erreurs à la console : {:#?}", n.erreurs);

    // **Arrêter un run depuis la page** (C4). Un outil lent — `wait_output` sur
    // un journal où rien ne viendra, une minute d'attente — appelé comme le
    // ferait un agent ; son DAG vivant montre le run en cours et le bouton
    // Arrêter. Le clic l'arrête par le même chemin que tout run : le run finit
    // arrêté dans l'IDE, l'appel rend « Arrêté », aucun nœud n'est en échec.
    n.aller(
        &format!("{racine}/ide/code/{id}/dag/tool/wait_output"),
        "document.querySelectorAll('#vue svg g.etape').length >= 1 && document.getElementById('lien')?.textContent === 'flux : ouvert'",
    );
    let journal = crate::dataflow::run_nodes::dossier_journaux().join(format!("temoin-c4-{}.log", std::process::id()));
    let lent = {
        let (adresse, manifeste, journal) = (adresse.clone(), manifeste.clone(), journal.to_string_lossy().to_string());
        std::thread::spawn(move || -> Value {
            let mut r = crate::daemon::agent()
                .post(format!("http://{adresse}/ide/call"))
                .header("Cookie", "rag3weaver_ide=j")
                .header("content-type", "application/json")
                .send(json!({ "manifest": manifeste, "name": "wait_output", "arguments": { "journal": journal, "pattern": "jamais", "timeout_s": 60 } }).to_string())
                .unwrap();
            serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
        })
    };
    n.attendre(
        "document.querySelector('#dag-etat')?.textContent.endsWith(': en cours') && !document.querySelector('#arreter-run').hidden",
        "le run lent en cours, et le bouton Arrêter",
    );
    let debut = Instant::now();
    n.evaluer("document.querySelector('#arreter-run').click()");
    n.attendre("document.querySelector('#dag-etat')?.textContent.includes(\"arrêté dans l'IDE\")", "le run arrêté dans l'IDE");
    let rendu = lent.join().unwrap();
    assert!(debut.elapsed() < Duration::from_secs(20), "l'arrêt a pris {:?}", debut.elapsed());
    assert_eq!(rendu["ok"], true, "{rendu}");
    assert!(rendu.to_string().contains("Arrêté"), "l'appel dit qu'il a été arrêté : {rendu}");
    assert_eq!(n.evaluer("document.querySelectorAll('#vue g.etape.echec').length"), json!(0), "un arrêt n'est pas un échec");
    assert!(n.evaluer("document.querySelector('#arreter-run').hidden") == json!(true), "le bouton reste après la fin");

    // Arrêter un run déjà fini : un refus nommé, et rien ne bouge.
    let etat = n.evaluer("document.querySelector('#dag-etat').textContent");
    let run = etat.as_str().unwrap().strip_prefix("run ").and_then(|r| r.split(' ').next()).unwrap().to_string();
    let refus: Value = {
        let mut r = crate::daemon::agent()
            .post(format!("http://{adresse}/ide/runs/arreter"))
            .header("Cookie", "rag3weaver_ide=j")
            .header("content-type", "application/json")
            .send(json!({ "backend": id, "run": run }).to_string())
            .unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    assert_eq!(refus["ok"], false, "{refus}");
    assert!(refus["error"].as_str().unwrap().contains(&format!("aucun run « {run} » en vol")), "{refus}");
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(n.evaluer("document.querySelector('#dag-etat').textContent"), etat, "un refus a fait bouger la page");
    assert!(n.erreurs.is_empty(), "l'arrêt depuis la page : erreurs à la console : {:#?}", n.erreurs);

    drop(n);
    let (statut, _) = super::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}

/// **L'IDE d'un vrai hôte, en lecture seule** (19 octobre 2026) : le même
/// parcours que [`les_vues_de_l_ide_se_rendent_dans_un_vrai_navigateur`],
/// contre un hôte en service et l'un de ses backends de code, sans rien
/// éditer. Le fichier et le symbole se choisissent seuls : un fichier indexé,
/// pas modifié depuis l'index, et l'un de ses scopes.
///
/// Ignoré par défaut ; se joue par
/// `RAG3WEAVER_TEMOIN_HOTE=127.0.0.1:7970 RAG3WEAVER_TEMOIN_MANIFESTE=<backend.json> cargo test --lib --features navigateur -- --ignored l_ide_d_un_vrai_hote`.
/// Le jeton est lu dans `RAG3WEAVER_TEMOIN_JETON_FICHIER` (par défaut
/// `~/.local/share/rag3weaver/daemon.token`), jamais écrit nulle part.
#[test]
#[ignore = "contre un hôte en service : RAG3WEAVER_TEMOIN_HOTE et RAG3WEAVER_TEMOIN_MANIFESTE"]
fn l_ide_d_un_vrai_hote_se_rend_en_lecture_seule() {
    let nom = "l_ide_d_un_vrai_hote_se_rend_en_lecture_seule";
    let (Ok(adresse), Ok(manifeste)) = (std::env::var("RAG3WEAVER_TEMOIN_HOTE"), std::env::var("RAG3WEAVER_TEMOIN_MANIFESTE")) else {
        eprintln!("SAUTÉ {nom} : RAG3WEAVER_TEMOIN_HOTE et RAG3WEAVER_TEMOIN_MANIFESTE manquent");
        return;
    };
    let Some(binaire) = navigateur_ou_saut(nom) else { return };
    let fichier_jeton = std::env::var_os("RAG3WEAVER_TEMOIN_JETON_FICHIER")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap()).join(".local/share/rag3weaver/daemon.token"));
    let jeton = std::fs::read_to_string(&fichier_jeton).unwrap_or_else(|e| panic!("{} : {e}", fichier_jeton.display())).trim().to_string();
    let cookie = format!("rag3weaver_ide={jeton}");
    let appeler = |outil: &str, args: Value| -> Value {
        let mut r = crate::daemon::agent()
            .post(format!("http://{adresse}/ide/call"))
            .header("Cookie", &cookie)
            .header("content-type", "application/json")
            .send(json!({ "manifest": manifeste, "name": outil, "arguments": args }).to_string())
            .unwrap();
        let v: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
        assert_eq!(v["ok"], true, "{outil} : {v}");
        // La sortie de l'outil, sous l'enveloppe d'une lecture du catalogue s'il y en a une.
        let s = v["result"]["result"].clone();
        if s.get("status").is_some() && s.get("result").is_some() && s.as_object().is_some_and(|o| o.len() == 2) {
            s["result"].clone()
        } else {
            s
        }
    };
    // Le fichier : un fichier de code indexé, pas modifié depuis l'index, de quelques dizaines de
    // lignes, sous src/ de préférence ; le premier dont un scope de code porte un nom d'identifiant.
    let liste = appeler("list_files", json!({ "path_prefix": "", "limit": 20000, "format": "json" }));
    let entrees = liste["entries"].as_array().unwrap_or_else(|| panic!("list_files : {liste}"));
    const CODE: &[&str] = &[".rs", ".cpp", ".h", ".hpp", ".cc", ".py", ".ts", ".tsx", ".js", ".go", ".java"];
    let bon = |e: &&Value| {
        e["indexed"] != false
            && e["stale"] != true
            && e["lines"].as_u64().is_none_or(|n| (30..2000).contains(&n))
            && e["path"].as_str().is_some_and(|p| CODE.iter().any(|x| p.ends_with(x)))
    };
    let mut candidats: Vec<&Value> = entrees.iter().filter(bon).collect();
    candidats.sort_by_key(|e| !e["path"].as_str().is_some_and(|p| p.starts_with("src/")));
    let (fichier, symbole) = candidats
        .iter()
        .take(40)
        .find_map(|e| {
            let chemin = e["path"].as_str()?.to_string();
            let lu = appeler("read_file", json!({ "path": chemin, "limit": 2000, "format": "json" }));
            let nom = lu["scopes"].as_array()?.iter().find_map(|x| {
                let n = x["name"].as_str()?;
                (x["scope_type"] != "texte_brut" && n.len() >= 4 && n.chars().all(|c| c.is_alphanumeric() || c == '_')).then(|| n.to_string())
            })?;
            Some((chemin, nom))
        })
        .unwrap_or_else(|| panic!("aucun fichier de code indexé, à jour et nommé parmi {} ({} candidats)", entrees.len(), candidats.len()));
    eprintln!("{nom} : {fichier}, symbole {symbole}");
    let etat: Value = {
        let mut r = crate::daemon::agent().get(format!("http://{adresse}/ide/etat")).header("Cookie", &cookie).call().unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    let id = etat["backends"]
        .as_array()
        .and_then(|b| b.iter().find(|b| b["manifest"] == manifeste.as_str()))
        .unwrap_or_else(|| panic!("le backend n'est pas dans l'état de l'hôte"))["id"]
        .as_str()
        .unwrap()
        .to_string();

    let mut n = Navigateur::lancer(&binaire);
    let racine = format!("http://{adresse}");
    n.aller(&format!("{racine}/ide/entrer?jeton={jeton}"), &format!("document.querySelector('#vue a[href=\"/ide/code/{id}/arbre/\"]')"));
    let fichier_url: String = fichier.split('/').map(|p| p.replace('%', "%25").replace('#', "%23").replace('?', "%3F").replace(' ', "%20")).collect::<Vec<_>>().join("/");
    let vues: Vec<(&str, String, String)> = vec![
        ("l'accueil", format!("{racine}/ide/"), "document.querySelector('#vue h1')?.textContent === 'Backends'".into()),
        ("l'arbre", format!("{racine}/ide/code/{id}/arbre/"), "document.querySelectorAll('#vue li a').length > 0".into()),
        (
            "un fichier",
            format!("{racine}/ide/code/{id}/fichier/{fichier_url}"),
            format!("document.querySelector('#L1') && [...document.querySelectorAll('#vue aside a')].some(a => a.textContent === {})", json!(symbole)),
        ),
        (
            "un symbole",
            format!("{racine}/ide/code/{id}/symbole/{symbole}"),
            "[...document.querySelectorAll('#vue h2')].some(h => /^Définitions \\([1-9]/.test(h.textContent))".into(),
        ),
        ("la recherche", format!("{racine}/ide/code/{id}/recherche?q={symbole}"), "document.querySelectorAll('#vue .resultat').length > 0".into()),
        ("le graphe", format!("{racine}/ide/code/{id}/graphe/{symbole}"), "document.querySelector('#vue svg g.noeud.centre')".into()),
    ];
    for (quoi, url, attendu) in &vues {
        n.aller(url, attendu);
        let alertes = n.evaluer("[...document.querySelectorAll('.alerte')].map(e => e.textContent)");
        assert_eq!(alertes, json!([]), "{quoi} montre une alerte");
        assert!(n.erreurs.is_empty(), "{quoi} : erreurs à la console : {:#?}", n.erreurs);
        eprintln!("{nom} : {quoi} rendu");
    }
}
