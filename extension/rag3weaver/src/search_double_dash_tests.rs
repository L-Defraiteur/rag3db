//! **A word starting with two dashes is a term, not a negation** (Lucie, 21 November 2026, 21 h 25: the defects of the
//! MCP are the agent's, so they block the v1; ticket
//! `docs/recherche/tickets/2026-11-21-un-mot-qui-commence-par-deux-tirets-est-lu-comme-une-negation.md`). An agent who
//! looks for `--save-dev`, `--features` or `--daemon` lost the search by words: the parser read `--daemon` as the
//! negation of `-daemon`, and a query of negations only was refused.

use super::*;

/// The JSON values of all clauses of a built query, as text.
fn values(built: &str) -> String {
    built.to_string()
}

#[test]
fn a_word_starting_with_two_dashes_is_a_term_not_a_negation() {
    let fields = vec!["content".to_string()];
    for (query, term) in [("adresse --daemon", "daemon"), ("--features", "features"), ("npm install --save-dev", "save-dev")] {
        let built = build_bm25_query(query, &fields, BM25Mode::Auto, 1);
        assert!(!values(&built).contains("--"), "{query}: the query still carries `--`, read as a negation by the parser: {built}");
        assert!(values(&built).contains(term), "{query}: the term `{term}` is not searched: {built}");
    }
    // A real negation stays one: `-word` after a space.
    let built = build_bm25_query("daemon -test", &fields, BM25Mode::Auto, 1);
    assert!(built.contains("-test") && built.contains("\"parse\""), "a negation `-test` is no longer one: {built}");
    // An explicit mode keeps the query as it is (a substring or a regex may want the dashes).
    let built = build_bm25_query("--daemon", &fields, BM25Mode::Contains, 1);
    assert!(built.contains("--daemon"), "an explicit contains lost its dashes: {built}");
}
