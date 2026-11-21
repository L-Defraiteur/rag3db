//! **`query` is text, nothing else** (Lucie, 21 November 2026, 21 h 35: « les agents ont besoin de contrats clairs ;
//! nos booléens construits explicitement sur ce qu'on veut exactement »). No character of the query has an operator's
//! meaning: no parser of lucivy reads it. A phrase is the boolean built explicitly — one `contains` per word and per
//! field, in `should` — the exact query lucivy built from a plain phrase before, so the same results and scores.

use super::*;

fn built(query: &str, fields: &[&str]) -> serde_json::Value {
    let fields: Vec<String> = fields.iter().map(|f| f.to_string()).collect();
    serde_json::from_str(&build_bm25_query(query, &fields, BM25Mode::Auto, 1)).unwrap()
}

#[test]
fn no_query_goes_to_a_parser_of_lucivy() {
    for query in ["adresse --daemon", "npm install --save-dev", "daemon -test", "say \"hi\"", "a AND b", "(x OR y)", "+must"] {
        let v = built(query, &["content"]);
        assert!(!v.to_string().contains("\"parse\""), "{query}: the query goes to lucivy's parser: {v}");
    }
}

#[test]
fn every_character_of_the_query_is_text() {
    // `--daemon` and `-test` are searched as they are, a quote is a character.
    let v = built("adresse --daemon", &["content"]).to_string();
    assert!(v.contains("\"--daemon\"") && v.contains("\"adresse\""), "{v}");
    let v = built("daemon -test", &["content"]).to_string();
    assert!(v.contains("\"-test\""), "« -test » is no longer a negation, it is searched: {v}");
    assert!(!v.contains("must_not"), "{v}");
    let v = built("say \"hi\"", &["content"]);
    assert!(v.to_string().contains(r#"\"hi\""#), "a quote is a character of the searched word: {v}");
}

#[test]
fn a_phrase_is_the_boolean_lucivy_built_from_it() {
    // The same query as lucivy's `expand_parse_simple`: per field, per word with a letter or a digit, a `contains`.
    let v = built("Rust safety", &["content", "name"]);
    let expected = serde_json::json!({ "type": "boolean", "should": [
        { "type": "contains", "field": "content", "value": "Rust" },
        { "type": "contains", "field": "content", "value": "safety" },
        { "type": "contains", "field": "name", "value": "Rust" },
        { "type": "contains", "field": "name", "value": "safety" },
    ] });
    assert_eq!(v, expected);
    // A word without a letter or a digit is not a clause; a query of such words is searched whole.
    let v = built("alpha --", &["content"]);
    assert_eq!(v, serde_json::json!({ "type": "contains", "field": "content", "value": "alpha" }));
    let v = built("-- ++", &["content"]);
    assert_eq!(v, serde_json::json!({ "type": "contains", "field": "content", "value": "-- ++" }));
}
