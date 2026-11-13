//! **Les images du README** (c7 et Lucie, 13 novembre 2026 : le README sur le modèle de
//! CocoIndex, huit cartes de démos, chacune avec une vraie image). Ce n'est pas un témoin : un
//! générateur, `#[ignore]`, qu'aucune passe ne joue, et qui refait les images à chaque version.
//!
//! ```sh
//! RAG3WEAVER_EXIGER_NAVIGATEUR=1 cargo test --lib --features rag3db-native,code,daemon,navigateur \
//!   -- --ignored daemon_host::captures_du_readme
//! ```
//!
//! Les vues de l'IDE se capturent sur un hôte de test, un faux modèle derrière le chat (les
//! scènes des témoins navigateur). Ce qui n'a pas de vue dans l'IDE est la **sortie réelle d'une
//! commande**, gardée telle quelle sous `docs/images/sorties/<nom>.txt` (sa première ligne est
//! la commande ; `docs/images/README.md` dit d'où vient chacune), rendue en image de terminal par
//! le même navigateur. Rien n'est dessiné à la main.

use super::chat::ChatModel;
use super::navigateur::{navigateur_ou_saut, Navigateur};
use crate::llm::{CallbackLlm, Llm, MockLlm, Turn};
use base64::Engine;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const LARGEUR: u32 = 1440;
const HAUTEUR: u32 = 900;

fn dossier_des_images() -> PathBuf {
    std::env::var_os("RAG3WEAVER_CAPTURES").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/images"))
}

/// L'écran du navigateur, en PNG, sous `nom`.
fn photo(n: &mut Navigateur, nom: &str) {
    let r = n.commande("Page.captureScreenshot", json!({ "format": "png" }));
    let donnees = r["data"].as_str().unwrap_or_else(|| panic!("Page.captureScreenshot : {r}"));
    let png = base64::engine::general_purpose::STANDARD.decode(donnees).expect("le PNG en base64");
    let chemin = dossier_des_images().join(nom);
    std::fs::create_dir_all(chemin.parent().unwrap()).unwrap();
    std::fs::write(&chemin, png).unwrap();
    eprintln!("[captures] {}", chemin.display());
}

/// **Une sortie réelle, en image de terminal** : `sorties/<nom>.txt`, sa première ligne la
/// commande (affichée après `$`), le reste sa sortie telle quelle.
fn terminal(n: &mut Navigateur, nom: &str) {
    let texte = std::fs::read_to_string(dossier_des_images().join("sorties").join(format!("{nom}.txt"))).unwrap_or_else(|e| panic!("sorties/{nom}.txt : {e}"));
    let (commande, sortie) = texte.split_once('\n').unwrap_or((&texte, ""));
    let echapper = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let lignes = sortie.lines().count() as u32 + 1;
    let page = format!(
        "<!doctype html><meta charset=utf-8><body style=\"margin:0;background:#1e1f24\"><pre style=\"margin:0;padding:22px 26px;\
         font:15px/1.5 'DejaVu Sans Mono',monospace;color:#d8dae0;white-space:pre-wrap;word-break:break-word\">\
         <span style=\"color:#7fb3ff\">$ </span><span style=\"color:#f2f3f5\">{}</span>\n{}</pre>",
        echapper(commande),
        echapper(sortie.trim_end())
    );
    // Une première hauteur d'après les lignes, puis celle que la page mesure (les lignes
    // longues se replient) : l'image tient toute la sortie, sans barre de défilement.
    let hauteur = (lignes * 23 + 60).clamp(160, 1400);
    n.commande("Emulation.setDeviceMetricsOverride", json!({ "width": LARGEUR, "height": hauteur, "deviceScaleFactor": 1, "mobile": false }));
    let url = format!("data:text/html;base64,{}", base64::engine::general_purpose::STANDARD.encode(page));
    n.aller(&url, "document.readyState === 'complete'");
    let mesuree = n.evaluer("document.documentElement.scrollHeight").as_u64().unwrap_or(hauteur as u64) as u32;
    n.commande("Emulation.setDeviceMetricsOverride", json!({ "width": LARGEUR, "height": mesuree.max(hauteur), "deviceScaleFactor": 1, "mobile": false }));
    photo(n, &format!("{nom}.png"));
}

