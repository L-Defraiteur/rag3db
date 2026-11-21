//! **`search_code` reads the view of a worktree, uncommitted work included** (Lucie, 21 November 2026, 12 h 28:
//! « une session doit pouvoir chercher dans son propre worktree » ; the criterion of the orchestration: a function not
//! committed in the worktree is found, a function it deleted is not returned). End to end through the host: a view
//! opened by `open_view`, its worktree edited without a commit, `search_code` called with `view`.

use serde_json::{json, Value};

use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index as wait_for_the_index, petit_depot as small_repository};

/// The hits of the view search for `query` under `layer`, as text: the tool's `result` only — the response repeats the
/// query in its metadata, so reading the whole response would always « find » it.
fn search(workshop: &crate::daemon_host::atelier_fichiers::tests::Atelier, manifest: &std::path::Path, query: &str, layer: &str) -> String {
    let hosted = workshop.host.hosted(manifest).unwrap();
    let answer: Result<Value, String> = workshop
        .host
        .with_backend(&hosted, |b| b.call("search_code", json!({ "query": query, "options": { "limit": 20 }, "view": { "layer": layer } })))
        .and_then(|r| r);
    match answer {
        Ok(v) => v["result"].to_string(),
        Err(e) => format!("REFUSED: {e}"),
    }
}

/// **A view on a worktree finds its uncommitted function and not the one it deleted.** The repository has `un` and
/// `deux`; in the view's worktree, without a commit, `deux` is deleted and `trois_non_commise` is added. Under the view's
/// layer, `search_code` finds `trois_non_commise` (within 120 s, the time for the view's work layer to be written) and
/// does not return `deux`; under the backend's layer, `deux` is still there.
#[test]
fn a_view_on_a_worktree_finds_its_uncommitted_function_and_not_its_deleted_one() {
    let dir = tempfile::tempdir().unwrap();
    let workshop = atelier(dir.path());
    let repository = small_repository(&dir.path().join("B"));
    workshop.appeler("attach_worktree", json!({ "path": repository })).unwrap();
    wait_for_the_index(&workshop, &repository);
    let tree = crate::code_tree::name_of_tree(&repository.canonicalize().unwrap()).unwrap();
    let manifest = workshop.host.racines_de_code().unwrap().manifest(&tree);
    let space = dir.path().join("e/git_workspaces");
    std::fs::create_dir_all(space.join("witness")).unwrap();
    let table = dir.path().join("e/dossiers-des-sessions.json");
    std::fs::write(&table, json!({ "witness": "witness" }).to_string()).unwrap();
    workshop.host.set_espace(space, table);
    workshop.host.set_vues_dir(dir.path().join("e/vues"));
    let body = json!({ "client": "mcp (pid 1)", "claude": { "session_id": "S1", "name": "witness" } }).to_string();
    let (_, opened) = workshop.host.handle("POST", "/session", None, body.as_bytes());
    let session = opened["session"].as_str().unwrap().to_string();
    let view = workshop.host.open_view(Some(&session), &manifest, "search", None);
    assert_eq!(view["ok"], json!(true), "{view}");
    assert!(workshop.host.attendre_l_ecriture_des_vues(std::time::Duration::from_secs(120)), "the view's layer is not written");
    let worktree = std::path::PathBuf::from(view["path"].as_str().unwrap());
    let layer = view["layer"].as_str().expect("the view's layer").to_string();

    std::fs::write(worktree.join("src/lib.rs"), "pub fn un() -> u32 {\n    1\n}\n\npub fn trois_non_commise() -> u32 {\n    3\n}\n").unwrap();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    let mut found = search(&workshop, &manifest, "trois_non_commise", &layer);
    while !found.contains("trois_non_commise") && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(500));
        found = search(&workshop, &manifest, "trois_non_commise", &layer);
    }
    assert!(found.contains("trois_non_commise"), "the view does not find the function not committed in its worktree: {found}");
    let deleted = search(&workshop, &manifest, "deux", &layer);
    assert!(!deleted.contains("fn deux"), "the view returns the function its worktree deleted: {deleted}");
    let backend = search(&workshop, &manifest, "deux", crate::generations::COUCHE_DU_BACKEND);
    assert!(backend.contains("deux"), "the backend's view lost `deux`, which only the worktree deleted: {backend}");
    let _ = workshop.host.close_view(Some(&session), &manifest, &view["name"].as_str().unwrap().to_string(), true);
}
