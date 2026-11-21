//! **The index made on demand** (archi's page « un agent qui part neutre », 21 November 2026, arbre's part 2): the
//! witness of the lot `index-a-la-demande`, in a file of its own.

use super::*;
use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index, petit_depot};

/// **A folder attached without an index is indexed when asked**: its watches come back from `no-index.json`, which
/// goes, and its first index runs; then the code tools that need it (`usages`) answer.
#[test]
fn a_folder_attached_without_an_index_is_indexed_on_demand() {
    let d = tempfile::tempdir().unwrap();
    let a = atelier(d.path());
    let espace = d.path().join("git_workspaces");
    std::fs::create_dir_all(&espace).unwrap();
    let table = d.path().join("dossiers-des-sessions.json");
    std::fs::write(&table, "{}").unwrap();
    a.host.set_espace(espace.clone(), table);
    let corps = json!({"client": "ide", "claude": {"session_id": "S1", "name": "le témoin"}}).to_string();
    let (_, r) = a.host.handle("POST", "/session", None, corps.as_bytes());
    let s = r["session"].as_str().unwrap().to_string();
    let depot = petit_depot(&espace);
    let r = a.host.ouvrir_un_dossier(Some(&s), &depot);
    assert_eq!(r["ok"], json!(true), "{r}");
    let racines = a.host.racines_de_code().unwrap();
    let nom = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
    let manifeste = racines.manifest(&nom).canonicalize().unwrap();
    let note = manifeste.ancestors().find(|p| p.file_name().is_some_and(|n| n == "templates")).unwrap().parent().unwrap().join("no-index.json");
    assert!(note.is_file(), "the witness starts from a folder attached without an index");

    let r = a.host.index_folder(&depot);
    let r = r.unwrap_or_else(|e| panic!("indexing on demand is refused: {e}"));
    assert!(!note.exists(), "no-index.json stays: {r}");
    let m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    assert!(m["watches"].is_object(), "the watches are not put back: {r}");
    attendre_l_index(&a, &depot);
    let h = a.host.hosted(&manifeste).unwrap();
    let usages = a.host.with_backend(&h, |b| b.call("usages", json!({"name": "un"}))).unwrap();
    let usages = usages.unwrap_or_else(|e| panic!("usages after the index on demand: {e}"));
    assert!(usages.to_string().contains("src/lib.rs"), "the index does not know the folder's code: {usages}");
}
