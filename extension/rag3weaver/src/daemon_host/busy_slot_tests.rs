//! **A call refused because a close, a reopening or an opening holds its backend says it is transient** (23 November
//! 2026: during the nightly full-text maintenance, `recall_fiches` answered « recommencez dans un moment » with
//! `transient: false`, and a client that reads the field gave up). A file of its own.

use super::*;

#[test]
fn a_refusal_while_a_close_or_reopening_holds_the_backend_is_transient() {
    let e = format!(
        "/m/backend.json : une fermeture ou une réouverture tient ce backend depuis plus de {} s : {SLOT_BUSY}",
        SLOT_READ_WAIT.as_secs()
    );
    let r = op_response(Err(e));
    assert_eq!(r["transient"], json!(true), "{r}");
}

#[test]
fn a_refusal_while_the_backend_opens_is_transient() {
    let r = op_response(Err(format!("/m/backend.json {OPENING} 12 s (son schéma et ses index se chargent) : {SLOT_BUSY}")));
    assert_eq!(r["transient"], json!(true), "{r}");
}

#[test]
fn any_other_error_stays_not_transient() {
    let r = op_response(Err("l'outil « x » n'existe pas".into()));
    assert_eq!(r["transient"], json!(false), "{r}");
}
