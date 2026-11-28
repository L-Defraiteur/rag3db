//! The witnesses of the lot `prices-from-template` (28 November 2026, Lucie: a new install has no `prices.json` in its
//! config and saw « le plafond des dépenses ne se lit pas »): the prices are the shipped template's, and the user's
//! file only overrides them, key by key and slug by slug.
use super::*;

fn ledger(prices: Option<&str>) -> (tempfile::TempDir, Ledger) {
    let d = tempfile::tempdir().unwrap();
    if let Some(p) = prices {
        std::fs::write(d.path().join("prices.json"), p).unwrap();
    }
    let l = Ledger::new(d.path().join("usage"), d.path().join("prices.json"));
    (d, l)
}

/// **Without a `prices.json`, the shipped prices hold, and nothing is said or written**: the month's report reads the
/// template's cap and currency, no alert waits in « À intervenir », a model the template prices passes, and the config
/// stays without a copy (a copy would keep yesterday's prices).
#[test]
fn without_a_prices_file_the_shipped_cap_holds_without_an_alert() {
    let (_d, l) = ledger(None);
    let r = l.report("2026-11", "2026-11-28");
    assert!(r["error"].is_null(), "the month's report reads: {r}");
    assert_eq!((r["cap"].as_f64(), r["currency"].as_str()), (Some(200.0), Some("USD")), "{r}");
    assert_eq!(l.alert_on("2026-11-28"), serde_json::Value::Null, "nothing waits in « À intervenir »");
    l.reserve("google/gemini-3.8-flash", "vertex", false, Tokens::default(), &Caller::default(), false).expect("a shipped model passes");
    assert!(!l.prices_path.exists(), "no copy of the template written in the config");
}

/// **The user's file overrides the template, key by key and slug by slug**: a cap of 50 keeps the template's currency
/// and its prices; a slug the file prices is its own, a slug it does not is the template's; an unreadable file still
/// refuses every paid call, naming it.
#[test]
fn the_users_prices_override_the_template_key_by_key() {
    let (_d, l) = ledger(Some(r#"{"monthly_cap": 50, "prices": {"given": [{"from": "2026-01-01", "input": 1, "output": 2}]}}"#));
    let r = l.report("2026-11", "2026-11-28");
    assert_eq!((r["cap"].as_f64(), r["currency"].as_str()), (Some(50.0), Some("USD")), "{r}");
    l.reserve("given", "vertex", false, Tokens::default(), &Caller::default(), false).expect("the user's own price");
    l.reserve("google/gemini-3.8-flash", "vertex", false, Tokens::default(), &Caller::default(), false).expect("the template's price, kept");
    let (_d, l) = ledger(Some("{ not json"));
    let e = l.reserve("google/gemini-3.8-flash", "vertex", false, Tokens::default(), &Caller::default(), false).unwrap_err();
    assert!(e.contains("prices.json") && e.contains("cannot be read"), "{e}");
}
