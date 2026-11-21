//! The witnesses of the title of a human, in a file of their own.

use super::*;

/// The user's token is made once, in a folder 0700 and a file 0600, and read again the same.
#[cfg(unix)]
#[test]
fn the_user_token_is_made_once_and_only_the_user_reads_it() {
    use std::os::unix::fs::PermissionsExt;
    let d = tempfile::tempdir().unwrap();
    let t = crate::daemon_lifecycle::user_token_in(d.path()).expect("made");
    assert!(t.len() >= 32, "{t}");
    assert_eq!(crate::daemon_lifecycle::user_token_in(d.path()).unwrap(), t, "kept, not remade");
    let dossier = d.path().join("user");
    assert_eq!(std::fs::metadata(&dossier).unwrap().permissions().mode() & 0o777, 0o700);
    assert_eq!(std::fs::metadata(dossier.join("user_token")).unwrap().permissions().mode() & 0o777, 0o600);
}

/// An entry code is asked with the user's token only, and serves once.
#[test]
fn an_entry_code_needs_the_users_token_and_serves_once() {
    let host = DaemonHost::new().with_user_token("le-jeton-de-lucie".into());
    assert!(host.issue_entry_code(None).is_err(), "the host's token alone gets no code");
    assert!(host.issue_entry_code(Some("faux")).is_err());
    let code = host.issue_entry_code(Some("le-jeton-de-lucie")).expect("a code");
    assert!(!code.contains("le-jeton-de-lucie"), "the code is not the token");
    assert_eq!(host.redeem_entry_code(&code).as_deref(), Some("le-jeton-de-lucie"));
    assert_eq!(host.redeem_entry_code(&code), None, "a code serves once");
    assert_eq!(host.redeem_entry_code("inconnu"), None);
}

/// A line is a human's only by the user cookie or header; the host's token alone is an agent.
#[test]
fn a_line_is_a_humans_only_by_the_users_title() {
    let host = DaemonHost::new().with_user_token("U".into());
    assert_eq!(host.by_of(Some("U"), None, "ide")["kind"], json!("human"));
    assert_eq!(host.by_of(None, Some("U"), "tty")["kind"], json!("human"));
    assert_eq!(host.by_of(None, None, "mcp")["kind"], json!("agent"));
    assert_eq!(host.by_of(Some("faux"), Some("faux"), "ide")["kind"], json!("agent"));
    let sans = DaemonHost::new();
    assert_eq!(sans.by_of(Some("U"), Some("U"), "ide")["kind"], json!("agent"), "a host without a user token knows no human");
}
