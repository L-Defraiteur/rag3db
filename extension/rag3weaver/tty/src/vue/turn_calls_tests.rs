//! **What a turn did, said truly** (a session on 23 November 2026: after one `ls -la`, three model calls answered
//! nothing, and the view kept showing « » run_command ls -la ⟨3⟩ » with seconds climbing, read as the same tool run
//! three times). The rank of a call counts the calls of its turn; each empty model answer has its own line with its
//! reason, in place of the last tool; the session journal keeps a trace of each call (turn, rank, tool, id,
//! arguments) that the screen does not show.
use super::*;
use serde_json::json;

fn ev(e: serde_json::Value) -> serde_json::Value {
    json!({ "evenement": e })
}

fn outil(v: &mut Vue, id: &str, tool: &str, args: &str) {
    v.trame(&ev(json!({ "kind": "ToolCallStarted", "tool": tool, "call_id": id, "arguments": args })));
    v.trame(&ev(json!({ "kind": "ToolCallFinished", "call_id": id, "ok": true, "ms": 3 })));
}

#[test]
fn the_rank_of_a_call_counts_the_calls_of_its_turn() {
    let mut v = Vue::new(160, true);
    v.ligne_de_boite(&json!({ "id": "b/t:1", "from": "supervisor", "kind": "request", "text": "un" }));
    outil(&mut v, "c1", "read_file", r#"{"path":"a"}"#);
    v.ligne_de_boite(&json!({ "id": "b/t:2", "from": "supervisor", "kind": "instruction", "text": "deux" }));
    outil(&mut v, "c2", "read_file", r#"{"path":"b"}"#);
    outil(&mut v, "c3", "read_file", r#"{"path":"c"}"#);
    let n = v.openable_count();
    assert_eq!(v.openable_label(0), "appel 1 sur 1 du tour 1");
    assert_eq!(v.openable_label(n - 1), "appel 2 sur 2 du tour 2");
}

#[test]
fn each_empty_model_answer_has_its_own_line_with_its_reason() {
    let mut v = Vue::new(160, true);
    v.ligne_de_boite(&json!({ "id": "b/t:1", "from": "supervisor", "kind": "request", "text": "site" }));
    v.trame(&ev(json!({ "kind": "LlmCall", "completion_tokens": 18, "finish": "ToolCall", "iteration": 1 })));
    outil(&mut v, "c1", "run_command", r#"{"command":"ls -la"}"#);
    for i in 2..5 {
        v.trame(&ev(json!({ "kind": "LlmCall", "completion_tokens": 0, "finish": "MaxTokens", "iteration": i })));
    }
    let lignes: Vec<String> = v.prendre_lignes().iter().map(|l| l.texte.clone()).collect();
    let vides = lignes.iter().filter(|l| l.contains("réponse vide du modèle : le budget de sortie est épuisé (MaxTokens)")).count();
    assert_eq!(vides, 3, "{lignes:#?}");
    // Without the steps shown, the tool runs on the muted line and is not written: never more than once.
    assert!(lignes.iter().filter(|l| l.contains("run_command ls -la")).count() <= 1, "no tool line repeated: {lignes:#?}");
    assert!(v.discret.as_deref().is_some_and(|d| d.contains("réponse vide")), "the muted line no longer shows the old tool: {:?}", v.discret);
    // A reason given by the host is taken as it is.
    v.trame(&ev(json!({ "kind": "LlmCall", "completion_tokens": 0, "finish": "MaxTokens", "empty": { "reason": "la réflexion a rempli tout le budget", "relaunch": true } })));
    let t: String = v.prendre_lignes().iter().map(|l| l.texte.clone()).collect();
    assert!(t.contains("réponse vide du modèle : la réflexion a rempli tout le budget"), "{t}");
}

#[test]
fn the_journal_keeps_a_trace_of_each_call_that_the_screen_does_not_show() {
    let mut v = Vue::new(160, true);
    v.ligne_de_boite(&json!({ "id": "b/t:1", "from": "supervisor", "kind": "request", "text": "x" }));
    outil(&mut v, "call_157", "run_command", r#"{"command":"ls -la"}"#);
    let traces = v.prendre_traces();
    assert_eq!(traces.len(), 1, "{traces:#?}");
    let t = &traces[0];
    assert!(t.contains("tour 1 · appel 1 · run_command · call_157") && t.contains("ls -la"), "{t}");
    assert!(v.prendre_lignes().iter().all(|l| !l.texte.contains("⟦trace⟧")), "the history, hence the screen, holds no trace");
}

/// The host's mailbox line for an empty answer (`report` with `cut: true`) is not said a second time, even with the
/// steps shown: the run's event already said it.
#[test]
fn the_hosts_cut_report_is_not_said_twice() {
    let mut v = Vue::new(160, true);
    v.montrer_etapes = true;
    v.ligne_de_boite(&json!({ "id": "b/t:1", "from": "supervisor", "kind": "request", "text": "x" }));
    v.trame(&ev(json!({ "kind": "LlmCall", "completion_tokens": 0, "finish": "MaxTokens" })));
    v.ligne_de_boite(&json!({ "id": "b/t:2", "from": "host", "kind": "report", "text": "réponse vide du modèle au pas 1 : le budget de sortie est épuisé (MaxTokens) ; le pas suivant reprend", "cut": true, "why": "le budget de sortie est épuisé (MaxTokens)" }));
    let t: Vec<String> = v.prendre_lignes().iter().map(|l| l.texte.clone()).collect();
    assert_eq!(t.iter().filter(|l| l.contains("réponse vide du modèle")).count(), 1, "{t:#?}");
}
