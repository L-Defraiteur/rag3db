//! **The thinking scrolls like a station board** (23 November 2026: « la réflexion pourrait défiler, comme dans une
//! gare l'affichage d'un train qui arrive »): beside the animation, the thinking slides from right to left at a
//! reading pace, the new thinking entering at the right; with the steps shown or `--still`, the sentence stays fixed;
//! wide characters never break the width.
use super::*;
use serde_json::json;

fn ev(e: serde_json::Value) -> serde_json::Value {
    json!({ "evenement": e })
}

fn droite(v: &Vue, t: Instant) -> String {
    v.ligne_vivante(120, t).spans.iter().filter(|s| s.style.add_modifier.contains(Modifier::ITALIC)).map(|s| s.content.to_string()).collect()
}

fn pense(v: &mut Vue, texte: &str) {
    v.trame(&ev(json!({ "kind": "LlmCall" })));
    v.trame(&ev(json!({ "kind": "Thinking", "iteration": 1, "text": texte })));
}

const LONG: &str = "Je dois lire le dossier, comprendre la structure du projet, puis écrire un fichier index.html avec une scène three.js et des particules qui explosent au clic.";

#[test]
fn the_thinking_slides_between_two_frames() {
    let mut v = Vue::new(120, true);
    pense(&mut v, LONG);
    let t0 = v.thinking_since;
    let a = droite(&v, t0);
    let b = droite(&v, t0 + Duration::from_secs(2));
    assert!(!a.is_empty() && a != b, "it moves: {a:?} then {b:?}");
    assert!(LONG.contains(a.trim()) && LONG.contains(b.trim()), "windows of the thinking itself: {a:?} {b:?}");
    let (ia, ib) = (LONG.find(a.trim()).unwrap(), LONG.find(b.trim()).unwrap());
    assert!(ib > ia, "from right to left: the later window starts further in the text ({ia} then {ib})");
}

#[test]
fn with_the_steps_shown_or_still_the_sentence_stays_fixed() {
    for (steps, still) in [(true, false), (false, true)] {
        let mut v = Vue::new(120, true);
        v.montrer_etapes = steps;
        v.still = still;
        pense(&mut v, LONG);
        let t0 = v.thinking_since;
        assert_eq!(droite(&v, t0), droite(&v, t0 + Duration::from_secs(5)), "fixed (steps {steps}, still {still})");
    }
}

#[test]
fn wide_characters_never_break_the_width() {
    let mut v = Vue::new(120, true);
    pense(&mut v, &"réflexion 🚂 très élaborée — ".repeat(12));
    let t0 = v.thinking_since;
    for s in 0..40 {
        let l = v.ligne_vivante(120, t0 + Duration::from_millis(s * 137));
        let largeur: usize = l.spans.iter().map(|x| colonnes(&x.content)).sum();
        assert!(largeur <= 120, "frame {s}: {largeur} columns");
    }
}
