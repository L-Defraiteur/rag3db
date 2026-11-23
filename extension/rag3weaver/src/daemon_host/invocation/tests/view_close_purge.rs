//! **A closed view leaves nothing behind by default** (c7 for Lucie, 21 November 2026, 23 h 5x: « on ne garde pas tous
//! les worktrees passés »): the witness of the lot `view-close-purge`, in a file of its own.

use super::*;

/// **`close_view` purges by default; `keep` keeps the view's generations**: through the host's route, as the MCP tool
/// calls it, a close without option leaves no generation of the view, and a close with `keep: true` leaves them.
#[test]
fn close_view_purges_by_default_and_keeps_on_demand() {
    let fermer = |v: &OpenedView, options: Value| {
        let mut corps = json!({ "manifest": v.manifeste, "name": v.nom });
        corps.as_object_mut().unwrap().extend(options.as_object().unwrap().clone());
        let (_, r) = v.a.host.handle("POST", "/views/close", Some(&v.s), corps.to_string().as_bytes());
        assert_eq!(r["ok"], json!(true), "{r}");
        r
    };
    let v = opened_view("purge");
    assert!(generations_de(&v) > 0, "the view has no generation to purge");
    let r = fermer(&v, json!({}));
    assert_eq!(generations_de(&v), 0, "close_view without option leaves the view's generations: {r}");
    let v = opened_view("garde");
    let r = fermer(&v, json!({ "keep": true }));
    assert!(generations_de(&v) > 0, "close_view with keep leaves no generation: {r}");
}
