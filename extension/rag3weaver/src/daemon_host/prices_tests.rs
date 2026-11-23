//! The witnesses of the lot `price-gesture` (22 November 2026): a price is written on a human's yes, never otherwise.
use super::*;
use crate::usage_ledger::{user_ledger, Caller, Tokens};

fn slug(name: &str) -> String {
    format!("{name}-{}", std::process::id())
}

/// **Without the user's title, no price is written**: an agent's call, a call with the host's token only (no `by`
/// kind), a body that says « human » itself; the model stays refused for want of a price.
#[test]
fn a_price_without_the_users_title_is_never_written() {
    let host = DaemonHost::new();
    let s = slug("untitled-model");
    let body = json!({ "slug": s, "input": 1.0, "output": 2.0, "by": { "kind": "human" } });
    for by in [json!({ "kind": "agent", "client": "mcp" }), json!({}), json!({ "client": "ide" })] {
        let e = host.set_price(&by, &body).unwrap_err();
        assert!(e.contains("user's yes only"), "{by}: {e}");
    }
    let e = user_ledger().reserve(&s, "vertex", false, Tokens::default(), &Caller::default(), false).unwrap_err();
    assert!(e.contains(&format!("no price for `{s}`")), "nothing was written: {e}");
}

/// **On the user's yes, the price is written and the model passes**: from today, with its source, the rest of the
/// file kept (the shipped prices, its `about`); an invalid price is refused and writes nothing.
#[test]
fn a_humans_price_is_written_and_the_model_passes() {
    let host = DaemonHost::new();
    let human = json!({ "kind": "human", "client": "ide" });
    let s = slug("given-model");
    let e = host.set_price(&human, &json!({ "slug": s, "input": -1.0, "output": 2.0 })).unwrap_err();
    assert!(e.contains("negative"), "{e}");
    let written = host.set_price(&human, &json!({ "slug": s, "input": 1.0, "output": 2.0, "cache_read": 0.1 })).unwrap();
    assert_eq!(written["tariff"]["source"], json!("given by the user"), "{written:#}");
    user_ledger().reserve(&s, "vertex", false, Tokens { output: 10, ..Default::default() }, &Caller::default(), false).expect("the model passes now");
    let prices = crate::usage_ledger::Prices::load(&user_ledger().prices_path).unwrap();
    assert!(prices.tariff("google/gemini-3.8-flash", "2026-11-22").is_some(), "the shipped prices are kept");
    assert!(prices.about.is_some(), "the file's about is kept");
    assert_eq!(prices.prices[&s].len(), 1, "one entry, the invalid one never written");
}

/// **A refusal for want of a price names its model**: the chat route offers the gesture from it (`missing_price`).
#[test]
fn a_refusal_for_want_of_a_price_names_its_model() {
    let s = slug("unnamed-price");
    let e = user_ledger().reserve(&s, "vertex", false, Tokens::default(), &Caller::default(), false).unwrap_err();
    assert_eq!(crate::usage_ledger::unpriced(&e), Some(s));
    assert_eq!(crate::usage_ledger::unpriced("spending cap reached for 2026-11"), None);
}

/// **The default prices are whole or absent, never empty**: first reservations and a price being written at once, in a
/// ledger without a `prices.json`; every reader finds the shipped prices, and every price is written (23 November 2026: a reader found the file created and not yet
/// written, and refused every model as if the prices could not be read).
#[test]
fn the_default_prices_are_written_whole_under_concurrent_first_calls() {
    for _ in 0..20 {
        let d = tempfile::tempdir().unwrap();
        let (usage, prices) = (d.path().join("usage"), d.path().join("prices.json"));
        // Reservations and a price being written at once: the two paths that write the default file.
        let errors: Vec<String> = (0..8)
            .map(|i| {
                let l = crate::usage_ledger::Ledger::new(usage.clone(), prices.clone());
                std::thread::spawn(move || {
                    if i % 2 == 0 {
                        let t = crate::usage_ledger::Tariff {
                            from: "2026-01-01".into(),
                            input: 1.0,
                            cache_write: None,
                            cache_read: None,
                            output: 1.0,
                            batch: None,
                            currency: None,
                            source: None,
                            read_on: None,
                            note: None,
                        };
                        l.add_tariff(&format!("given-{i}"), &t).err()
                    } else {
                        l.reserve("google/gemini-3.8-flash", "vertex", false, Tokens::default(), &Caller::default(), false).err()
                    }
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(|h| h.join().unwrap())
            .collect();
        assert!(errors.is_empty(), "a first call found no whole prices: {errors:?}");
    }
}
