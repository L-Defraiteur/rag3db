//! **A code backend's chat starts from the person's project** (recherche's audit and Lucie's dry run, 22 Nov 2026: the
//! chat of a newly opened folder described the backend's declarations as « ce projet » and proposed a README of internal
//! manifests). Its prompt names the project's folder, says the declarations are not the project, and sends a change to
//! the project through open_code_task, never through propose. A module of its own.

use std::path::Path;

#[test]
fn the_prompt_names_the_project_and_says_the_declarations_are_not_it() {
    let p = super::chat::project_first(Some(Path::new("/home/someone/essai-audit")), false);
    assert!(p.contains("/home/someone/essai-audit"), "{p}");
    assert!(p.contains("not the project"), "{p}");
    assert!(p.contains("open_code_task") && p.contains("never through propose"), "{p}");
}

#[test]
fn in_a_task_the_project_is_changed_here() {
    let p = super::chat::project_first(Some(Path::new("/home/someone/essai-audit")), true);
    assert!(p.contains("not the project") && !p.contains("open_code_task"), "{p}");
}
