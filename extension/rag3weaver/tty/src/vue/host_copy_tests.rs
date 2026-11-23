//! **The host's copy is never shown in a turn whose text was streamed** (a trial on 23 November 2026: « wtf pourquoi il
//! parle 2 fois maintenant » — the announcement before a tool and the answer after it were shown live, then the end of
//! step copy, which joins both, was shown again). In a turn where `Token`s were shown, the `host_copy` report is not
//! written; it serves the reopening of a conversation.
use super::*;
use serde_json::json;

fn tour(v: &mut Vue) -> Vec<String> {
    let ev = |e: serde_json::Value| json!({ "evenement": e });
    v.ligne_de_boite(&json!({ "id": "b/t:1", "from": "supervisor", "kind": "request", "text": "tu as accès à l'index ?" }));
    v.trame(&ev(json!({ "kind": "RunStarted", "run": "r1" })));
    v.trame(&ev(json!({ "kind": "Token", "run": "r1", "text": "Je vérifie l'état de l'index et des fichiers." })));
    v.trame(&ev(json!({ "kind": "LlmCall", "run": "r1", "completion_tokens": 12, "finish": "ToolCall" })));
    v.trame(&ev(json!({ "kind": "ToolCallStarted", "run": "r1", "tool": "list_files", "call_id": "c1", "arguments": "{}" })));
    v.trame(&ev(json!({ "kind": "ToolCallFinished", "run": "r1", "call_id": "c1", "ok": true, "ms": 3 })));
    v.trame(&ev(json!({ "kind": "Token", "run": "r1", "text": "Bonjour, je suis Algareth. Oui, j'ai bien accès à un index." })));
    v.trame(&ev(json!({ "kind": "LlmCall", "run": "r1", "completion_tokens": 14, "finish": "Stop" })));
    v.trame(&ev(json!({ "kind": "RunFinished", "run": "r1", "ok": true })));
    v.tick(Instant::now() + Duration::from_secs(5));
    v.ligne_de_boite(&json!({ "id": "b/t:2", "from": "teammate", "kind": "report", "origin": "host_copy", "text": "Je vérifie l'état de l'index et des fichiers.\n\nBonjour, je suis Algareth. Oui, j'ai bien accès à un index." }));
    v.ligne_de_boite(&json!({ "id": "b/t:3", "from": "host", "kind": "ended", "text": "fait sans plan" }));
    v.prendre_lignes().iter().map(|l| l.texte.clone()).collect()
}

#[test]
fn a_streamed_turn_is_not_said_twice_by_the_hosts_copy() {
    for steps in [false, true] {
        let mut v = Vue::new(200, true);
        v.montrer_etapes = steps;
        let h = tour(&mut v);
        for t in ["Je vérifie l'état de l'index", "Bonjour, je suis Algareth"] {
            assert_eq!(h.iter().filter(|l| l.contains(t)).count(), 1, "« {t} » once (steps {steps}): {h:#?}");
        }
    }
}
