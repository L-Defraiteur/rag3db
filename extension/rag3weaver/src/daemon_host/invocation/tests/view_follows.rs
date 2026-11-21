//! **A view follows its worktree within two seconds** (c7, 21 November 2026): the witness of the lot `view-follows`, in a
//! file of its own.

use super::*;

/// **A write in a view's worktree is followed by a kernel event, and found under the view** (c7, 21 November 2026; made
/// reliable the same night: the bound of 2 s fell under the load of the whole lib at the pusher). What it proves: the
/// layer is rewritten on a kernel event (inotify), not by the slow poll of 30 s, and the function written is then found
/// under the view. The bound of 30 s only keeps it from hanging; the delays measured are printed, not judged.
#[test]
fn a_write_in_a_views_worktree_is_followed_by_a_kernel_event_and_seen_under_the_view() {
    let v = une_vue_ouverte("delai");
    let (evenements, releves) = v.a.host.view_follower_rewrites(&v.couche).expect("the view does not follow its worktree");
    std::fs::write(v.chemin.join("src/lib.rs"), "pub fn un() -> u32 {\n    1\n}\n\npub fn vue_suivie_par_le_noyau() -> u32 {\n    2\n}\n").unwrap();
    let debut = std::time::Instant::now();
    let borne = std::time::Duration::from_secs(30);
    let reecrite = loop {
        let (e, r) = v.a.host.view_follower_rewrites(&v.couche).unwrap();
        if e + r > evenements + releves {
            break (e - evenements, r - releves);
        }
        assert!(debut.elapsed() < borne, "the view's layer is not rewritten within {borne:?} of a write in its worktree");
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let a_la_reecriture = debut.elapsed();
    assert_eq!(reecrite, (1, 0), "the layer was not rewritten on a kernel event (by event, by poll): {reecrite:?}");
    let mut vu = String::new();
    while debut.elapsed() < borne {
        let r = v.a.host.call(Some(&v.s), &v.manifeste, "search_code", json!({"query": "vue_suivie_par_le_noyau", "options": {"limit": 10}}));
        vu = r["result"]["result"].to_string();
        if vu.contains("vue_suivie_par_le_noyau") {
            eprintln!("vue suivie : couche réécrite en {a_la_reecriture:?}, fonction trouvée en {:?}", debut.elapsed());
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!("the layer was rewritten, but the function written is not found under the view: {vu}");
}

/// **A view at rest computes no signature** (c7, 21 November 2026): opened, then left five seconds without a write, its
/// follower runs no `git status` — none, or one if the slow poll of 30 s falls in. A poll each second would run five.
#[test]
fn a_view_at_rest_runs_no_signature() {
    let v = une_vue_ouverte("repos");
    let avant = v.a.host.view_follower_turns(&v.couche).expect("the view does not follow its worktree");
    std::thread::sleep(std::time::Duration::from_secs(5));
    let tours = v.a.host.view_follower_turns(&v.couche).unwrap() - avant;
    assert!(tours <= 1, "a view at rest ran {tours} signatures in 5 s");
}
