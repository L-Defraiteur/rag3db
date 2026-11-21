//! The witnesses of the lot `billing-ledger` (21 November 2026): the spending ledger and the monthly cap.
use super::*;
use crate::llm::{CallbackLlm, FinishReason, StringSink};

/// A ledger of its own, with `prices` as its `prices.json` (`None`: no file).
fn ledger(prices: Option<&str>) -> (tempfile::TempDir, Arc<Ledger>) {
    let d = tempfile::tempdir().unwrap();
    if let Some(p) = prices {
        std::fs::write(d.path().join("prices.json"), p).unwrap();
    }
    let l = Arc::new(Ledger::new(d.path().join("usage"), d.path().join("prices.json")));
    (d, l)
}

/// One dollar a million input tokens, two the output, a tenth the cache read, 1.25 the cache write; a cap of 200 USD.
const PRICES: &str = r#"{"currency":"USD","monthly_cap":200,"prices":{"paid":[
    {"from":"2026-01-01","input":1,"cache_write":1.25,"cache_read":0.1,"output":2}]}}"#;

/// A model that answers with this usage.
fn model(usage: Usage) -> Arc<dyn Llm> {
    Arc::new(CallbackLlm::new(move |_t, _o, sink| {
        let finish = Finish::new(FinishReason::MaxTokens, Vec::new());
        sink.on_finish(&finish);
        Ok((finish, usage))
    }))
}

fn call(llm: &dyn Llm, max_tokens: usize) -> Result<(Finish, Usage), LlmError> {
    let opts = GenOptions { max_tokens, ..Default::default() };
    llm.generate(&[Turn::user("bonjour")], &opts, &mut StringSink::default())
}

fn month() -> String {
    utc_day(now_secs())[..7].to_string()
}

/// **A call is reserved, then settled from the provider's `usage`**, the cache written and read apart; the month's cost
/// is the settle's, at the slug's tariff.
#[test]
fn a_call_is_reserved_then_settled_from_its_usage() {
    let (_d, l) = ledger(Some(PRICES));
    let usage = Usage { prompt_tokens: 1_000_000, cached_prompt_tokens: 200_000, cache_write_tokens: 100_000, completion_tokens: 50_000, ..Default::default() };
    let m = MeteredLlm::new(model(usage), l.clone(), "paid", "vertex", false);
    call(&m, 1_000).unwrap();
    let (lines, bad) = l.lines(&month());
    assert!(bad.is_empty());
    assert_eq!(lines.iter().map(|x| x.kind.as_str()).collect::<Vec<_>>(), ["reserve", "settle"]);
    assert_eq!(lines[0].id, lines[1].id);
    assert_eq!(lines[1].tokens, Some(Tokens { input: 700_000, cache_write: 100_000, cache_read: 200_000, output: 50_000 }));
    let prices = Prices::load(&l.prices_path).unwrap();
    let spent = l.spent(&prices, &month());
    // 0.7 + 0.125 + 0.02 + 0.1
    assert!((spent.total - 0.945).abs() < 1e-9, "{}", spent.total);
    assert_eq!(spent.open, 0);
}

/// **A reply without `usage` counts at its bound**, and so does a reservation never settled: never at zero.
#[test]
fn a_reply_without_usage_and_an_open_reservation_count_at_their_bound() {
    let (_d, l) = ledger(Some(PRICES));
    let m = MeteredLlm::new(model(Usage::default()), l.clone(), "paid", "vertex", false);
    call(&m, 500_000).unwrap();
    let prices = Prices::load(&l.prices_path).unwrap();
    let after_one = l.spent(&prices, &month()).total;
    assert!(after_one >= 1.0, "the bound of 500 000 output tokens at 2 USD: {after_one}");
    l.reserve("paid", "vertex", false, Tokens { output: 500_000, ..Default::default() }, &Caller::default(), false).unwrap();
    let spent = l.spent(&prices, &month());
    assert_eq!(spent.open, 1, "a reservation without its settle");
    assert!((spent.total - after_one - 1.0).abs() < 1e-9, "{} after {after_one}", spent.total);
}

