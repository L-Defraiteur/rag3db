//! **Opening a folder poses the atelier when it is absent** (the night of 21 November 2026, lot 2 after the views): the
//! witness of the lot `atelier-pose`, in a file of its own.

use super::*;
use crate::daemon_host::chat::ChatModel;
use crate::daemon_host::chat_qui_code::hote;
use crate::llm::{Llm, MockLlm};
use std::sync::Arc;

/// **Opening a folder on a host without an atelier poses it, then attaches the folder**: until now `ouvrir_un_dossier`
/// refused, « l'atelier n'est pas posé ; `rag3weaver-backend atelier` le pose », and Lucie's first step in the IDE
/// stopped there. The atelier is posed as the command poses it (the delivered template, its base under the data
/// folder), its root the space of worktrees.
#[test]
fn opening_a_folder_poses_the_atelier_when_it_is_absent() {
    let d = tempfile::tempdir().unwrap();
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Ok(Arc::new(MockLlm::new(""))) });
    let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
    let host = Arc::new(hote(d.path(), model).with_racines_de_code(racines.clone()).with_runs_root(d.path().join("runs")));
    host.se_connaitre();
    let (_h, espace, _) = hote_d_espace(&d.path().join("e"));
    host.set_espace(espace.clone(), d.path().join("e/dossiers-des-sessions.json"));
    host.set_vues_dir(d.path().join("e/vues"));
    let corps = json!({"client": "ide", "claude": {"session_id": "S1", "name": "le témoin"}}).to_string();
    let (_, r) = host.handle("POST", "/session", None, corps.as_bytes());
    let s = r["session"].as_str().unwrap().to_string();
    let manifeste = racines.config.join("atelier/templates/backends/atelier/backend.json");
    assert!(!manifeste.exists(), "the witness starts with an atelier already posed");

    let neuf = espace.join("arbre/neuf");
    let r = host.ouvrir_un_dossier(Some(&s), &neuf);
    assert_eq!(r["ok"], json!(true), "opening a folder without an atelier is refused: {r}");
    assert!(manifeste.is_file(), "the atelier is not posed: {r}");
    let m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    assert_eq!(m["workspace"]["root"], json!(espace.display().to_string()), "the atelier's root is not the space of worktrees");
    let nom = crate::code_tree::name_of_tree(&neuf.canonicalize().unwrap()).unwrap();
    assert!(racines.manifest(&nom).exists(), "the folder is not attached: {r}");
    let a = crate::daemon_host::atelier_fichiers::tests::Atelier { h: host.hosted(&manifeste).unwrap(), host, racine: espace };
    crate::daemon_host::atelier_fichiers::tests::attendre_l_index(&a, &neuf);
}
