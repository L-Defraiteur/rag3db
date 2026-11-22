//! **Every data root follows the host's data folder** (22 Nov 2026). The e2e hosts set `RAG3WEAVER_DAEMON_DIR`,
//! which only the host's own files followed: the tasks, the chats and the runs fell back to the user's real
//! folder, and an e2e host opened, and wrote, tasks there. A module of its own.

use crate::daemon_lifecycle::{data_roots_with, DataRoots};
use std::ffi::OsString;
use std::path::PathBuf;

fn roots(vars: &[(&str, &str)]) -> DataRoots {
    let vars: Vec<(String, String)> = vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    data_roots_with(&move |k| vars.iter().find(|(n, _)| n == k).map(|(_, v)| OsString::from(v)))
}

fn under(r: &DataRoots, dir: &str) -> Vec<String> {
    [("data", &r.data), ("tasks", &r.tasks), ("chats", &r.chats), ("runs", &r.runs)]
        .iter()
        .filter(|(_, p)| !p.starts_with(dir))
        .map(|(n, p)| format!("{n} = {}", p.display()))
        .collect()
}

#[test]
fn the_witnesses_host_folder_holds_every_data_root() {
    let r = roots(&[("HOME", "/home/someone"), ("RAG3WEAVER_DAEMON_DIR", "/tmp/witness")]);
    assert!(under(&r, "/tmp/witness").is_empty(), "outside the witness's folder: {:?}", under(&r, "/tmp/witness"));
    assert_eq!(r.tasks, PathBuf::from("/tmp/witness/taches"));
}

#[test]
fn the_data_folder_holds_every_data_root() {
    let r = roots(&[("HOME", "/home/someone"), ("RAG3WEAVER_DAEMON_DIR", "/tmp/witness"), ("RAG3WEAVER_DATA_DIR", "/tmp/data")]);
    assert!(under(&r, "/tmp/data").is_empty(), "outside the data folder: {:?}", under(&r, "/tmp/data"));
}

#[test]
fn a_roots_own_variable_still_wins() {
    let r = roots(&[("HOME", "/home/someone"), ("RAG3WEAVER_DATA_DIR", "/tmp/data"), ("RAG3WEAVER_TACHES_DIR", "/tmp/t"), ("RAG3WEAVER_RUNS_DIR", "/tmp/r")]);
    assert_eq!((r.tasks, r.runs), (PathBuf::from("/tmp/t"), PathBuf::from("/tmp/r")));
}

#[test]
fn in_service_nothing_moves() {
    let r = roots(&[("HOME", "/home/someone")]);
    let d = PathBuf::from("/home/someone/.local/share/rag3weaver");
    assert_eq!(r, DataRoots { data: d.clone(), tasks: d.join("taches"), chats: d.join("chats"), runs: d.join("runs") });
}

/// **The commands' journals too** (c7, 22 Nov 2026, 08 h: 54 968 index journals in `~/.cache/rag3weaver/commandes`
/// at luciepc, written by the witnesses' indexes). Under the lib's witnesses, without `RAG3WEAVER_JOURNAUX_DIR`, the
/// journals' folder is not the user's cache.
#[cfg(feature = "code")]
#[test]
fn the_witnesses_journals_stay_out_of_the_users_cache() {
    if std::env::var_os("RAG3WEAVER_JOURNAUX_DIR").is_some() {
        return;
    }
    let dossier = crate::dataflow::run_nodes::dossier_journaux();
    if let Some(cache) = crate::commande::dossier_de_cache() {
        assert!(!dossier.starts_with(&cache), "the witnesses write their journals in the user's cache: {}", dossier.display());
    }
}