/// **Two writers at the edge of the cap: only one passes** (the reservation under `flock`). Each thread opens the files
/// itself, as two processes would: `flock` holds per open file.
#[test]
fn two_writers_at_the_edge_of_the_cap_only_one_passes() {
    let (_d, l) = ledger(Some(r#"{"currency":"USD","monthly_cap":100,"prices":{"paid":[{"from":"2026-01-01","input":0,"output":1}]}}"#));
    for _ in 0..20 {
        std::fs::remove_dir_all(&l.dir).ok();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let passed: Vec<bool> = (0..2)
            .map(|_| {
                let (l, b) = (Ledger::new(l.dir.clone(), l.prices_path.clone()), barrier.clone());
                std::thread::spawn(move || {
                    b.wait();
                    l.reserve("paid", "vertex", false, Tokens { output: 60_000_000, ..Default::default() }, &Caller::default(), false).is_ok()
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect();
        assert_eq!(passed.iter().filter(|p| **p).count(), 1, "60 and 60 over a cap of 100: {passed:?}");
    }
    // Many writers at once: every line stays whole.
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let l = Ledger::new(l.dir.clone(), l.prices_path.clone());
            std::thread::spawn(move || {
                for _ in 0..50 {
                    l.reserve("local", "http://127.0.0.1:1/v1", true, Tokens::default(), &Caller::default(), false).unwrap();
                }
            })
        })
        .collect();
    handles.into_iter().for_each(|h| h.join().unwrap());
    let (lines, bad) = l.lines(&month());
    assert!(bad.is_empty(), "lines mixed: {bad:?}");
    assert_eq!(lines.len(), 401);
}

/// **What is refused, said**: a slug without a price, a prices file that cannot be read, a tariff in another currency
/// than the cap. A model at a private address passes without any prices, written as free.
#[test]
fn a_slug_without_a_price_unreadable_prices_and_another_currency_are_refused() {
    let (_d, l) = ledger(Some(PRICES));
    let e = l.reserve("unknown", "vertex", false, Tokens::default(), &Caller::default(), false).unwrap_err();
    assert!(e.contains("no price for `unknown`") && e.contains("never counted at zero"), "{e}");
    let (_d, l) = ledger(None);
    let e = l.reserve("paid", "vertex", false, Tokens::default(), &Caller::default(), false).unwrap_err();
    assert!(e.contains("prices.json") && e.contains("a paid call is refused"), "{e}");
    let r = l.reserve("qwen", "http://192.168.1.20:8080/v1", true, Tokens::default(), &Caller::default(), false).unwrap();
    assert!(r.line.free);
    let (_d, l) = ledger(Some("{ not json"));
    assert!(l.reserve("paid", "vertex", false, Tokens::default(), &Caller::default(), false).is_err());
    let (_d, l) = ledger(Some(r#"{"currency":"USD","monthly_cap":200,"prices":{"paid":[{"from":"2026-01-01","input":1,"output":1,"currency":"EUR"}]}}"#));
    let e = l.reserve("paid", "vertex", false, Tokens::default(), &Caller::default(), false).unwrap_err();
    assert!(e.contains("EUR") && e.contains("never converted"), "{e}");
    assert!(is_private_address("http://127.0.0.1:8080/v1") && is_private_address("http://luciepc:7983/v1"));
    assert!(!is_private_address("https://api.anthropic.com") && !is_private_address("https://8.8.8.8/v1"));
}

/// **A tariff that changes on 1 December**: a call of 30 November and one of 1 December each count at their own.
#[test]
fn a_tariff_that_changes_counts_each_call_at_the_tariff_of_its_day() {
    let (_d, l) = ledger(Some(
        r#"{"currency":"USD","monthly_cap":200,"prices":{"argon":[
            {"from":"2026-09-01","input":0,"output":1},{"from":"2026-12-01","input":0,"output":3}]}}"#,
    ));
    let k = Tokens { output: 1_000_000, ..Default::default() };
    for day in ["2026-11-30", "2026-12-01"] {
        let r = l.reserve_on(day.into(), "argon", "vertex", false, k, &Caller::default(), false).unwrap();
        l.settle(&r, Some(k)).unwrap();
    }
    let prices = Prices::load(&l.prices_path).unwrap();
    assert!((l.spent(&prices, "2026-11").total - 1.0).abs() < 1e-9);
    assert!((l.spent(&prices, "2026-12").total - 3.0).abs() < 1e-9);
}

/// **Beyond the cap, an agent's call is refused, a human's own passes** and is counted.
#[test]
fn beyond_the_cap_an_agent_is_refused_and_a_human_passes() {
    let (_d, l) = ledger(Some(r#"{"currency":"USD","monthly_cap":1,"prices":{"paid":[{"from":"2026-01-01","input":0,"output":1}]}}"#));
    let usage = Usage { prompt_tokens: 10, completion_tokens: 900_000, ..Default::default() };
    let m = MeteredLlm::new(model(usage), l.clone(), "paid", "vertex", false);
    call(&m, 900_000).unwrap();
    let e = call(&m, 900_000).err().expect("the cap refuses an agent");
    assert!(matches!(&e, LlmError::Unavailable { quota: Quota::Exhausted, .. }), "{e}");
    assert!(e.to_string().contains("spending cap reached"), "{e}");
    let _human = attribute(Caller { human: true, session: "s".into(), ..Default::default() });
    call(&m, 900_000).expect("a human's own call passes");
    let prices = Prices::load(&l.prices_path).unwrap();
    assert!((l.spent(&prices, &month()).total - 1.8).abs() < 1e-9);
}

/// **Every declared model is metered**: a paid model built by `connect_llm` is refused before the network when its slug
/// has no price.
#[cfg(feature = "openai-llm")]
#[test]
fn a_declared_model_goes_through_the_ledger() {
    let (_d, l) = ledger(Some(PRICES));
    let declaration: crate::model_source::ModelSource = serde_json::from_value(serde_json::json!({
        "provider": "compatible", "protocol": "vertex", "model": "unpriced-model", "params": {"project": "p"}
    }))
    .unwrap();
    let (llm, _) = crate::model_source::connect_llm_with_vertex(&declaration, l, |_, _, model| {
        Ok(crate::openai_llm::OpenAiLlm::new("http://127.0.0.1:9/v1", model))
    })
    .unwrap();
    let e = call(&*llm, 10).err().expect("refused by the ledger, before the network");
    assert!(e.to_string().contains("no price for `unpriced-model`"), "{e}");
}

/// **No paid client is built around the ledger**: outside tests, the clients are built in `model_source.rs`, and the
/// regime's model in `regime.rs`, which the product takes only through `regime::metered`.
#[test]
fn no_paid_client_is_built_outside_the_metered_paths() {
    let builds = ["OpenAiLlm::new(", "OpenAiLlm::vertex", "OpenAiLlm::from_env(", "AnthropicLlm::new(", "AnthropicLlm::from_env("];
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let rel = p.strip_prefix(&root).unwrap().to_string_lossy().to_string();
            if !rel.ends_with(".rs") || rel.contains("tests") {
                continue;
            }
            let text = std::fs::read_to_string(&p).unwrap();
            let lines: Vec<&str> = text.lines().collect();
            let end = lines.iter().position(|l| l.trim() == "#[cfg(test)]").unwrap_or(lines.len());
            for (i, l) in lines[..end].iter().enumerate().filter(|(_, l)| !l.trim_start().starts_with("//")) {
                let built = builds.iter().any(|b| l.contains(b)) && !["model_source.rs", "regime.rs"].contains(&rel.as_str());
                // The regime's model is taken through `regime::metered`, within the same statement.
                let unmetered = l.contains("regime::modele_agentique")
                    && !lines[i..(i + 12).min(end)].iter().any(|n| n.contains("regime::metered("));
                if built || unmetered {
                    offenders.push(format!("{rel}:{}: {}", i + 1, l.trim()));
                }
            }
        }
    }
    assert!(offenders.is_empty(), "a paid model built around the spending ledger:\n{}", offenders.join("\n"));
}
