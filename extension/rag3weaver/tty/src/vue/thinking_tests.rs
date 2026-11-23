//! **The model's thinking, glimpsed in the animation** (23 November 2026): while the teammate thinks, the living line
//! shows the last words of its thinking, muted and in italics, never in the history; the answer takes their place;
//! Ctrl-O reads the whole of it. Without thinking, the animation stays as it was.
use super::*;
use serde_json::json;

fn ev(e: serde_json::Value) -> serde_json::Value {
    json!({ "evenement": e })
}

fn vivante(v: &Vue) -> Line<'static> {
    v.ligne_vivante(120, Instant::now())
}

fn texte(l: &Line) -> String {
    l.spans.iter().map(|s| s.content.to_string()).collect()
}

#[test]
fn the_thinking_is_glimpsed_in_the_living_line_and_never_written() {
    let mut v = Vue::new(120, true);
    v.trame(&ev(json!({ "kind": "LlmCall" })));
    v.trame(&ev(json!({ "kind": "Thinking", "iteration": 1, "text": "L'utilisatrice veut un site. " })));
    v.trame(&ev(json!({ "kind": "Thinking", "iteration": 1, "text": "Je vais lire le dossier puis écrire index.html" })));
    // The ticker starts at the thinking's beginning; once it has slid to the end, the last words show.
    let l = v.ligne_vivante(120, v.thinking_since + Duration::from_secs(60));
    let t = texte(&l);
    assert!(t.contains("puis écrire index.html"), "{t}");
    let span = l.spans.iter().find(|s| s.content.contains("index.html")).expect("a span of thinking");
    assert!(span.style.add_modifier.contains(Modifier::ITALIC), "the thinking is in italics: {:?}", span.style);
    assert!(v.prendre_lignes().iter().all(|x| !x.texte.contains("index.html")), "never written in the history");
}

#[test]
fn the_answer_takes_the_place_of_the_thinking() {
    let mut v = Vue::new(120, true);
    v.trame(&ev(json!({ "kind": "LlmCall" })));
    v.trame(&ev(json!({ "kind": "Thinking", "iteration": 1, "text": "je réfléchis à la structure" })));
    v.trame(&ev(json!({ "kind": "Token", "text": "Voici le site." })));
    assert!(!texte(&vivante(&v)).contains("structure"), "{}", texte(&vivante(&v)));
    v.trame(&ev(json!({ "kind": "ToolCallStarted", "tool": "read_file" })));
    v.trame(&ev(json!({ "kind": "LlmCall" })));
    assert!(!texte(&vivante(&v)).contains("structure"), "a new call does not bring the old thinking back: {}", texte(&vivante(&v)));
    let h: String = v.prendre_lignes().iter().map(|x| x.texte.clone()).collect::<Vec<_>>().join("\n");
    assert!(h.contains("Voici le site.") && !h.contains("structure"), "{h}");
}

#[test]
fn ctrl_o_reads_the_whole_thinking() {
    let mut v = Vue::new(120, true);
    v.trame(&ev(json!({ "kind": "LlmCall" })));
    v.trame(&ev(json!({ "kind": "Thinking", "iteration": 1, "text": "première idée. " })));
    v.trame(&ev(json!({ "kind": "Thinking", "iteration": 1, "text": "seconde idée." })));
    let n = v.openable_count();
    assert!(n >= 1);
    assert_eq!(v.openable_label(n - 1), "réflexion");
    let tout: String = v.openable_lines(n - 1).iter().map(|x| x.texte.clone()).collect::<Vec<_>>().join("\n");
    assert!(tout.contains("première idée.") && tout.contains("seconde idée."), "{tout}");
}
