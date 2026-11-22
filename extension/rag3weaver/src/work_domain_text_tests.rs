//! The witnesses of `TextConstraints` and of the case-folded `contains` (22 November 2026), in a file of their own.

use super::*;
use crate::dialect::SchemaDialect;

#[test]
fn a_list_of_words_is_read_as_a_tool_receives_it() {
    let w = |v: serde_json::Value| TextConstraints::words("exclude", Some(&v));
    assert_eq!(w(serde_json::json!(["test", "", "mock"])).unwrap(), ["test", "mock"]);
    assert_eq!(w(serde_json::json!("[\"test\"]")).unwrap(), ["test"], "a JSON list written as a string");
    assert_eq!(w(serde_json::json!("test")).unwrap(), ["test"], "one word");
    assert!(w(serde_json::json!("")).unwrap().is_empty());
    assert!(TextConstraints::words("exclude", None).unwrap().is_empty());
    assert!(w(serde_json::json!([1])).unwrap_err().contains("exclude"), "a number is not a word");
}

#[test]
fn the_rule_in_memory_is_the_rule_of_the_filter() {
    let c = TextConstraints { exclude: vec!["test".into()], must: vec!["frob".into()], phrase: vec!["widget gear".into()], exact: false };
    assert!(c.admits("FROB the Widget Gear"));
    assert!(!c.admits("frob the widget gear in a TEST"), "« TEST » is « test » without the case");
    assert!(!c.admits("frob the widget"), "the phrase is missing");
    let exact = TextConstraints { exact: true, ..c.clone() };
    assert!(!exact.admits("FROB the Widget Gear"), "with the case, « FROB » is not « frob »");
    assert!(exact.admits("frob the widget gear in a TEST"), "with the case, « TEST » is not « test »");
    assert!(TextConstraints::default().to_filter("content").is_none());
    let Some(FilterCondition::Must(all)) = c.to_filter("content") else { panic!("a Must") };
    assert_eq!(all.len(), 3, "must, phrase, then the MustNot of exclude: {all:?}");
    assert!(matches!(&all[2], FilterCondition::MustNot(n) if n.len() == 1));
}

#[test]
fn the_line_says_what_was_searched() {
    let c = TextConstraints { exclude: vec!["test".into(), "mock".into()], must: vec![], phrase: vec!["widget gear".into()], exact: false };
    assert_eq!(
        c.describe("alpha").unwrap(),
        "cherché : « alpha » ; la phrase « widget gear » ; sans « test », « mock » ; sans tenir compte de la casse"
    );
    assert!(TextConstraints::default().describe("alpha").is_none());
}

#[test]
fn a_folded_contains_lowers_the_field() {
    assert_eq!(crate::dialect::Rag3dbDialect.filter_contains_fold("n.content", "p0"), "contains(lower(n.content), $p0)");
    assert_eq!(crate::dialect::PostgresDialect.filter_contains_fold("m.content", "p0"), "lower(m.content) LIKE '%' || $p0 || '%'");
}
