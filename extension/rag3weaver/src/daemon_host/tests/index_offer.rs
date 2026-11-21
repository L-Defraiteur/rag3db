//! **The index offered, or not** (archi's page « un agent qui part neutre », 21 November 2026, arbre's part 3: a rule
//! declared in the atelier's template, `index_offer.markers`): the witness of the lot `index-offer`, in a file of its own.

use super::*;
use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index};

/// **A code project is offered its index, once; a folder without a project's marker is not**: opening a repository with
/// a `Cargo.toml` says `index_offer: true`; after « non merci » (`decline_index`), opening it again says false, with
/// the refusal as its reason; a new empty folder (a git repository, nothing else) says false.
#[test]
fn the_index_is_offered_to_a_project_once_and_not_to_a_folder_without_markers() {
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

    let projet = espace.join("projet");
    std::fs::create_dir_all(projet.join("src")).unwrap();
    std::fs::write(projet.join("Cargo.toml"), "[package]\nname = \"projet\"\nversion = \"0.1.0\"\n").unwrap();
    std::fs::write(projet.join("src/main.rs"), "fn main() {}\n").unwrap();
    let r = a.host.ouvrir_un_dossier(Some(&s), &projet);
    assert_eq!(r["ok"], json!(true), "{r}");
    assert_eq!(r["index_offer"], json!(true), "a project with a Cargo.toml is not offered its index: {r}");

    let non = a.host.decline_index(&projet);
    assert!(non.is_ok(), "the refusal is not kept: {non:?}");
    let r = a.host.ouvrir_un_dossier(Some(&s), &projet);
    assert_eq!(r["index_offer"], json!(false), "the index is offered again after « non merci »: {r}");
    assert!(r["index_offer_why"].as_str().is_some_and(|w| w.contains("refusé")), "the reason does not say the refusal: {r}");

    let vide = espace.join("vide");
    let r = a.host.ouvrir_un_dossier(Some(&s), &vide);
    assert_eq!(r["index_offer"], json!(false), "a folder without a project's marker is offered an index: {r}");
    attendre_l_index(&a, &projet);
}
