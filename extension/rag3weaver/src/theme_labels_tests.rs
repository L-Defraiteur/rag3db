//! **The words of the dress live in the theme** (Lucie, 22 November 2026: the welcome agent is « Accueil » by default,
//! « mais selon le thème, ajuster ces wordings »). `templates/themes/labels/neutral.json` holds every label, the fallback
//! of any key a theme does not give; a theme's `labels` only overrides keys of the neutral one; the neutral words carry
//! no plumbing. Shared by the IDE and the terminal.

use serde_json::Value;

fn json(name: &str) -> Value {
    let bytes = crate::builtin_templates::get(name).unwrap_or_else(|| panic!("{name} is not shipped"));
    serde_json::from_slice(bytes).unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[test]
fn the_neutral_labels_are_shipped_and_every_theme_overrides_only_their_keys() {
    let neutral = json("themes/labels/neutral.json");
    let keys: Vec<&String> = neutral.as_object().unwrap().keys().filter(|k| *k != "_").collect();
    for key in ["welcome_agent", "welcome_greeting", "back", "state_active", "state_waiting_you", "granted_by_judge", "decided_by_survivor"] {
        assert!(keys.iter().any(|k| *k == key), "the neutral labels give « {key} »");
    }
    assert_eq!(neutral["welcome_agent"], "Accueil", "the welcome agent is « Accueil » by default");
    for theme in ["daemon", "foret", "cyberpunk", "cyborg", "alien"] {
        let labels = json(&format!("themes/{theme}.json"))["labels"].clone();
        let labels = labels.as_object().unwrap_or_else(|| panic!("{theme} has its labels"));
        for k in labels.keys() {
            assert!(keys.iter().any(|n| *n == k), "{theme}: « {k} » is not a key of the neutral labels");
        }
    }
    assert_eq!(json("themes/daemon.json")["labels"]["welcome_agent"], "le magicien du chaos");
}

#[test]
fn the_neutral_words_carry_no_plumbing() {
    let neutral = json("themes/labels/neutral.json");
    for (k, v) in neutral.as_object().unwrap().iter().filter(|(k, _)| *k != "_") {
        let v = v.as_str().unwrap_or_default().to_lowercase();
        for word in ["backend", "manifeste", "nœud", "node", "bm25", "reflink", "pid", "run_command", "worktree"] {
            assert!(!v.contains(word), "« {k} » says « {word} »: {v}");
        }
    }
}
