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

/// **The built words find and score as lucivy's `parse` did** (the measure of 29 August 2026, in `BM25Mode::resolve`:
/// three French questions, `parse` 3/3 where `contains_split` was 0/3). On a real lucivy index, for three French
/// questions, the built query and lucivy's former `parse` of the same text return the same documents with the same
/// scores — and each finds at least one.
#[test]
fn the_built_words_find_and_score_as_lucivys_parse_did() {
    use lucivy_core::blob_store::MemBlobStore;
    use lucivy_core::query::QueryConfig;
    use lucivy_core::sharded_handle::{BlobShardStorage, ShardedHandle};
    let dir = tempfile::tempdir().unwrap();
    let fields = vec!["content".to_string(), "name".to_string()];
    let config = crate::fts_handle::build_schema_config(&fields, &[], 2).unwrap();
    let storage = BlobShardStorage::new(std::sync::Arc::new(MemBlobStore::new()), crate::fts_handle::fts_index_name("Words"), dir.path());
    let handle = ShardedHandle::create_with_storage(Box::new(storage), &config).expect("the index");
    let documents = [
        (1_u64, "connexions", "pour savoir qui est connecté sur le site, lire le journal des sessions ouvertes"),
        (2, "prix", "le prix d'un abonnement se calcule avec la remise de fin de mois"),
        (3, "index", "fermer un worktree garde son index, la vue se rouvre plus tard"),
        (4, "bruit", "des mots vides, avec un peu de tout et rien de précis"),
        (5, "sessions", "chaque session garde son nom et la date de sa dernière connexion"),
        (6, "remise", "la remise s'applique au prix avant les taxes"),
    ];
    for (offset, name, content) in documents {
        crate::fts_handle::index_document(&handle, &[("content".into(), content.to_string()), ("name".into(), name.to_string())], offset).unwrap();
    }
    handle.commit().unwrap();
    for question in ["de quoi savoir qui est connecté sur mon site", "comment se calcule le prix avec la remise", "que devient l'index quand je ferme un worktree"] {
        let parse: QueryConfig = serde_json::from_value(serde_json::json!({ "type": "parse", "fields": fields, "value": question })).unwrap();
        let built: QueryConfig = serde_json::from_str(&build_bm25_query(question, &fields, BM25Mode::Auto, 1)).unwrap();
        let hits = |q: &QueryConfig| -> Vec<(u64, f64)> {
            let mut h: Vec<(u64, f64)> = crate::fts_handle::search_hits(&handle, q, 10, None).unwrap().into_iter().map(|(o, s, _)| (o, s)).collect();
            h.sort_by(|a, b| a.0.cmp(&b.0));
            h
        };
        let (before, after) = (hits(&parse), hits(&built));
        assert!(!before.is_empty(), "{question}: lucivy's parse finds nothing — the witness proves nothing");
        assert_eq!(before.len(), after.len(), "{question}: not the same documents: {before:?} / {after:?}");
        for ((o1, s1), (o2, s2)) in before.iter().zip(&after) {
            assert_eq!(o1, o2, "{question}: not the same documents: {before:?} / {after:?}");
            assert!((s1 - s2).abs() < 1e-9, "{question}: not the same score for {o1}: {s1} / {s2}");
        }
    }
}
