//! **The landing's git waits for an index lock to be released** (the lib in full, 23 Nov 2026: the host's
//! end-of-step commit found `index.lock` taken by another git in the same task tree, and the step's work stayed
//! uncommitted). A lock held a moment, then released: the command goes through.

use super::git;

#[test]
fn a_git_command_waits_for_an_index_lock_held_a_moment() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    assert!(std::process::Command::new("git").args(["init", "-q"]).current_dir(r).status().unwrap().success());
    std::fs::write(r.join("a.txt"), "a\n").unwrap();
    let verrou = r.join(".git/index.lock");
    std::fs::write(&verrou, "").unwrap();
    let libere = std::thread::spawn({
        let verrou = verrou.clone();
        move || {
            std::thread::sleep(std::time::Duration::from_millis(400));
            std::fs::remove_file(verrou).unwrap();
        }
    });
    let r2 = git(r, &["add", "a.txt"]);
    libere.join().unwrap();
    assert!(r2.is_ok(), "a lock held 400 ms is waited for: {r2:?}");
}
