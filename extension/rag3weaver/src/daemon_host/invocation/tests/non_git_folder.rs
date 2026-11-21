//! **A folder that is not a git repository opens as it is** (Lucie, 21 November 2026, 9 h 5x): the witness of the lot
//! `dossier-sans-git`, in a file of its own.

use super::*;

/// **A folder like Steam's opens without git init and without an index, and read_file reads it**: an existing folder
/// with files, not a git repository, was refused; it is attached as it is — no `.git` made, no index, no watch — says
/// `git: false` and `index_offer: false` (no marker of a code or docs project), and its files are read.
#[test]
fn a_folder_like_steams_opens_without_git_init_nor_index_and_is_read() {
    let d = tempfile::tempdir().unwrap();
    let (a, _espace, s) = atelier_d_espace(d.path());
    let jeux = d.path().join("steam");
    std::fs::create_dir_all(jeux.join("steamapps/common/UnJeu")).unwrap();
    std::fs::write(jeux.join("steamapps/common/UnJeu/donnees.pak"), vec![0u8; 64 * 1024]).unwrap();
    std::fs::write(jeux.join("libraryfolders.vdf"), "\"libraryfolders\"\n{\n}\n").unwrap();

    let r = a.host.ouvrir_un_dossier(Some(&s), &jeux);
    assert_eq!(r["ok"], json!(true), "a folder that is not a git repository is refused: {r}");
    assert_eq!((&r["git"], &r["index_offer"], &r["created"]), (&json!(false), &json!(false), &json!(false)), "{r}");
    assert!(!jeux.join(".git").exists(), "the host made git init in an existing folder");
    let racines = a.host.racines_de_code().unwrap();
    let nom = crate::code_tree::name_of_tree(&jeux.canonicalize().unwrap()).unwrap();
    let manifeste = racines.manifest(&nom).canonicalize().unwrap();
    let h = a.host.hosted(&manifeste).unwrap();
    assert_eq!(a.host.with_backend(&h, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap(), 0, "an indexing runs");
    let lu = a.host.with_backend(&h, |b| b.call("read_file", json!({"path": "libraryfolders.vdf"}))).unwrap();
    let lu = lu.unwrap_or_else(|e| panic!("read_file in the folder: {e}"));
    assert!(lu.to_string().contains("libraryfolders"), "read_file does not read the folder: {lu}");
}
