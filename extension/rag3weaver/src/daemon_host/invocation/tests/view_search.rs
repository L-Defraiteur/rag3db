//! **A session searches its own worktree** (Lucie, 21 November 2026, 12 h 40: the code MCP « vraiment utile »): the
//! witness of the lot `view-search`, in a file of its own.

use super::*;

/// **`search_code` under the view of a worktree finds what is written there, uncommitted, and not what was removed**:
/// after `open_view`, a function added in the view's worktree and not committed is found by `search_code` under the
/// session's view, and a function the worktree removed is no longer returned. Polled up to a minute: the view follows
/// its worktree by a watch.
#[test]
fn search_code_under_a_worktree_view_finds_its_uncommitted_code_and_not_its_removed_code() {
    let v = une_vue_ouverte("cherche");
    std::fs::write(
        v.chemin.join("src/lib.rs"),
        "pub fn un() -> u32 {\n    1\n}\n\npub fn quarante_deux_dans_la_vue() -> u32 {\n    42\n}\n",
    )
    .unwrap();
    let chercher = |q: &str| v.a.host.call(Some(&v.s), &v.manifeste, "search_code", json!({"query": q, "options": {"limit": 20}})).to_string();
    let debut = std::time::Instant::now();
    let (mut neuve, mut partie) = (String::new(), String::new());
    while debut.elapsed() < std::time::Duration::from_secs(60) {
        neuve = chercher("quarante_deux_dans_la_vue");
        partie = chercher("deux");
        if neuve.contains("quarante_deux_dans_la_vue()") && !partie.contains("pub fn deux()") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    assert!(neuve.contains("quarante_deux_dans_la_vue()"), "search_code under the view does not find the function written in its worktree: {neuve}");
    assert!(!partie.contains("pub fn deux()"), "search_code under the view still returns the function its worktree removed: {partie}");
}
