//! The witnesses of the lot `attention-actions` (28 November 2026, Lucie: every line of « À intervenir » carries its
//! action): a model refused for want of a price waits there with its price card, until a price is given.

use serde_json::{json, Value};

use crate::usage_ledger::{user_ledger, Caller, Tariff, Tokens};

fn slug(name: &str) -> String {
    format!("{name}-{}", std::process::id())
}

fn state(address: &str) -> Value {
    let (status, v) = super::tests::demander("GET", &format!("http://{address}/ide/etat"), Some("j"));
    assert_eq!(status, 200, "{v}");
    v
}

/// **A model refused for want of a price waits in the host's state until it is priced**: `/ide/etat` lists it under
/// `spending.unpriced` (the page offers its price card there); a price given takes it off.
#[test]
fn a_model_refused_for_want_of_a_price_waits_until_priced() {
    let s = slug("waiting-model");
    let e = user_ledger().reserve(&s, "vertex", false, Tokens::default(), &Caller::default(), false).unwrap_err();
    assert_eq!(crate::usage_ledger::unpriced(&e), Some(s.clone()), "{e}");
    let (address, thread) = super::tests::hote_garde("j");
    let waiting = state(&address)["spending"]["unpriced"].clone();
    assert!(waiting.as_array().is_some_and(|a| a.contains(&json!(s))), "the refused model waits: {waiting}");
    let tariff = Tariff { from: "2026-01-01".into(), input: 1.0, cache_write: None, cache_read: None, output: 2.0, batch: None, currency: None, source: None, read_on: None, note: None };
    user_ledger().add_tariff(&s, &tariff).unwrap();
    let waiting = state(&address)["spending"]["unpriced"].clone();
    assert!(!waiting.as_array().is_some_and(|a| a.contains(&json!(s))), "priced, it no longer waits: {waiting}");
    let (status, _) = super::tests::demander("POST", &format!("http://{address}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}

/// **The welcome's « À intervenir » offers the refused model's price card**, and every line there carries an action.
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_attention_list_offers_the_price_card_of_a_refused_model() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binary) = navigateur_ou_saut("the_attention_list_offers_the_price_card_of_a_refused_model") else {
        return;
    };
    let s = slug("card-model");
    user_ledger().reserve(&s, "vertex", false, Tokens::default(), &Caller::default(), false).unwrap_err();
    let (address, thread) = super::tests::hote_garde("j");
    let mut n = Navigateur::lancer(&binary);
    n.aller(&format!("http://{address}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.attendre(&format!("document.querySelector('#needs-attention .price-card[data-slug=\"{s}\"]')"), "the refused model's price card in « À intervenir »");
    let lines = n.evaluer("[...document.querySelectorAll('#needs-attention [data-waiting]')].map(l => [l.dataset.waiting, !!l.querySelector('[data-action], .price-card')])");
    assert!(lines.as_array().unwrap().iter().all(|l| l[1] == json!(true)), "every line carries its action: {lines}");
    drop(n);
    let (status, _) = super::tests::demander("POST", &format!("http://{address}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}

/// **The cap and an unreadable file set aside are the user's gestures only**: without her title the host refuses both;
/// the cap is written as the one key `monthly_cap` of her file, the template's prices kept; a file that reads is never
/// set aside; an unreadable one is renamed beside itself, never deleted, and the shipped prices take over.
#[test]
fn the_cap_and_a_file_set_aside_are_the_users_gestures_only() {
    let host = super::DaemonHost::new();
    let agent = json!({ "kind": "agent", "client": "mcp" });
    assert!(host.set_cap(&agent, &json!({ "monthly_cap": 500 })).unwrap_err().contains("user's yes"));
    assert!(host.set_aside_prices(&agent).unwrap_err().contains("user's yes"));

    let d = tempfile::tempdir().unwrap();
    let l = crate::usage_ledger::Ledger::new(d.path().join("usage"), d.path().join("prices.json"));
    l.set_cap(500.0).unwrap();
    let file: Value = serde_json::from_str(&std::fs::read_to_string(&l.prices_path).unwrap()).unwrap();
    assert_eq!(file, json!({ "monthly_cap": 500.0 }), "only the cap is written");
    assert_eq!(l.report("2026-11", "2026-11-28")["cap"], json!(500.0));
    assert!(l.set_aside_prices().unwrap_err().contains("reads"), "a file that reads stays");
    std::fs::write(&l.prices_path, "{ not json").unwrap();
    let aside = l.set_aside_prices().unwrap();
    assert!(!l.prices_path.exists() && std::fs::read_to_string(&aside).unwrap() == "{ not json", "renamed, never deleted: {}", aside.display());
    l.reserve("google/gemini-3.8-flash", "vertex", false, Tokens::default(), &Caller::default(), false).expect("the shipped prices take over");
}
