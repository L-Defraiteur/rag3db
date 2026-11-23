//! The witnesses of the lot `billing-ledger` (21 November 2026): the spending ledger and the monthly cap.
use super::*;
use crate::llm::{CallbackLlm, StringSink};
use serde_json::json;

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
    Arc::new(CallbackLlm::new("model", 100_000, move |_t: &[Turn], _o: &GenOptions, sink: &mut dyn TokenSink| {
        sink.on_finish(&Finish::eos());
        Ok((Finish::eos(), usage))
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
fn two_writers_at_the_edge_of_the_cap_only_one_goes_through() {
    let (_d, l) = ledger(Some(r#"{"currency":"USD","monthly_cap":100,"free_private_hosts":true,"prices":{"paid":[{"from":"2026-01-01","input":0,"output":1}]}}"#));
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

/// **What goes on and what is refused**: a slug without a price goes on at a provisional price (never at zero); a
/// prices file that cannot be read and a tariff in another currency than the cap are refused. Without a file, the
/// ledger writes the default one, which frees the private addresses: a local model is free because the file says so,
/// never by default.
#[test]
fn a_slug_without_a_price_goes_on_and_unreadable_prices_and_another_currency_are_refused() {
    let (_d, l) = ledger(Some(PRICES));
    l.reserve("unknown", "vertex", false, Tokens::default(), &Caller::default(), false).expect("a provisional price, the work goes on");
    let t = Prices::load(&l.prices_path).unwrap().tariff("unknown", &utc_day(now_secs())).cloned().unwrap();
    assert!(t.is_provisional() && t.input > 0.0 && t.output > 0.0, "never at zero: {t:?}");
    let (_d, l) = ledger(None);
    l.reserve("google/gemini-3.8-flash", "vertex", false, Tokens::default(), &Caller::default(), false)
        .expect("the shipped prices know the regime's default model");
    let written = Prices::load(&l.prices_path).expect("the default file is written");
    assert_eq!(written, Prices::default_file());
    assert_eq!((written.currency.as_str(), written.monthly_cap, written.free_private_hosts), ("USD", 200.0, true));
    let r = l.reserve("qwen", "http://192.168.1.20:8080/v1", true, Tokens::default(), &Caller::default(), false).unwrap();
    assert!(r.line.free, "free because the written file says so");
    let (_d, l) = ledger(Some(r#"{"currency":"USD","monthly_cap":200,"prices":{"paid":[{"from":"2026-01-01","input":1,"output":2}]}}"#));
    let r = l.reserve("qwen", "http://127.0.0.1:8080/v1", true, Tokens::default(), &Caller::default(), false).unwrap();
    assert!(!r.line.free, "a file that does not free the private addresses: a provisional price, not free");
    let (_d, l) = ledger(Some("{ not json"));
    let e = l.reserve("qwen", "http://127.0.0.1:8080/v1", true, Tokens::default(), &Caller::default(), false).unwrap_err();
    assert!(e.contains("a paid call is refused while the prices cannot be read"), "an unreadable file frees nothing: {e}");
    assert_eq!(std::fs::read_to_string(&l.prices_path).unwrap(), "{ not json", "never written over");
    let (_d, l) = ledger(Some(r#"{"currency":"USD","monthly_cap":200,"prices":{"paid":[{"from":"2026-01-01","input":1,"output":1,"currency":"EUR"}]}}"#));
    let e = l.reserve("paid", "vertex", false, Tokens::default(), &Caller::default(), false).unwrap_err();
    assert!(e.contains("EUR") && e.contains("never converted"), "{e}");
    assert!(is_private_address("http://127.0.0.1:8080/v1") && is_private_address("http://luciepc:7983/v1"));
    assert!(!is_private_address("https://api.anthropic.com") && !is_private_address("https://8.8.8.8/v1"));
}

/// **The shipped prices are whole and dated**: they parse and pass the checks, every tariff names its source and the
/// day it was read, and Argon's rise of 1 December 2026 is an entry of its own.
#[test]
fn the_shipped_prices_name_their_source_and_carry_argons_rise() {
    let p = Prices::default_file();
    p.check().unwrap();
    for (slug, tariffs) in &p.prices {
        for t in tariffs {
            assert!(t.source.as_deref().is_some_and(|s| s.starts_with("https://")) && t.read_on.as_deref().is_some_and(is_day), "{slug} {}", t.from);
        }
    }
    let argon = |day: &str| p.tariff("google/gemini-4-argon", day).unwrap().output;
    assert_eq!((argon("2026-11-30"), argon("2026-12-01")), (10.0, 20.0));
    assert!(p.tariff(crate::regime::MODELE_VERTEX_PAR_DEFAUT, "2026-11-21").is_some(), "the regime's default model has a price");
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
fn beyond_the_cap_an_agent_is_refused_and_a_human_goes_through() {
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

/// **Every declared model is metered**: a paid model built by `connect_llm` reserves before the network, at a
/// provisional price when its slug has none; the call then fails on the network, never on the ledger.
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
    let ledger = Ledger::new(_d.path().join("usage"), _d.path().join("prices.json"));
    let e = call(&*llm, 10).err().expect("nothing answers at that address");
    assert!(!e.to_string().contains("no price") && !e.to_string().contains("spending cap"), "{e}");
    let (lines, _) = ledger.lines(&month());
    assert!(lines.iter().any(|x| x.slug == "unpriced-model" && x.kind == "reserve"), "reserved before the network: {lines:#?}");
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

/// **The gate's local judge goes through the ledger at price zero** (c7, 22 November 2026): a model declared at a
/// loopback or private address (the gate's Qwen at 127.0.0.1:8096, a service at luciepc) is metered, written as free,
/// and never refused for want of a price, under the prices shipped with the crate. Were it refused, every class the
/// gate judges would go to the card without saying why.
#[cfg(feature = "openai-llm")]
#[test]
fn the_gates_local_judge_goes_through_the_ledger_at_price_zero() {
    for address in ["http://127.0.0.1:8096/v1", "http://luciepc:7983/v1"] {
        let (_d, l) = ledger(None);
        let declaration: crate::model_source::ModelSource = serde_json::from_value(serde_json::json!({
            "provider": "compatible", "model": "qwen3.5-9b", "address": address
        }))
        .unwrap();
        let (llm, _) = crate::model_source::connect_llm_with_vertex(&declaration, l.clone(), |_, _, _| unreachable!()).unwrap();
        // Nothing answers at that address: the call fails on the network, never on the ledger.
        let opts = GenOptions { max_tokens: 8, ..Default::default() }.with_retry_budget(std::time::Duration::from_millis(1));
        if let Err(e) = llm.generate(&[Turn::user("bonjour")], &opts, &mut StringSink::default()) {
            let e = e.to_string();
            assert!(!e.contains("spending cap") && !e.contains("no price"), "{address}: refused by the ledger: {e}");
        }
        let (lines, _) = l.lines(&month());
        assert!(!lines.is_empty() && lines.iter().all(|x| x.free && x.slug == "qwen3.5-9b"), "{address}: {lines:?}");
    }
}

/// **The month's report** (`usage_report`, and its line in `daemon_status`): the total against the cap, the projection
/// on the days elapsed, and the calls by slug, by day and by caller.
#[test]
fn the_months_report_says_the_total_the_projection_and_who_spent() {
    let (_d, l) = ledger(Some(
        r#"{"currency":"USD","monthly_cap":200,"free_private_hosts":true,"prices":{"paid":[{"from":"2026-01-01","input":1,"output":2}]}}"#,
    ));
    let call = |day: &str, slug: &str, provider: &str, private: bool, k: Tokens, caller: Caller| {
        let r = l.reserve_on(day.into(), slug, provider, private, k, &caller, false).unwrap();
        l.settle(&r, Some(k)).unwrap();
    };
    let agent = Caller { agent: "agent:teammate".into(), ..Default::default() };
    call("2026-11-02", "paid", "vertex", false, Tokens { input: 1_000_000, output: 1_000_000, ..Default::default() }, agent.clone());
    call("2026-11-05", "paid", "vertex", false, Tokens { output: 5_000_000, ..Default::default() }, Caller { human: true, ..Default::default() });
    call("2026-11-05", "qwen", "http://127.0.0.1:8096/v1", true, Tokens { input: 500, output: 3, ..Default::default() }, agent);
    let r = l.report("2026-11", "2026-11-10");
    assert_eq!(r["spent"], json!(13.0), "{r:#}");
    assert_eq!(r["projection"], json!(39.0), "13 in 10 days of 30: {r:#}");
    assert_eq!((r["by_slug"]["paid"]["calls"].clone(), r["by_slug"]["paid"]["output"].clone()), (json!(2), json!(6_000_000)));
    assert_eq!((r["by_slug"]["qwen"]["cost"].clone(), r["by_slug"]["qwen"]["free"].clone()), (json!(0.0), json!(true)));
    assert_eq!((r["by_day"]["2026-11-02"].clone(), r["by_day"]["2026-11-05"].clone()), (json!(3.0), json!(10.0)));
    assert_eq!((r["by_caller"]["agent:teammate"].clone(), r["by_caller"]["human"].clone()), (json!(3.0), json!(10.0)));
    assert_eq!(r["percent"], json!(6.5));
    assert_eq!(l.report("2026-11", "2026-12-01")["projection"], json!(13.0), "a month over is its total");
}

/// **A script's call** (`usage check`, `usage record`): `check` says, without writing, whether a call of that bound
/// passes the cap; `record` writes a call already made, never refused (the money is spent), and a slug without a price
/// is written and said, then listed as unpriced.
#[test]
fn a_scripts_call_is_checked_without_writing_and_recorded_without_refusal() {
    let (_d, l) = ledger(Some(r#"{"currency":"USD","monthly_cap":1,"prices":{"paid":[{"from":"2026-01-01","input":0,"output":1}]}}"#));
    let script = Caller { agent: "bench".into(), ..Default::default() };
    let k = Tokens { output: 600_000, ..Default::default() };
    assert_eq!(l.check("paid", false, k, &script, false), Ok(false), "0.60 under a cap of 1");
    assert!(l.lines(&month()).0.is_empty(), "check writes nothing");
    assert_eq!(l.record("paid", "vertex", false, k, &script, false), Ok(None));
    let e = l.check("paid", false, k, &script, false).unwrap_err();
    assert!(e.contains("spending cap reached"), "0.60 + 0.60 over 1: {e}");
    assert_eq!(l.record("paid", "vertex", false, k, &script, false), Ok(None), "a call made is written, over the cap too");
    let w = l.record("gemini-unknown", "vertex", false, k, &script, false).unwrap().expect("said");
    assert!(w.contains("no price for `gemini-unknown`"), "{w}");
    let prices = Prices::load(&l.prices_path).unwrap();
    let spent = l.spent(&prices, &month());
    assert!((spent.total - 1.2).abs() < 1e-9 && spent.open == 0, "{spent:?}");
    assert_eq!(spent.unpriced, vec!["gemini-unknown".to_string()]);
}

/// **The month's alert** (« À intervenir »): nothing under 50 % with a projection under the cap; 50 %, then 80 %, then
/// the cap reached; and a projection past the cap alerts before 50 %.
#[test]
fn the_months_alert_says_the_threshold_passed_or_the_projection() {
    let (_d, l) = ledger(Some(r#"{"currency":"USD","monthly_cap":100,"prices":{"paid":[{"from":"2026-01-01","input":0,"output":1}]}}"#));
    let spend = |usd: u64| {
        let k = Tokens { output: usd * 1_000_000, ..Default::default() };
        let r = l.reserve_on("2026-11-01".into(), "paid", "vertex", false, k, &Caller { human: true, ..Default::default() }, false).unwrap();
        l.settle(&r, Some(k)).unwrap();
    };
    // On the 30th of a month of 30 days, the projection is the total.
    assert_eq!(l.alert_on("2026-11-30"), serde_json::Value::Null);
    spend(10);
    assert_eq!(l.alert_on("2026-11-30"), serde_json::Value::Null, "10 %");
    let early = l.alert_on("2026-11-02");
    assert_eq!((early["threshold"].clone(), early["projection_over"].clone()), (serde_json::Value::Null, json!(true)), "10 in 2 days: 150 projected: {early:#}");
    spend(45);
    assert_eq!(l.alert_on("2026-11-30")["threshold"], json!(50.0));
    spend(30);
    assert_eq!(l.alert_on("2026-11-30")["threshold"], json!(80.0));
    spend(20);
    let at_cap = l.alert_on("2026-11-30");
    assert_eq!(at_cap["threshold"], json!(100.0));
    assert!(at_cap["text"].as_str().unwrap().contains("plafond du mois est atteint"), "{at_cap:#}");
}

/// **The month's alert is said once per level in a thread** (« le plafond approché se dit dans le fil »): the first
/// turn past 50 % says it, the next does not, the next level is said again, and another thread says its own.
#[test]
fn the_months_alert_is_said_once_per_level_in_a_thread() {
    let (_d, l) = ledger(Some(r#"{"currency":"USD","monthly_cap":100,"prices":{"paid":[{"from":"2026-01-01","input":0,"output":1}]}}"#));
    let spend = |usd: u64| {
        let k = Tokens { output: usd * 1_000_000, ..Default::default() };
        let r = l.reserve_on("2026-11-01".into(), "paid", "vertex", false, k, &Caller { human: true, ..Default::default() }, false).unwrap();
        l.settle(&r, Some(k)).unwrap();
    };
    assert_eq!(l.alert_to_say_on("chat:a", "2026-11-30"), None, "nothing under 50 %");
    spend(55);
    let said = l.alert_to_say_on("chat:a", "2026-11-30").expect("said at 50 %");
    assert!(said["text"].as_str().unwrap().contains("50 % du plafond"), "{said:#}");
    assert_eq!(l.alert_to_say_on("chat:a", "2026-11-30"), None, "said once");
    assert!(l.alert_to_say_on("chat:b", "2026-11-30").is_some(), "another thread says its own");
    spend(30);
    assert_eq!(l.alert_to_say_on("chat:a", "2026-11-30").expect("the next level")["threshold"], json!(80.0));
}

/// **A model without a price goes on at a provisional price looked up** (c7, 23 November 2026: the host looks it up
/// itself, the turn goes): the price found on the provider's page is written provisional with its source; the calls
/// counted at it are said provisional; the user's price replaces it.
#[test]
fn a_model_without_a_price_goes_on_at_a_provisional_price_looked_up_and_the_users_replaces_it() {
    let (_d, l) = ledger(Some(PRICES));
    let finder: PriceFinder = Arc::new(|slug: &str| {
        Ok(Tariff { from: "2026-01-01".into(), input: 0.5, cache_write: None, cache_read: Some(0.05), output: 2.0, batch: None, currency: None,
            source: Some(format!("https://provider.example/{slug}")), read_on: None, note: None, provisional: None })
    });
    let l = Arc::new(Ledger::new(l.dir.clone(), l.prices_path.clone()).with_finder(finder));
    let m = MeteredLlm::new(model(Usage { prompt_tokens: 10, completion_tokens: 1_000_000, ..Default::default() }), l.clone(), "new-model", "vertex", false);
    call(&m, 10).expect("the work goes on");
    let t = Prices::load(&l.prices_path).unwrap().tariff("new-model", &utc_day(now_secs())).cloned().unwrap();
    assert!(t.is_provisional() && t.output == 2.0 && t.source.as_deref() == Some("https://provider.example/new-model"), "{t:?}");
    let r = l.report(&month(), &utc_day(now_secs()));
    assert_eq!(r["by_slug"]["new-model"]["provisional"], json!(true), "{r:#}");
    assert!(r["provisional_cost"].as_f64().unwrap() > 1.9, "{r:#}");
    assert_eq!(l.provisional_prices().len(), 1);
    let given = Tariff { provisional: None, output: 3.0, source: Some("given by the user".into()), ..t };
    l.add_tariff("new-model", &given).unwrap();
    assert!(l.provisional_prices().is_empty(), "the user's price replaces the provisional one");
    assert_eq!(Prices::load(&l.prices_path).unwrap().prices["new-model"].len(), 1);
}

/// **A failed lookup takes the dearest price of the model's family, said**, and the work goes on: never zero, never a
/// refusal.
#[test]
fn a_failed_lookup_takes_the_dearest_price_of_its_family() {
    let (_d, l) = ledger(Some(
        r#"{"currency":"USD","monthly_cap":200,"prices":{
            "claude-opus-5":[{"from":"2026-01-01","input":5,"output":25,"cache_read":0.5}],
            "claude-sonnet-5":[{"from":"2026-01-01","input":2,"output":10}],
            "paid":[{"from":"2026-01-01","input":9,"output":90}]}}"#,
    ));
    let l = Ledger::new(l.dir.clone(), l.prices_path.clone()).with_finder(Arc::new(|_: &str| Err("the provider's page did not answer".into())));
    l.reserve("claude-opus-9", "anthropic", false, Tokens::default(), &Caller::default(), false).expect("the work goes on");
    let t = Prices::load(&l.prices_path).unwrap().tariff("claude-opus-9", &utc_day(now_secs())).cloned().unwrap();
    assert_eq!((t.input, t.output, t.is_provisional()), (5.0, 25.0, true), "the dearest of `claude-`, not of every model: {t:?}");
    let note = t.note.unwrap_or_default();
    assert!(note.contains("family `claude-`") && note.contains("did not answer"), "{note}");
}

/// **The models shipped by default have their shipped price**: a first launch never needs a lookup.
#[test]
fn the_models_shipped_by_default_have_their_price() {
    let p = Prices::default_file();
    let today = utc_day(now_secs());
    assert!(p.tariff(crate::regime::MODELE_VERTEX_PAR_DEFAUT, &today).is_some_and(|t| !t.is_provisional()));
    #[cfg(feature = "anthropic-llm")]
    assert!(p.tariff(crate::anthropic_llm::DEFAULT_MODEL, &today).is_some_and(|t| !t.is_provisional()));
}
