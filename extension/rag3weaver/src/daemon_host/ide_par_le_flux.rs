//! **L'IDE par le flux, pas par relecture** (Lucie, 8 novembre 2026 au soir : « que ces
//! trucs-là fonctionnent par SSE, pas par polling, voire rendre générique quand on a besoin
//! d'un SSE » ; c7, 9 novembre). Le témoin compte ce que la page demande à l'hôte hors du flux :
//! rien pendant une minute d'inactivité, et, pour un tour de chat, le seul geste qui l'envoie.

use super::chat::ChatModel;
use super::navigateur::{navigateur_ou_saut, Navigateur};
use crate::llm::{CallbackLlm, Llm, MockLlm, Turn};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// **Ce que la page demande à l'hôte hors du flux** : une minute sans rien faire, puis un tour
/// de chat qui lit un fichier. Les requêtes se relèvent en enveloppant `fetch` (le flux, une
/// `EventSource`, n'y passe pas) : pendant l'inactivité, aucune ; pendant le tour et les cinq
/// secondes qui suivent, le seul `POST /ide/chat` qui l'envoie.
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_ide_asks_the_host_nothing_while_idle_and_only_its_gesture_for_a_turn() {
    const TEMOIN: &str = "the_ide_asks_the_host_nothing_while_idle_and_only_its_gesture_for_a_turn";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let racine = d.path().join("rw");
    std::fs::create_dir_all(&racine).unwrap();
    std::fs::write(racine.join("note.txt"), "une note\n").unwrap();
    let gabarits = d.path().join("atelier/templates");
    std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
    crate::code_tree::copy(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
    let manifeste = gabarits.join("backends/atelier/backend.json");
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["workspace"]["root"] = json!(racine.to_string_lossy());
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
            let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
            if turns[depuis..].iter().any(|t| t.role == "tool") {
                MockLlm::new("lu").generate(turns, opts, sink)
            } else {
                MockLlm::new("").with_tool_calls(vec![("read_file".to_string(), json!({ "path": "note.txt" }).to_string())]).generate(turns, opts, sink)
            }
        })))
    });
    let (runs, chats, taches) = (d.path().join("runs"), d.path().join("chats"), d.path().join("taches"));
    let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
    let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
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

    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine_url}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    let prete = "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'";
    n.aller(&format!("{racine_url}/ide/code/{id}/chat"), prete);
    // Le chargement fini (ses lectures faites), chaque requête suivante se relève.
    std::thread::sleep(std::time::Duration::from_secs(2));
    n.evaluer(
        "window.__req = []; const f = window.fetch.bind(window); window.fetch = (u, o) => { window.__req.push(((o && o.method) || 'GET') + ' ' + String(u).replace(location.origin, '').split('?')[0]); return f(u, o); }",
    );
    std::thread::sleep(std::time::Duration::from_secs(60));
    let au_repos = n.evaluer("window.__req.splice(0)");
    n.evaluer("document.querySelector('#message').value = 'lis la note'; document.querySelector('#send').click()");
    n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le tour");
    std::thread::sleep(std::time::Duration::from_secs(5));
    let pour_un_tour = n.evaluer("window.__req.splice(0)");
    assert_eq!(au_repos, json!([]), "une minute d'inactivité : rien hors du flux");
    assert_eq!(pour_un_tour, json!(["POST /ide/chat"]), "un tour : son seul geste, le reste par le flux");

    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
    drop(n);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine_url}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}

/// **Un `RunStarted` en retard n'ouvre pas de bulle fantôme** (relevé par everything declarative
/// le 13 novembre 2026 dans les rouges « status jamais fini » de l'étape navigateur) : sous charge,
/// l'événement de début d'un tour envoyé par la page peut arriver après la réponse de son `POST`.
/// La page le prenait pour un tour venu d'ailleurs, ouvrait une bulle vivante, et rien ne la
/// fermait. Rejoué ici après la fin du tour : le statut reste « fini », aucune bulle en cours.
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn a_late_run_started_of_a_finished_turn_opens_no_ghost_bubble() {
    const TEMOIN: &str = "a_late_run_started_of_a_finished_turn_opens_no_ghost_bubble";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let racine = d.path().join("rw");
    std::fs::create_dir_all(&racine).unwrap();
    let gabarits = d.path().join("atelier/templates");
    std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
    crate::code_tree::copy(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
    let manifeste = gabarits.join("backends/atelier/backend.json");
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["workspace"]["root"] = json!(racine.to_string_lossy());
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| MockLlm::new("fait").generate(turns, opts, sink))))
    });
    let (runs, chats, taches) = (d.path().join("runs"), d.path().join("chats"), d.path().join("taches"));
    let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
    let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
        h.with_racines_de_code(racines).with_runs_root(runs).with_chats_root(chats).with_racine_des_taches(taches).with_chat_model(model)
    });
    let racine_url = format!("http://{adresse}");
    let lire = |route: &str| -> Value {
        let mut r = crate::daemon::agent_de_temoin().get(format!("{racine_url}{route}")).header("Cookie", "rag3weaver_ide=j").call().unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    let mut r = crate::daemon::agent_de_temoin()
        .post(format!("{racine_url}/ide/describe"))
        .header("Cookie", "rag3weaver_ide=j")
        .header("content-type", "application/json")
        .send(json!({ "manifest": manifeste }).to_string())
        .unwrap();
    let decrit: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
    assert_eq!(decrit["ok"], true, "{decrit}");
    let canon = manifeste.canonicalize().unwrap();
    let etat = lire("/ide/etat");
    let id = etat["backends"].as_array().and_then(|bs| bs.iter().find(|b| b["manifest"].as_str().map(Path::new) == Some(canon.as_path()))).and_then(|b| b["id"].as_str()).unwrap().to_string();

    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine_url}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    let prete = "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'";
    n.aller(&format!("{racine_url}/ide/code/{id}/chat"), prete);
    n.evaluer("document.querySelector('#message').value = 'bonjour'; document.querySelector('#send').click()");
    n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le tour");
    let run = lire(&format!("/ide/chat?backend={id}"))["result"]["usage"]["tour_run"].as_str().expect("le run du tour fini").to_string();

    // Le RunStarted de ce run, arrivé en retard par le flux.
    let evenement = json!({ "sujet": "agent", "backend": manifeste.display().to_string(), "evenement": { "kind": "RunStarted", "run_kind": "agent", "name": format!("chat:{id}"), "run": run } });
    n.evaluer(&format!("window.__ide.flux.onmessage(new MessageEvent('message', {{ data: {} }}))", json!(evenement.to_string())));
    // Et un appel d'outil de ce run, relayé par le bus après l'état de fin poussé : il remettait
    // le statut au nom de l'outil (le rouge de la carte de chemin sous les 40, 13 novembre).
    let outil = json!({ "sujet": "agent", "backend": manifeste.display().to_string(), "evenement": { "kind": "ToolCallStarted", "run": run, "tool": "read_file", "call_id": "x" } });
    n.evaluer(&format!("window.__ide.flux.onmessage(new MessageEvent('message', {{ data: {} }}))", json!(outil.to_string())));
    std::thread::sleep(std::time::Duration::from_millis(1500));
    let statut = n.evaluer("document.querySelector('#status')?.textContent");
    let fantomes = n.evaluer("document.querySelectorAll('.message.en-cours').length");
    assert_eq!(statut, json!("fini"), "le statut reste « fini »");
    assert_eq!(fantomes, json!(0), "aucune bulle en cours pour un tour déjà fini");

    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
    drop(n);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine_url}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}
