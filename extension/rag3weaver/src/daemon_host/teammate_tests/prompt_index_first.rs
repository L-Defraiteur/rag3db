//! **What the teammate's prompt says of a new subject and of a written report** (24 Nov 2026: on every measure since
//! the 23rd, the agent searched the index only when told to word for word, and a reply after a written report gave the
//! report again). A subject taken up for the first time starts with `search_code`; a reply after a document gives its
//! conclusion and its path. A module of its own.

use crate::daemon_host::teammate::Step;

#[test]
fn the_prompt_starts_a_new_subject_with_search_code() {
    let p = Step::prompt_for_witnesses();
    assert!(p.contains("for the first time in a conversation, call `search_code` FIRST"), "{p}");
    assert!(p.contains("Read a file only once the index has told you which one"), "{p}");
}

#[test]
fn the_prompt_gives_a_reports_conclusion_and_path_not_the_report() {
    let p = Step::prompt_for_witnesses();
    assert!(p.contains("your reply does not repeat it: say in a few lines what it concludes, and give its path"), "{p}");
}
