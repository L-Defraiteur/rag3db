//! **A price written on a human's yes** (22 November 2026, Lucie: « un refus offre le geste »): a model without a price
//! was refused flat; the refusal now says which model, and the user gives its price, or accepts the one an agent found
//! on its provider's page. The host writes `prices.json`, which no agent can touch (it is under the protected floor),
//! and only when the call carries the user's title: `by.kind == "human"` from `by_of`, never the body.

use serde_json::{json, Value};

use super::DaemonHost;

impl DaemonHost {
    /// **Writes a price** `{slug, input, output, cache_read?, cache_write?, batch?, from?, source?, read_on?}` (per
    /// million tokens, in the file's currency) when `by` is a human's; refused otherwise, and nothing is written. `from`
    /// is today (UTC) when absent: the price holds from now on, the past keeps its own. `source` says where the price was
    /// read, or « given by the user ».
    /// **The monthly cap, on the user's yes** (`POST /ide/prices/cap {monthly_cap}`): written by the host only when the
    /// call carries the user's title.
    pub fn set_cap(&self, by: &Value, body: &Value) -> Result<Value, String> {
        if by["kind"] != "human" {
            return Err("the spending cap is set on the user's yes only: this call does not carry the user's title".into());
        }
        let cap = body["monthly_cap"].as_f64().ok_or("`monthly_cap` is an amount, a number")?;
        let ledger = crate::usage_ledger::user_ledger();
        ledger.set_cap(cap)?;
        Ok(json!({ "monthly_cap": cap, "prices": ledger.prices_path }))
    }

    /// **An unreadable `prices.json` set aside, on the user's yes** (`POST /ide/prices/set-aside`): renamed beside
    /// itself, never deleted; the shipped prices are then in force.
    pub fn set_aside_prices(&self, by: &Value) -> Result<Value, String> {
        if by["kind"] != "human" {
            return Err("the prices file is set aside on the user's yes only: this call does not carry the user's title".into());
        }
        let aside = crate::usage_ledger::user_ledger().set_aside_prices()?;
        Ok(json!({ "set_aside": aside }))
    }

    pub fn set_price(&self, by: &Value, body: &Value) -> Result<Value, String> {
        if by["kind"] != "human" {
            return Err("a price is written on the user's yes only: this call does not carry the user's title".into());
        }
        let slug = body["slug"].as_str().map(str::trim).filter(|s| !s.is_empty()).ok_or("a price needs `slug`, its model")?;
        let number = |k: &str| -> Result<Option<f64>, String> {
            match &body[k] {
                Value::Null => Ok(None),
                v => v.as_f64().map(Some).ok_or_else(|| format!("`{k}` is a price per million tokens, a number")),
            }
        };
        let (input, output) = (number("input")?.ok_or("a price needs `input`")?, number("output")?.ok_or("a price needs `output`")?);
        let today = crate::usage_ledger::utc_day(
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0),
        );
        let tariff = crate::usage_ledger::Tariff {
            from: body["from"].as_str().unwrap_or(&today).to_string(),
            input,
            cache_write: number("cache_write")?,
            cache_read: number("cache_read")?,
            output,
            batch: number("batch")?,
            currency: None,
            source: Some(body["source"].as_str().filter(|s| !s.trim().is_empty()).unwrap_or("given by the user").to_string()),
            read_on: Some(body["read_on"].as_str().unwrap_or(&today).to_string()),
            note: body["note"].as_str().map(str::to_string),
        };
        let ledger = crate::usage_ledger::user_ledger();
        ledger.add_tariff(slug, &tariff)?;
        Ok(json!({ "slug": slug, "tariff": tariff, "prices": ledger.prices_path }))
    }
}

/// **Looks up a model's price** on its provider's official page, by the web search (Gemini grounded by Google Search,
/// on the host's own model, which has a price: the model without one cannot look for itself). Gives a proposal
/// `{input, output, cache_read, cache_write}` per million tokens with its sources, never writes it: the user accepts
/// it (`set_price`). `policy` is the host's web search (its model, its endpoint).
pub fn lookup_price(policy: &crate::backend::WebSearchPolicy, slug: &str) -> Result<Value, String> {
    let slug = slug.trim();
    if slug.is_empty() {
        return Err("a price lookup needs the model's slug".into());
    }
    let query = format!(
        "What is the official API price of the model « {slug} », in US dollars per million tokens, on its provider's own pricing page? \
         Answer with one JSON object and nothing else: {{\"input\": <number>, \"output\": <number>, \"cache_read\": <number or null>, \
         \"cache_write\": <number or null>, \"source\": \"<the pricing page's address>\"}}. When the page gives several tiers, give the highest."
    );
    let found = super::web_search::web_search(policy, &json!({ "query": query }))?;
    let answer = found["answer"].as_str().unwrap_or_default();
    let proposal = first_json_object(answer).ok_or_else(|| format!("the price of `{slug}` was not found in a form to propose: {answer}"))?;
    let number = |k: &str| proposal[k].as_f64().filter(|v| v.is_finite() && *v >= 0.0);
    let (Some(input), Some(output)) = (number("input"), number("output")) else {
        return Err(format!("the price of `{slug}` was found without its input and output prices: {proposal}"));
    };
    let source = proposal["source"]
        .as_str()
        .filter(|s| s.starts_with("http"))
        .map(str::to_string)
        .or_else(|| found["sources"][0]["url"].as_str().map(str::to_string))
        .unwrap_or_default();
    Ok(json!({
        "slug": slug,
        "proposal": { "input": input, "output": output, "cache_read": number("cache_read"), "cache_write": number("cache_write"), "source": source },
        "sources": found["sources"],
        "answer": answer,
    }))
}

/// The first `{…}` of a text that parses as JSON (a model's answer may wrap it in prose or a code fence).
fn first_json_object(text: &str) -> Option<Value> {
    let start = text.find('{')?;
    let mut depth = 0usize;
    for (i, c) in text[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return serde_json::from_str(&text[start..=start + i]).ok();
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
#[path = "prices_tests.rs"]
mod tests;
