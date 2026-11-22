//! **A closed task's chat is put away, not called an orphan** (the tablet, 22 Nov 2026: « 11 session(s) orpheline(s) »,
//! each the chat of a task whose tree was removed when it closed). At the host's start, a chat whose manifest is gone
//! and lay under the tasks' root moves to `chats/closed/`, said in one line; only a chat whose manifest vanished from
//! elsewhere stays an orphan. A module of its own.

use super::DaemonHost;
use serde_json::json;

fn chat(dir: &std::path::Path, id: &str, manifest: &std::path::Path) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join(format!("{id}.json")), json!({ "manifest": manifest, "session": { "turns": [] } }).to_string()).unwrap();
}

#[test]
fn a_closed_tasks_chat_is_put_away_and_only_a_real_orphan_is_said() {
    let d = tempfile::tempdir().unwrap();
    let (taches, chats) = (d.path().join("taches"), d.path().join("chats"));
    let host = DaemonHost::new().with_racine_des_taches(taches.clone()).with_chats_root(chats.clone());
    chat(&chats, "fermee", &taches.join("p/doc/arbre/templates/backends/code/backend.json"));
    chat(&chats, "perdue", &d.path().join("ailleurs/backend.json"));
    let lignes = host.reread_chats();
    let orphelines: Vec<String> =
        host.orphan_chats.lock().unwrap().iter().filter_map(|o| o["backend"].as_str().map(str::to_string)).collect();
    assert_eq!(orphelines, ["perdue"], "only a chat lost from elsewhere is an orphan: {lignes:?}");
    assert!(chats.join("closed/fermee.json").is_file() && !chats.join("fermee.json").exists(), "put away: {lignes:?}");
    assert!(lignes.iter().any(|l| l.contains("fermee") && l.contains("rangée")), "said: {lignes:?}");
}
