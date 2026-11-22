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

#[cfg(test)]
#[path = "prices_tests.rs"]
mod tests;
