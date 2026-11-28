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