/// Un atelier de test : le gabarit livré, sa racine sous `d/rw`, un hôte garde sur un port à
/// lui avec `model` derrière le chat ; rend l'adresse, le fil de l'hôte et l'id de l'atelier.
fn atelier(d: &Path, model: ChatModel) -> (String, std::thread::JoinHandle<()>, String) {
    let racine = d.join("rw");
    std::fs::create_dir_all(&racine).unwrap();
    let gabarits = d.join("atelier/templates");
    std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
    crate::code_tree::copy(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
    let manifeste = gabarits.join("backends/atelier/backend.json");
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["workspace"]["root"] = json!(racine.to_string_lossy());
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let (runs, chats, taches) = (d.join("runs"), d.join("chats"), d.join("taches"));
    let racines = crate::code_tree::Roots { config: d.join("config"), data: d.join("data") };
    let (adresse, fil) = super::tests::hote_garde_avec("j", move |h| {
        h.with_racines_de_code(racines).with_runs_root(runs).with_chats_root(chats).with_racine_des_taches(taches).with_chat_model(model)
    });
    let racine_url = format!("http://{adresse}");
    let mut r = crate::daemon::agent_de_temoin()
        .post(format!("{racine_url}/ide/describe"))
        .header("Cookie", "rag3weaver_ide=j")
        .header("content-type", "application/json")
        .send(json!({ "manifest": manifeste }).to_string())
        .unwrap();
    let decrit: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
    assert_eq!(decrit["ok"], true, "{decrit}");
    let mut r = crate::daemon::agent_de_temoin().get(format!("{racine_url}/ide/etat")).header("Cookie", "rag3weaver_ide=j").call().unwrap();
    let etat: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
    let canon = manifeste.canonicalize().unwrap();
    let id = etat["backends"].as_array().and_then(|bs| bs.iter().find(|b| b["manifest"].as_str().map(Path::new) == Some(canon.as_path()))).and_then(|b| b["id"].as_str()).unwrap().to_string();
    (adresse, fil, id)
}

/// Le chat de l'atelier ouvert dans la page, le flux ouvert.
fn ouvrir_le_chat(n: &mut Navigateur, adresse: &str, id: &str) {
    n.commande("Emulation.setDeviceMetricsOverride", json!({ "width": LARGEUR, "height": HAUTEUR, "deviceScaleFactor": 1, "mobile": false }));
    n.aller(&format!("http://{adresse}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(
        &format!("http://{adresse}/ide/code/{id}/chat"),
        "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'",
    );
}

fn envoyer(n: &mut Navigateur, message: &str) {
    n.evaluer(&format!("document.querySelector('#message').value = {}; document.querySelector('#send').click()", json!(message)));
    n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le tour");
}

fn arreter(adresse: &str, fil: std::thread::JoinHandle<()>) {
    let (statut, _) = super::tests::demander("POST", &format!("http://{adresse}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}

/// Un faux modèle qui joue, pour chaque message, ses appels d'outils dans l'ordre, puis répond.
fn modele(fenetre: usize, script: Vec<(&'static str, Vec<(&'static str, Value)>, &'static str)>) -> ChatModel {
    let script = Arc::new(script);
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        let script = script.clone();
        Ok(Arc::new(CallbackLlm::new("demo-model", fenetre, move |turns: &[Turn], opts, sink| {
            let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
            let message = turns[depuis].content.clone();
            let faits = turns[depuis..].iter().filter(|t| t.role == "tool").count();
            let (appels, fin) = script.iter().find(|(m, _, _)| message.starts_with(m)).map(|(_, a, f)| (a.clone(), *f)).unwrap_or((Vec::new(), "Done."));
            match appels.get(faits) {
                Some((nom, args)) => MockLlm { context_len: fenetre, ..MockLlm::new("") }.with_tool_calls(vec![(nom.to_string(), args.to_string())]).generate(turns, opts, sink),
                None => MockLlm { context_len: fenetre, ..MockLlm::new(fin) }.generate(turns, opts, sink),
            }
        })))
    })
}

/// Un petit dépôt git, sur sa branche `feature/parser`.
fn depot(dans: &Path) -> PathBuf {
    let depot = dans.join("demo-service");
    std::fs::create_dir_all(depot.join("src")).unwrap();
    std::fs::write(depot.join("src/lib.rs"), "pub mod config;\n\npub fn start() {\n    let c = config::parse_config(\"app.toml\");\n    println!(\"{}\", c.raw);\n}\n").unwrap();
    std::fs::write(depot.join("src/config.rs"), "pub struct Config {\n    pub raw: String,\n}\n\npub fn parse_config(path: &str) -> Config {\n    Config { raw: std::fs::read_to_string(path).unwrap_or_default() }\n}\n").unwrap();
    for args in [&["init", "-q", "-b", "feature/parser"][..], &["config", "user.name", "Demo User"], &["config", "user.email", "demo@example.org"], &["add", "-A"], &["commit", "-q", "-m", "demo"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(&depot).status().unwrap().success(), "git {args:?}");
    }
    depot
}

#[test]
#[ignore = "génère les images du README : RAG3WEAVER_EXIGER_NAVIGATEUR=1 cargo test --lib --features rag3db-native,code,daemon,navigateur -- --ignored daemon_host::captures_du_readme"]
fn captures_du_readme() {
    let Some(binaire) = navigateur_ou_saut("captures_du_readme") else {
        return;
    };

    // 1. L'index de code par worktree : l'atelier attache un dépôt, la fin de l'index se dit
    //    dans le chat, le dépôt et sa branche paraissent sous « Dépôts ».
    {
        let d = tempfile::tempdir().unwrap();
        let depot = depot(&d.path().join("rw"));
        let chemin: &'static str = Box::leak(depot.display().to_string().into_boxed_str());
        let model = modele(8192, vec![("Attach", vec![("attach_worktree", json!({ "path": chemin }))], "Attached; the index runs in the background and the sidebar shows it.")]);
        let (adresse, fil, id) = atelier(d.path(), model);
        let mut n = Navigateur::lancer(&binaire);
        ouvrir_le_chat(&mut n, &adresse, &id);
        n.evaluer("(() => { const s = document.querySelector('#vue-barre'); const set = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set; set.call(s, 'repo'); s.dispatchEvent(new Event('change', { bubbles: true })); })()");
        envoyer(&mut n, &format!("Attach the worktree {chemin}"));
        n.attendre("!!document.querySelector('.fin-d-index')", "la fin de l'index dans le chat");
        n.attendre("!!document.querySelector('#arbre-depots .depot[data-depot=\"demo-service\"] .espace')", "le dépôt sous « Dépôts »");
        std::thread::sleep(std::time::Duration::from_millis(500));
        photo(&mut n, "ide-index-par-worktree.png");
        drop(n);
        arreter(&adresse, fil);
    }

    // 2. L'atelier derrière une carte d'accord : `mkdir` écrit, la porte le met en attente,
    //    la carte glisse sous l'appel.
    {
        let d = tempfile::tempdir().unwrap();
        let model = modele(8192, vec![("Create", vec![("run_command", json!({ "command": "mkdir build" }))], "The command waits for your approval on the card below.")]);
        let (adresse, fil, id) = atelier(d.path(), model);
        let mut n = Navigateur::lancer(&binaire);
        ouvrir_le_chat(&mut n, &adresse, &id);
        envoyer(&mut n, "Create a build directory");
        n.attendre("!!document.querySelector('.carte-accord button.accorder')", "la carte d'accord");
        n.evaluer("document.querySelector('.carte-accord')?.scrollIntoView({ block: 'center' })");
        std::thread::sleep(std::time::Duration::from_millis(300));
        photo(&mut n, "ide-carte-d-accord.png");
        drop(n);
        arreter(&adresse, fil);
    }

    // 3. Le chat et la jauge du contexte : trois lectures de 200 lignes sous un modèle d'un
    //    million de jetons ; la jauge écrit son pourcentage.
    {
        let d = tempfile::tempdir().unwrap();
        let racine = d.path().join("rw");
        std::fs::create_dir_all(&racine).unwrap();
        for f in ["config.rs", "server.rs", "routes.rs"] {
            let texte: String = (1..=200).map(|i| format!("// line {i} of {f}: request handling, configuration and routing\n")).collect();
            std::fs::write(racine.join(f), texte).unwrap();
        }
        let model = modele(
            1_000_000,
            vec![("Read", vec![("read_file", json!({ "path": "config.rs" })), ("read_file", json!({ "path": "server.rs" })), ("read_file", json!({ "path": "routes.rs" }))], "I read the three files, 200 lines each.")],
        );
        let (adresse, fil, id) = atelier(d.path(), model);
        let mut n = Navigateur::lancer(&binaire);
        ouvrir_le_chat(&mut n, &adresse, &id);
        envoyer(&mut n, "Read config.rs, server.rs and routes.rs");
        n.attendre(&format!("Number(document.querySelector('.conversation[data-backend=\"{id}\"] .usage')?.dataset.used) > 0"), "la jauge");
        std::thread::sleep(std::time::Duration::from_millis(500));
        photo(&mut n, "ide-chat-et-jauge.png");
        drop(n);
        arreter(&adresse, fil);
    }

    // 4. Les sorties réelles des commandes, en images de terminal.
    let mut n = Navigateur::lancer(&binaire);
    for nom in ["trace-d-un-index", "memoire-recall", "juge-local", "postgres", "lucivy"] {
        terminal(&mut n, nom);
    }
    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
}
