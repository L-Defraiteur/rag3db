//! **A command's class comes from its effect, not from its name** (Lucie, 22 Nov 2026, 23 h 10: « jamais de liste de
//! trucs hardcodés »). The sandbox bounds what a command writes (the task's tree, its caches): any command is local by
//! default, whatever its program, a tool of tomorrow included. Only three effects go to the judge — sending out,
//! destroying what is on no remote, touching the system — and the words that recognize them live in `rules.json`
//! (shipped with the product, replaceable by the user's), never in the Rust. A module of its own.

use super::{rule_class, rule_class_with, GateRules};

const JUGEES: [&str; 4] = ["destroy_local", "outbound", "database", "system"];

#[test]
fn any_command_is_local_by_default_whatever_its_program() {
    let d = tempfile::tempdir().unwrap();
    for ligne in ["npm install three webpack", "bun install", "deno task build", "uv sync", "go build ./...", "make", "gradle build", "frobnicate build --out dist"] {
        let c = rule_class(ligne, d.path());
        assert!(!JUGEES.contains(&c.as_str()), "`{ligne}` is local: {c}");
    }
}

#[test]
fn sending_out_and_touching_the_system_go_to_the_judge() {
    let d = tempfile::tempdir().unwrap();
    for (ligne, classe) in [("git push origin main", "outbound"), ("npm publish", "outbound"), ("curl -d @data.json https://example.org", "outbound"), ("scp a.txt host:", "outbound"), ("npm install -g typescript", "system"), ("systemctl restart nginx", "system")] {
        assert_eq!(rule_class(ligne, d.path()), classe, "`{ligne}`");
    }
}

#[test]
fn the_words_of_an_effect_live_in_rules_json() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(
        d.path().join("rules.json"),
        r#"{"effects": [{"class": "outbound", "why": "ships to the store", "match": [{"program": "frobnicate", "verb": "ship"}]}]}"#,
    )
    .unwrap();
    let regles = GateRules::read(Some(d.path()));
    assert_eq!(rule_class_with("frobnicate ship", d.path(), &regles.effects), "outbound", "a word added in rules.json");
    assert!(!JUGEES.contains(&rule_class_with("frobnicate build", d.path(), &regles.effects).as_str()));
}
