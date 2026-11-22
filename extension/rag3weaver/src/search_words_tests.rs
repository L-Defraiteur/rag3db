//! **`query` is text, nothing else** (Lucie, 21 November 2026, 21 h 35: « les agents ont besoin de contrats clairs ;
//! nos booléens construits explicitement sur ce qu'on veut exactement »). No character of the query has an operator's
//! meaning: no parser of lucivy reads it. A phrase is lucivy's `contains_split` with the distance of the fuzzy search,
//! one per field (lucivy 4.4.2, 22 November 2026: one error budget per document, where a boolean of fuzzy `contains`
//! paid a penalty per clause and nothing for a missing word — lucivy warned about it on the default search).

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
    // `--daemon` and `-test` are searched as they are, a quote is a character: the whole text goes to lucivy as a
    // value, split on spaces only.
    let v = built("adresse --daemon", &["content"]).to_string();
    assert!(v.contains("\"adresse --daemon\""), "{v}");
    let v = built("daemon -test", &["content"]).to_string();
    assert!(v.contains("\"daemon -test\""), "« -test » is no longer a negation, it is searched: {v}");
    assert!(!v.contains("must_not"), "{v}");
    let v = built("say \"hi\"", &["content"]);
    assert!(v.to_string().contains(r#"\"hi\""#), "a quote is a character of the searched word: {v}");
}

#[test]
fn a_phrase_is_lucivys_contains_split_with_the_distance() {
    // One `contains_split` per field, with the distance; lucivy splits the words (those with a letter or a digit) and
    // searches a query of no such word whole.
    let v = built("Rust safety", &["content", "name"]);
    let expected = serde_json::json!({ "type": "boolean", "should": [
        { "type": "contains_split", "field": "content", "value": "Rust safety", "distance": 1 },
        { "type": "contains_split", "field": "name", "value": "Rust safety", "distance": 1 },
    ] });
    assert_eq!(v, expected);
    let v = built("alpha --", &["content"]);
    assert_eq!(v, serde_json::json!({ "type": "contains_split", "field": "content", "value": "alpha --", "distance": 1 }));
}

fn index_of(fields: &[&str], documents: &[(u64, &[&str])]) -> (tempfile::TempDir, lucivy_core::sharded_handle::ShardedHandle) {
    use lucivy_core::blob_store::MemBlobStore;
    use lucivy_core::sharded_handle::{BlobShardStorage, ShardedHandle};
    let dir = tempfile::tempdir().unwrap();
    let fields: Vec<String> = fields.iter().map(|f| f.to_string()).collect();
    let config = crate::fts_handle::build_schema_config(&fields, &[], 1).unwrap();
    let storage = BlobShardStorage::new(std::sync::Arc::new(MemBlobStore::new()), crate::fts_handle::fts_index_name("Words"), dir.path());
    let handle = ShardedHandle::create_with_storage(Box::new(storage), &config).expect("the index");
    for (offset, texts) in documents {
        let doc: Vec<(String, String)> = fields.iter().cloned().zip(texts.iter().map(|t| t.to_string())).collect();
        crate::fts_handle::index_document(&handle, &doc, *offset).unwrap();
    }
    handle.commit().unwrap();
    (dir, handle)
}

/// **The default search counts one error budget** (c7 and banc, 22 November 2026: lucivy 4.4.2 did not reach
/// `search_code`, whose phrases went to a boolean of fuzzy `contains`). On a real index, under `Auto` with the fuzzy
/// distance: lucivy has no warning about the query, and three words with a typo each come before a single exact word.
#[test]
fn the_words_of_a_phrase_count_one_error_budget() {
    use lucivy_core::query::QueryConfig;
    let (_dir, handle) = index_of(&["content"], &[(1, &["memary safty borowing checker"]), (2, &["memory and nothing else at all"]), (3, &["unrelated words only here"])]);
    let q: QueryConfig = serde_json::from_str(&build_bm25_query("memory safety borrowing", &["content".to_string()], BM25Mode::Auto, 1)).unwrap();
    let warnings = handle.query_warnings(&q);
    assert!(!warnings.iter().any(|w| w.contains("contains_split")), "lucivy warns about the query: {warnings:?}");
    let mut hits = crate::fts_handle::search_hits(&handle, &q, 10, None).unwrap();
    hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    assert_eq!(hits.first().map(|h| h.0), Some(1), "three words with a typo each come first: {:?}", hits.iter().map(|h| (h.0, h.1)).collect::<Vec<_>>());
}

/// **Each French question still finds** (the measure of 29 August 2026, in `BM25Mode::resolve`: three French
/// questions, the words 3/3). On a real lucivy index, the words of each question find the document it asks for first.
#[test]
fn each_french_question_finds_its_document_first() {
    use lucivy_core::query::QueryConfig;
    let fields = ["content", "name"];
    let documents: [(u64, &[&str]); 6] = [
        (1, &["pour savoir qui est connecté sur le site, lire le journal des sessions ouvertes", "connexions"]),
        (2, &["le prix d'un abonnement se calcule avec la remise de fin de mois", "prix"]),
        (3, &["fermer un worktree garde son index, la vue se rouvre plus tard", "index"]),
        (4, &["des mots vides, avec un peu de tout et rien de précis", "bruit"]),
        (5, &["chaque session garde son nom et la date de sa dernière connexion", "sessions"]),
        (6, &["la remise s'applique au prix avant les taxes", "remise"]),
    ];
    let (_dir, handle) = index_of(&fields, &documents);
    let fields: Vec<String> = fields.iter().map(|f| f.to_string()).collect();
    for (question, expected) in [("de quoi savoir qui est connecté sur mon site", 1), ("comment se calcule le prix avec la remise", 2), ("que devient l'index quand je ferme un worktree", 3)] {
        let q: QueryConfig = serde_json::from_str(&build_bm25_query(question, &fields, BM25Mode::Auto, 1)).unwrap();
        let mut hits = crate::fts_handle::search_hits(&handle, &q, 10, None).unwrap();
        hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        assert_eq!(hits.first().map(|h| h.0), Some(expected), "{question}: {:?}", hits.iter().map(|h| (h.0, h.1)).collect::<Vec<_>>());
    }
}
