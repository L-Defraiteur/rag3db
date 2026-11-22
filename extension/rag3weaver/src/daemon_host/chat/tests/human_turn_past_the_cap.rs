//! The witness of the lot `billing-views` (22 November 2026): a human's chat turn passes the cap.
use super::*;
use crate::usage_ledger::{Caller, Ledger, MeteredLlm, Tokens};

/// **Past the monthly cap, the user's own chat turn passes and is counted; a turn the host does not sign
/// as a human's is refused, said so.** The month is at its cap before either turn.
#[test]
fn a_humans_chat_turn_goes_past_the_cap_and_an_unsigned_one_is_refused() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(
        d.path().join("prices.json"),
        r#"{"currency":"USD","monthly_cap":1,"prices":{"paid":[{"from":"2026-01-01","input":0,"output":1}]}}"#,
    )
    .unwrap();
    let ledger = Arc::new(Ledger::new(d.path().join("usage"), d.path().join("prices.json")));
    let full = Tokens { output: 1_000_000, ..Default::default() };
    let r = ledger.reserve("paid", "vertex", false, full, &Caller { human: true, ..Default::default() }, false).unwrap();
    ledger.settle(&r, Some(full)).unwrap();
    let l = ledger.clone();
    let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(MeteredLlm::new(Arc::new(MockLlm::new("an answer")), l.clone(), "paid", "vertex", false)))
    });
    let (_d, host, _m, id) = banc_avec(model, |_| {});
    let says_cap = |r: &Result<Value, String>| match r {
        Err(e) => e.contains("spending cap"),
        Ok(v) => v.to_string().contains("spending cap"),
    };
    let unsigned = host.chat_turn_by(&id, "a question", &json!({ "kind": "agent", "client": "mcp" }));
    assert!(says_cap(&unsigned), "an unsigned turn past the cap is refused, said so: {unsigned:?}");
    let human = host.chat_turn_by(&id, "a question", &json!({ "kind": "human", "client": "ide" }));
    assert!(human.is_ok() && !says_cap(&human), "the user's own turn passes: {human:?}");
    let month = crate::usage_ledger::utc_day(
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64,
    )[..7]
        .to_string();
    let (lines, _) = ledger.lines(&month);
    assert!(
        lines.iter().any(|x| x.kind == "settle" && x.caller.human && x.caller.session == format!("chat:{id}")),
        "the human's turn is counted, said whose: {lines:#?}"
    );
}
