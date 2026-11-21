//! **A view follows its worktree within two seconds** (c7, 21 November 2026): the witness of the lot `view-follows`, in a
//! file of its own.

use super::*;

/// **A function written in a view's worktree is found under the view within two seconds**: the bound the move from a
/// poll each second to the kernel's events (inotify, with a slow poll every 30 s) must keep.
#[test]
fn a_write_in_a_views_worktree_is_seen_under_the_view_within_two_seconds() {
    let v = une_vue_ouverte("delai");
    std::fs::write(v.chemin.join("src/lib.rs"), "pub fn un() -> u32 {\n    1\n}\n\npub fn vue_en_moins_de_deux_secondes() -> u32 {\n    2\n}\n").unwrap();
    let debut = std::time::Instant::now();
    let mut vu = String::new();
    while debut.elapsed() < std::time::Duration::from_secs(2) {
        let r = v.a.host.call(Some(&v.s), &v.manifeste, "search_code", json!({"query": "vue_en_moins_de_deux_secondes", "options": {"limit": 10}}));
        vu = r["result"]["result"].to_string();
        if vu.contains("vue_en_moins_de_deux_secondes") {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!("a write in the view's worktree is not seen under the view within 2 s: {vu}");
}
