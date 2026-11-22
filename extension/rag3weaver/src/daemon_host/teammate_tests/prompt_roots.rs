//! **What the teammate's prompt says of roots and cards** (Lucie, 21 Nov 2026, 20 h 25; the dry run of 22 Nov). Outside
//! its roots, the agent asks for a project's root once, with its reason, never a folder above the project; a command
//! that waits for the supervisor's card has not run, and its result is never reported as done. A module of its own.

use crate::daemon_host::teammate::Step;

#[test]
fn the_prompt_asks_for_a_projects_root_once() {
    let p = Step::prompt_for_witnesses();
    assert!(p.contains("the project's root") && p.contains(".git") && p.contains("package.json"), "{p}");
    assert!(p.contains("never a folder above the project"), "{p}");
}

#[test]
fn the_prompt_says_a_held_command_has_not_run() {
    let p = Step::prompt_for_witnesses();
    assert!(p.contains("waits for the supervisor's card has not run"), "{p}");
}
