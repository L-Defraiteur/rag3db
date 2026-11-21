//! **`rag3weaver-backend ouvrir [dossier]`** (Lucie, 21 November 2026, 8 h 1x: « atelier veut rien dire en soi pour un
//! user »): the witness of the lot `commande-ouvrir`, in a file of its own.

use super::*;
use crate::daemon_host::chat::ChatModel;
use crate::daemon_host::chat_qui_code::hote;
use crate::llm::{Llm, MockLlm};

/// **The command opens a folder as the IDE does**: by the host's `/dossiers/ouvrir`, on a host without anything posed
/// yet; the folder is created, attached, and the command says so in the user's words — « dossier ouvert », « dépôt
/// attaché » — never « atelier ».
#[test]
fn the_command_opens_a_folder_by_the_host_and_says_it_without_atelier() {
    let d = tempfile::tempdir().unwrap();
    let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
    let espace = d.path().join("git_workspaces");
    std::fs::create_dir_all(&espace).unwrap();
    let table = d.path().join("dossiers-des-sessions.json");
    std::fs::write(&table, "{}").unwrap();
    let (racines_h, espace_h, table_h, runs) = (racines.clone(), espace.clone(), table.clone(), d.path().join("runs"));
    let base = d.path().to_path_buf();
    let (adresse, fil) = hote_garde_avec("j", move |_| {
        let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Ok(Arc::new(MockLlm::new(""))) });
        let h = hote(&base, model).with_racines_de_code(racines_h).with_runs_root(runs);
        h.set_espace(espace_h, table_h);
        h
    });
    let endpoint = crate::daemon_client::Endpoint { address: adresse.clone(), token: Some("j".into()) };
    let neuf = espace.join("neuf");
    let r = crate::code_tree::ouvrir(&endpoint, &neuf);
    let r = r.unwrap_or_else(|e| panic!("the command does not open the folder: {e}"));
    let texte = crate::code_tree::dire_l_ouverture(&r);
    assert!(texte.contains("dossier ouvert") && texte.contains("dépôt attaché"), "the command does not say it in the user's words: {texte}");
    assert!(!texte.to_lowercase().contains("atelier"), "the command still says « atelier »: {texte}");
    let nom = crate::code_tree::name_of_tree(&neuf.canonicalize().unwrap()).unwrap();
    assert!(racines.manifest(&nom).exists(), "the folder is not attached: {r}");
    let (statut, _) = demander("POST", &format!("http://{adresse}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}
