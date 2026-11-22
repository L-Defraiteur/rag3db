//! **The chat links only a path it resolved** (Lucie, 22 November 2026, at her first trial: a file the agent listed,
//! `depots/rag3db.queries_via.json`, was a link to « ReadFileNode: no such file in worktree:… » — « il me sort d'où le
//! lien alors ? pourquoi ça s'affiche comme un lien si nous on sait pas résoudre ? »). A cited path is a link only if it
//! exists in the files of the repository; another stays text. A file that exists nowhere is said in human words.

use super::*;

#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_chat_links_only_a_path_that_exists() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binary) = navigateur_ou_saut("the_chat_links_only_a_path_that_exists") else {
        return;
    };
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-modele", 8192, |turns, opts, sink| {
            MockLlm::new("Le code est dans `src/present.rs:1` ; la liste est dans `depots/rag3db.queries_via.json`.").generate(turns, opts, sink)
        })))
    });
    let (d, h, m, id) = banc_dans("atelier", model.clone(), |_| {});
    std::fs::create_dir_all(d.path().join("code/src")).unwrap();
    std::fs::write(d.path().join("code/src/present.rs"), "pub fn present() {}\n").unwrap();
    drop(h);
    let (chats, tasks, runs) = (d.path().join("chats"), d.path().join("taches"), d.path().join("runs"));
    let (address, thread) = crate::daemon_host::tests::hote_garde_avec("j", move |h| h.with_racine_des_taches(tasks).with_chats_root(chats).with_runs_root(runs).with_chat_model(model));
    let base = format!("http://{address}");
    let mut r = crate::daemon::agent_de_temoin()
        .post(format!("{base}/ide/describe"))
        .header("Cookie", "rag3weaver_ide=j")
        .header("content-type", "application/json")
        .send(json!({ "manifest": m }).to_string())
        .unwrap();
    let v: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
    assert_eq!(v["ok"], true, "{v}");
    let mut n = Navigateur::lancer(&binary);
    n.aller(&format!("{base}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    let ready = "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'";
    n.aller(&format!("{base}/ide/code/{id}/chat"), ready);
    n.evaluer("document.querySelector('#message').value = 'où est le code ?'; document.querySelector('#send').click()");
    n.attendre("document.querySelector('#status')?.textContent === 'fini'", "the turn");
    n.attendre("!!document.querySelector('article.message.assistant a.code-ref[data-path=\"src/present.rs\"]')", "the existing path is a link");
    let missing = n.evaluer("!!document.querySelector('article.message.assistant a.code-ref[data-path=\"depots/rag3db.queries_via.json\"]')");
    assert_eq!(missing, json!(false), "a path that does not exist is not a link");
    let text = n.evaluer("[...document.querySelectorAll('article.message.assistant code')].some(c => c.textContent === 'depots/rag3db.queries_via.json' && !c.closest('a'))");
    assert_eq!(text, json!(true), "the missing path stays text");
    n.aller(&format!("{base}/ide/code/{id}/fichier/depots/rag3db.queries_via.json"), "!!document.querySelector('[role=alert]')");
    let said = n.evaluer("document.querySelector('[role=alert]').innerText").as_str().unwrap_or_default().to_string();
    assert!(said.contains("n'existe pas") && !said.contains("ReadFileNode"), "a missing file is said in human words: {said}");
    drop(n);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}
