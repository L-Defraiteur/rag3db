//! **The spending ledger and the monthly cap** (21 November 2026; the design is
//! `docs/everything-declarative/21-11-2026/09-le-registre-des-depenses-et-le-plafond.md`).
//!
//! Every call of a declared language model goes through [`MeteredLlm`], which `connect_llm` puts around each client.
//! Before the call, under a file lock, it **reserves** the call's upper bound (the input it sends at full price, plus its
//! `max_tokens` at the output price) and refuses it if the month's total plus that bound passes the cap. After the call,
//! it **settles** the reservation with the tokens the provider reported in `usage`, never an estimate.
//!
//! - **Facts, never a price** (the doctrine of `meter.rs`): a line holds tokens; the cost is computed at reading, with the
//!   tariff in force on the call's day (`prices.json`, one entry per slug and per date of effect).
//! - **The side that is safe**: a reservation never settled (a process killed, a call cut) counts at its bound; a reply
//!   without `usage` counts at its bound; a slug without a price is refused, never counted at zero; a prices file that
//!   cannot be read refuses every paid call.
//! - **Several writers** (the host, several clients, the bench's scripts): a line is appended by one `write` in
//!   `O_APPEND`, under `PIPE_BUF`; the total is read and the reservation written under `flock` on `<month>.lock`.
//! - **Beyond the cap**, a call that is not a human's own is refused; a human's passes and is counted. The caller is said
//!   by [`attribute`], a guard on the calling thread; a call nobody attributed is not a human's.
//! - Months and days are UTC.
use crate::llm::{Finish, GenOptions, Llm, LlmError, Quota, TokenSink, Turn, Usage};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The default currency and cap (Lucie, 21 November 2026: 200 USD a month).
pub const DEFAULT_CURRENCY: &str = "USD";
pub const DEFAULT_MONTHLY_CAP: f64 = 200.0;
/// A line stays under `PIPE_BUF` (4 096 bytes on Linux): one `write` in `O_APPEND`, never mixed with another writer's.
const LINE_MAX: usize = 3_800;
/// The prices shipped with the crate.
pub const DEFAULT_PRICES: &str = include_str!("../templates/prices.json");

/// One tariff of a slug, in force from `from` (`YYYY-MM-DD`, UTC), per million tokens.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Tariff {
    pub from: String,
    #[serde(default)]
    pub input: f64,
    /// Writing the provider's cache; the input price when absent.
    #[serde(default)]
    pub cache_write: Option<f64>,
    /// Reading the provider's cache; the input price when absent (never guessed cheaper).
    #[serde(default)]
    pub cache_read: Option<f64>,
    /// The output, reasoning included.
    #[serde(default)]
    pub output: f64,
    /// The multiplier of a batch call.
    #[serde(default)]
    pub batch: Option<f64>,
    /// When given, it must be the file's currency: there is never an implicit conversion.
    #[serde(default)]
    pub currency: Option<String>,
    /// Where the price was read, the day it was read, and what is to know about it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_on: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// `prices.json`: the currency, the monthly cap, the alerts, and the tariffs per slug.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Prices {
    pub currency: String,
    pub monthly_cap: f64,
    #[serde(default = "default_alerts")]
    pub alerts: Vec<f64>,
    /// A model served at a loopback or private address (a llama-server on this machine or the next) costs nothing:
    /// only when the file says so (c7, 21 November 2026), never by default. The file the ledger writes says so.
    #[serde(default)]
    pub free_private_hosts: bool,
    /// What the file says of itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    #[serde(default)]
    pub prices: BTreeMap<String, Vec<Tariff>>,
}

fn default_alerts() -> Vec<f64> {
    vec![0.5, 0.8]
}

fn is_day(d: &str) -> bool {
    let b = d.as_bytes();
    b.len() == 10 && b[4] == b'-' && b[7] == b'-' && d.chars().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

impl Prices {
    /// The file the ledger writes when there is none (`templates/prices.json`, shipped with the crate): the cap of
    /// 200 USD a month, models at a private address free, and the prices read on the providers' official pages, each
    /// with its source, the day it was read, and its date of effect. A model it does not price is refused.
    pub fn default_file() -> Prices {
        serde_json::from_str(DEFAULT_PRICES).expect("templates/prices.json parses: its witness says so")
    }

    /// Reads and checks `prices.json`. Every refusal names the file and what is wrong.
    pub fn load(path: &Path) -> Result<Prices, String> {
        let text = std::fs::read_to_string(path).map_err(|e| {
            format!("{}: {e} — the spending cap needs this file (currency, monthly_cap, a price per slug)", path.display())
        })?;
        let p: Prices = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        p.check().map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(p)
    }

    fn check(&self) -> Result<(), String> {
        if self.currency.len() != 3 || !self.currency.chars().all(|c| c.is_ascii_uppercase()) {
            return Err(format!("currency `{}` is not a currency code (USD, EUR…)", self.currency));
        }
        if !self.monthly_cap.is_finite() || self.monthly_cap < 0.0 {
            return Err(format!("monthly_cap {} is not an amount", self.monthly_cap));
        }
        for (slug, tariffs) in &self.prices {
            for t in tariffs {
                if !is_day(&t.from) {
                    return Err(format!("{slug}: `from` {} is not a day (YYYY-MM-DD)", t.from));
                }
                if let Some(c) = t.currency.as_deref().filter(|c| *c != self.currency) {
                    return Err(format!(
                        "{slug}: a tariff in {c} beside a cap in {}: the cap is compared in the prices' currency, never converted",
                        self.currency
                    ));
                }
                let all = [Some(t.input), t.cache_write, t.cache_read, Some(t.output), t.batch];
                if all.iter().flatten().any(|v| !v.is_finite() || *v < 0.0) {
                    return Err(format!("{slug}: a price is negative or not a number"));
                }
            }
        }
        Ok(())
    }

    /// The tariff of `slug` in force on `day`: the latest `from` on or before it.
    pub fn tariff(&self, slug: &str, day: &str) -> Option<&Tariff> {
        self.prices.get(slug)?.iter().filter(|t| t.from.as_str() <= day).max_by(|a, b| a.from.cmp(&b.from))
    }
}

/// The tokens of a call, as the provider bills them: the input at full price, the cache written, the cache read, the
/// output (reasoning included).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tokens {
    pub input: u64,
    pub cache_write: u64,
    pub cache_read: u64,
    pub output: u64,
}

impl Tokens {
    /// From the provider's `usage`; `None` when it reported nothing (never estimated).
    pub fn from_usage(u: &Usage) -> Option<Tokens> {
        if u.prompt_tokens == 0 && u.completion_tokens == 0 {
            return None;
        }
        let (read, written) = (u.cached_prompt_tokens as u64, u.cache_write_tokens as u64);
        Some(Tokens {
            input: (u.prompt_tokens as u64).saturating_sub(read + written),
            cache_write: written,
            cache_read: read,
            output: u.completion_tokens as u64,
        })
    }

    /// The cost under `t`, in the prices' currency.
    pub fn cost(&self, t: &Tariff, batch: bool) -> f64 {
        let c = self.input as f64 * t.input
            + self.cache_write as f64 * t.cache_write.unwrap_or(t.input)
            + self.cache_read as f64 * t.cache_read.unwrap_or(t.input)
            + self.output as f64 * t.output;
        c / 1e6 * if batch { t.batch.unwrap_or(1.0) } else { 1.0 }
    }
}

/// Who called: the agent's key, the session, the run, and whether a human launched this call themselves.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Caller {
    #[serde(default)]
    pub agent: String,
    #[serde(default)]
    pub session: String,
    #[serde(default)]
    pub run: String,
    #[serde(default)]
    pub human: bool,
}

thread_local! {
    static CALLER: RefCell<Option<Caller>> = const { RefCell::new(None) };
}

/// The caller of the calls made on this thread until the guard drops.
pub struct Attribution(Option<Caller>);

/// Says who calls, for the calls of this thread until the returned guard drops.
pub fn attribute(caller: Caller) -> Attribution {
    Attribution(CALLER.with(|c| c.borrow_mut().replace(caller)))
}

impl Drop for Attribution {
    fn drop(&mut self) {
        let previous = self.0.take();
        CALLER.with(|c| *c.borrow_mut() = previous);
    }
}

fn current_caller() -> Caller {
    CALLER.with(|c| c.borrow().clone().unwrap_or_default())
}

/// One line of the ledger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Line {
    pub t: String,
    pub day: String,
    pub id: String,
    /// `reserve` (the bound, before the call) or `settle` (what the provider reported, after it).
    pub kind: String,
    pub slug: String,
    pub provider: String,
    /// `None` on a settle: the provider reported nothing, and the reservation's bound stands.
    pub tokens: Option<Tokens>,
    #[serde(default)]
    pub free: bool,
    #[serde(default)]
    pub batch: bool,
    #[serde(default)]
    pub caller: Caller,
}

/// What a month has spent.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Spent {
    pub total: f64,
    /// Reservations without their settle: counted at their bound.
    pub open: usize,
    /// Slugs written without a price (by `usage record`): listed, never counted at zero silently.
    pub unpriced: Vec<String>,
    /// Lines that do not parse: listed by their number.
    pub unreadable: Vec<usize>,
}

/// One call of the ledger, read back.
#[derive(Debug, Clone)]
pub struct Call {
    pub line: Line,
    pub tokens: Option<Tokens>,
    /// `None`: no price for its slug on its day.
    pub cost: Option<f64>,
    pub open: bool,
}

/// The days of a month (`YYYY-MM`).
fn days_in_month(month: &str) -> u32 {
    let y: i64 = month[..4].parse().unwrap_or(1970);
    let m: u32 = month[5..7].parse().unwrap_or(1);
    match m {
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// **The month's projection**: linear on the days elapsed, today included. A month already over is its total.
pub fn projection(spent: f64, month: &str, today: &str) -> f64 {
    if &today[..7] != month {
        return spent;
    }
    let day: u32 = today[8..10].parse().unwrap_or(1).max(1);
    spent / day as f64 * days_in_month(month) as f64
}

/// A reservation made, to settle.
#[derive(Debug, Clone)]
pub struct Reservation {
    pub line: Line,
}

/// The ledger: `<dir>/<YYYY-MM>.jsonl`, its lock `<dir>/<YYYY-MM>.lock`, and the prices it reads.
#[derive(Debug, Clone)]
pub struct Ledger {
    pub dir: PathBuf,
    pub prices_path: PathBuf,
}

/// The UTC day of a Unix second.
pub fn utc_day(secs: i64) -> String {
    let (y, m, d) = crate::host_log::civil_from_days(secs.div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("u-{}-{nanos:x}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed))
}

/// The lock of a month, held until dropped (`flock` is released with the file).
struct MonthLock(#[allow(dead_code)] std::fs::File);

impl Ledger {
    pub fn new(dir: impl Into<PathBuf>, prices_path: impl Into<PathBuf>) -> Self {
        Ledger { dir: dir.into(), prices_path: prices_path.into() }
    }

    /// The user's ledger: `usage/` under the host's state folder (`$XDG_STATE_HOME/rag3weaver` or
    /// `~/.local/state/rag3weaver`, `RAG3WEAVER_DAEMON_DIR` for a witness's host), and `prices.json` in the
    /// configuration folder (`RAG3WEAVER_CONFIG_DIR` or `~/.config/rag3weaver`). A unit test never writes there: it
    /// gets a folder of its own process under the temporary directory.
    pub fn of_user() -> Self {
        if cfg!(test) {
            let d = std::env::temp_dir().join(format!("rag3weaver-usage-witnesses-{}", std::process::id()));
            return Ledger::new(d.join("usage"), d.join("prices.json"));
        }
        // The host's state folder, where `daemon_lifecycle` puts its journal; the same folder in a build without the
        // host (`RAG3WEAVER_DAEMON_DIR`, else `~/.local/state/rag3weaver`).
        #[cfg(feature = "daemon")]
        let state = crate::daemon_lifecycle::log_path().parent().map(Path::to_path_buf).unwrap_or_default();
        #[cfg(not(feature = "daemon"))]
        let state = std::env::var_os("RAG3WEAVER_DAEMON_DIR").map(PathBuf::from).unwrap_or_else(|| {
            std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join(".local/state/rag3weaver")
        });
        let config = std::env::var_os("RAG3WEAVER_CONFIG_DIR").map(PathBuf::from).unwrap_or_else(|| {
            std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join(".config/rag3weaver")
        });
        Ledger::new(state.join("usage"), config.join("prices.json"))
    }

    pub fn month_path(&self, month: &str) -> PathBuf {
        self.dir.join(format!("{month}.jsonl"))
    }

    fn lock(&self, month: &str) -> Result<MonthLock, String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| format!("{}: {e}", self.dir.display()))?;
        let path = self.dir.join(format!("{month}.lock"));
        let f = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            // SAFETY: a valid descriptor, held by `f` for the life of the lock.
            if unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX) } != 0 {
                return Err(format!("{}: flock: {}", path.display(), std::io::Error::last_os_error()));
            }
        }
        Ok(MonthLock(f))
    }

    /// Every line of a month, in order; a line that does not parse is kept by its number.
    pub fn lines(&self, month: &str) -> (Vec<Line>, Vec<usize>) {
        let text = std::fs::read_to_string(self.month_path(month)).unwrap_or_default();
        let (mut lines, mut bad) = (Vec::new(), Vec::new());
        for (i, l) in text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty()) {
            match serde_json::from_str::<Line>(l) {
                Ok(x) => lines.push(x),
                Err(_) => bad.push(i + 1),
            }
        }
        (lines, bad)
    }

    /// Every call of a month, under `prices`: its head line (the settle, or the reservation), its tokens (the settle's,
    /// or the reservation's bound), its cost (`None`: no price), and whether it is still open.
    pub fn calls(&self, prices: &Prices, month: &str) -> (Vec<Call>, Vec<usize>) {
        let (lines, unreadable) = self.lines(month);
        let mut by_id: BTreeMap<String, (Option<Line>, Option<Line>)> = BTreeMap::new();
        for l in lines {
            let e = by_id.entry(l.id.clone()).or_default();
            if l.kind == "settle" {
                e.1 = Some(l);
            } else {
                e.0 = Some(l);
            }
        }
        let calls = by_id
            .into_values()
            .filter_map(|(reserve, settle)| {
                let open = settle.is_none();
                let tokens = settle.as_ref().and_then(|l| l.tokens).or_else(|| reserve.as_ref().and_then(|l| l.tokens));
                let line = settle.or(reserve)?;
                let cost = if line.free {
                    Some(0.0)
                } else {
                    prices.tariff(&line.slug, &line.day).map(|t| tokens.map(|k| k.cost(t, line.batch)).unwrap_or(0.0))
                };
                Some(Call { line, tokens, cost, open })
            })
            .collect();
        (calls, unreadable)
    }

    /// What a month has spent, under `prices`: each call at its settle, or at its reservation's bound.
    pub fn spent(&self, prices: &Prices, month: &str) -> Spent {
        let (calls, unreadable) = self.calls(prices, month);
        let mut s = Spent { unreadable, ..Default::default() };
        for c in calls.iter().filter(|c| !c.line.free) {
            if c.open {
                s.open += 1;
            }
            match c.cost {
                Some(x) => s.total += x,
                None => {
                    if !s.unpriced.contains(&c.line.slug) {
                        s.unpriced.push(c.line.slug.clone());
                    }
                }
            }
        }
        s
    }

    /// **The month's report** (`usage_report`): the total against the cap, the projection, and the calls by slug, by
    /// day and by caller. `today` is the UTC day the projection counts to.
    pub fn report(&self, month: &str, today: &str) -> serde_json::Value {
        use serde_json::json;
        let prices = match Prices::load(&self.prices_path) {
            Ok(p) => p,
            Err(e) => return json!({ "month": month, "error": e }),
        };
        let (calls, unreadable) = self.calls(&prices, month);
        let spent = self.spent(&prices, month);
        let mut by_slug: BTreeMap<String, serde_json::Value> = BTreeMap::new();
        let mut by_day: BTreeMap<String, f64> = BTreeMap::new();
        let mut by_caller: BTreeMap<String, f64> = BTreeMap::new();
        for c in &calls {
            let cost = c.cost.unwrap_or(0.0);
            let k = c.tokens.unwrap_or_default();
            let e = by_slug.entry(c.line.slug.clone()).or_insert_with(|| {
                json!({ "provider": c.line.provider, "calls": 0, "input": 0, "cache_write": 0, "cache_read": 0, "output": 0, "cost": 0.0, "priced": c.cost.is_some(), "free": c.line.free })
            });
            for (key, n) in [("calls", 1), ("input", k.input), ("cache_write", k.cache_write), ("cache_read", k.cache_read), ("output", k.output)] {
                e[key] = json!(e[key].as_u64().unwrap_or(0) + n);
            }
            e["cost"] = json!(e["cost"].as_f64().unwrap_or(0.0) + cost);
            *by_day.entry(c.line.day.clone()).or_default() += cost;
            let who = if c.line.caller.human {
                "human".to_string()
            } else if c.line.caller.agent.is_empty() {
                "unattributed".to_string()
            } else {
                c.line.caller.agent.clone()
            };
            *by_caller.entry(who).or_default() += cost;
        }
        let projection = projection(spent.total, month, today);
        json!({
            "month": month,
            "currency": prices.currency,
            "cap": prices.monthly_cap,
            "spent": spent.total,
            "percent": if prices.monthly_cap > 0.0 { spent.total / prices.monthly_cap * 100.0 } else { 0.0 },
            "projection": projection,
            "open_reservations": spent.open,
            "unpriced": spent.unpriced,
            "unreadable_lines": unreadable,
            "by_slug": by_slug,
            "by_day": by_day,
            "by_caller": by_caller,
            "prices": self.prices_path,
            "ledger": self.month_path(month),
        })
    }

    /// **The alert of the month**, for « À intervenir »: `null` under the first threshold of `alerts` (50 % and 80 % by
    /// default) with a projection under the cap; otherwise the highest threshold passed (100 at the cap), whether the
    /// projection passes the cap, and the sentence to show. Read from `report`, at each look: a state, not a message.
    pub fn alert_on(&self, today: &str) -> serde_json::Value {
        use serde_json::json;
        let r = self.report(&today[..7], today);
        if let Some(e) = r["error"].as_str() {
            return json!({ "threshold": null, "projection_over": false, "text": format!("le plafond des dépenses ne se lit pas : {e}") });
        }
        let (spent, cap, projection) = (r["spent"].as_f64().unwrap_or(0.0), r["cap"].as_f64().unwrap_or(0.0), r["projection"].as_f64().unwrap_or(0.0));
        let alerts: Vec<f64> = Prices::load(&self.prices_path).map(|p| p.alerts).unwrap_or_else(|_| default_alerts());
        let passed = if cap > 0.0 && spent >= cap {
            Some(100.0)
        } else {
            alerts.iter().copied().filter(|a| cap > 0.0 && spent >= a * cap).map(|a| a * 100.0).fold(None, |m: Option<f64>, a| Some(m.map_or(a, |m| m.max(a))))
        };
        let projection_over = cap > 0.0 && projection > cap;
        if passed.is_none() && !projection_over {
            return serde_json::Value::Null;
        }
        let cur = r["currency"].as_str().unwrap_or("");
        let text = match passed {
            Some(t) if t >= 100.0 => format!(
                "le plafond du mois est atteint : {spent:.2} / {cap:.0} {cur} ; l'appel payant d'un agent est refusé jusqu'au mois prochain ou à un plafond relevé"
            ),
            Some(t) => format!("{t:.0} % du plafond du mois : {spent:.2} / {cap:.0} {cur}, projection {projection:.0} {cur}"),
            None => format!("le mois va dépasser son plafond : projection {projection:.0} / {cap:.0} {cur} ({spent:.2} dépensés)"),
        };
        json!({ "threshold": passed, "projection_over": projection_over, "spent": spent, "cap": cap, "projection": projection, "currency": cur, "text": text })
    }

    /// **The alert to say in a thread** (« le plafond approché se dit dans le fil, avant de bloquer »): the month's
    /// alert when its level (the threshold passed, or the projection past the cap) has not been said yet in the thread
    /// `key` this month; then it is marked said. Once per level, per thread, per month: a higher level is said again.
    pub fn alert_to_say_on(&self, key: &str, today: &str) -> Option<serde_json::Value> {
        let alert = self.alert_on(today);
        if alert.is_null() {
            return None;
        }
        let level = alert["threshold"].as_f64().map(|t| format!("{t:.0}")).unwrap_or_else(|| "projection".to_string());
        let month = &today[..7];
        let _lock = self.lock(month).ok()?;
        let path = self.dir.join(format!("{month}.said.json"));
        let mut said: serde_json::Value = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_else(|| serde_json::json!({}));
        let levels = said.as_object_mut()?.entry(key.to_string()).or_insert_with(|| serde_json::json!([]));
        if levels.as_array().is_some_and(|l| l.iter().any(|x| x == &serde_json::json!(level))) {
            return None;
        }
        levels.as_array_mut()?.push(serde_json::json!(level));
        let tmp = path.with_extension("json.new");
        std::fs::write(&tmp, said.to_string()).ok()?;
        std::fs::rename(&tmp, &path).ok()?;
        Some(alert)
    }

    /// [`alert_to_say_on`](Self::alert_to_say_on) today (UTC).
    pub fn alert_to_say(&self, key: &str) -> Option<serde_json::Value> {
        self.alert_to_say_on(key, &utc_day(now_secs()))
    }

    /// [`alert_on`](Self::alert_on) today (UTC).
    pub fn alert(&self) -> serde_json::Value {
        self.alert_on(&utc_day(now_secs()))
    }

    /// **One line for `daemon_status`**: `spending: 41.20 / 200 USD (21 %), projection 59 USD`.
    pub fn summary_line(&self) -> String {
        let today = utc_day(now_secs());
        let r = self.report(&today[..7], &today);
        if let Some(e) = r["error"].as_str() {
            return format!("spending: unknown ({e})");
        }
        let cur = r["currency"].as_str().unwrap_or("");
        let mut s = format!(
            "spending: {:.2} / {:.0} {cur} ({:.0} %), projection {:.0} {cur}",
            r["spent"].as_f64().unwrap_or(0.0),
            r["cap"].as_f64().unwrap_or(0.0),
            r["percent"].as_f64().unwrap_or(0.0),
            r["projection"].as_f64().unwrap_or(0.0)
        );
        if let Some(u) = r["unpriced"].as_array().filter(|u| !u.is_empty()) {
            s.push_str(&format!("; without a price: {}", u.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(", ")));
        }
        s
    }

    fn append(&self, month: &str, line: &Line) -> Result<(), String> {
        let mut line = line.clone();
        let mut text = serde_json::to_string(&line).map_err(|e| e.to_string())?;
        if text.len() > LINE_MAX {
            let cut = |s: &mut String| s.truncate(s.char_indices().nth(64).map(|(i, _)| i).unwrap_or(s.len()));
            cut(&mut line.caller.agent);
            cut(&mut line.caller.session);
            cut(&mut line.caller.run);
            text = serde_json::to_string(&line).map_err(|e| e.to_string())?;
        }
        text.push('\n');
        if text.len() > LINE_MAX + 1 {
            return Err(format!("a ledger line of {} bytes, over {LINE_MAX}", text.len()));
        }
        let path = self.month_path(month);
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let n = f.write(text.as_bytes()).map_err(|e| format!("{}: {e}", path.display()))?;
        if n != text.len() {
            return Err(format!("{}: a line written in part ({n} of {} bytes)", path.display(), text.len()));
        }
        Ok(())
    }

    /// Reserves a call's bound, or refuses it. `free`: a model that costs nothing (written, never capped). `private`:
    /// served at a loopback or private address, free when the prices say so, or when there are no prices at all.
    pub fn reserve(
        &self,
        slug: &str,
        provider: &str,
        private: bool,
        bound: Tokens,
        caller: &Caller,
        batch: bool,
    ) -> Result<Reservation, String> {
        self.reserve_on(utc_day(now_secs()), slug, provider, private, bound, caller, batch)
    }

    /// [`reserve`](Self::reserve) on a given day (the witnesses of a tariff that changes).
    #[allow(clippy::too_many_arguments)]
    pub fn reserve_on(
        &self,
        day: String,
        slug: &str,
        provider: &str,
        private: bool,
        bound: Tokens,
        caller: &Caller,
        batch: bool,
    ) -> Result<Reservation, String> {
        let month = day[..7].to_string();
        let _lock = self.lock(&month)?;
        let free = self.check_locked(&day, slug, private, bound, caller, batch)?;
        let line = Line {
            t: crate::host_log::now_iso(),
            day,
            id: new_id(),
            kind: "reserve".into(),
            slug: slug.into(),
            provider: provider.into(),
            tokens: Some(bound),
            free,
            batch,
            caller: caller.clone(),
        };
        self.append(&month, &line)?;
        Ok(Reservation { line })
    }

    /// **Whether a call of this bound would pass**, without reserving (`usage check`, for a script that calls a paid
    /// model itself): `Ok(free)`, or the refusal `reserve` would give.
    pub fn check(&self, slug: &str, private: bool, bound: Tokens, caller: &Caller, batch: bool) -> Result<bool, String> {
        let day = utc_day(now_secs());
        let _lock = self.lock(&day[..7])?;
        self.check_locked(&day, slug, private, bound, caller, batch)
    }

    /// **A call already made**, written whole (`usage record`, the bench's scripts): its reservation and its settle, at
    /// the tokens the provider reported. Never refused: the money is spent; a slug without a price is written, listed
    /// by the report as unpriced, and said in the returned warning.
    #[allow(clippy::too_many_arguments)]
    pub fn record(&self, slug: &str, provider: &str, private: bool, tokens: Tokens, caller: &Caller, batch: bool) -> Result<Option<String>, String> {
        self.write_default_prices();
        let day = utc_day(now_secs());
        let month = day[..7].to_string();
        let prices = Prices::load(&self.prices_path);
        let free = prices.as_ref().is_ok_and(|p| private && p.free_private_hosts);
        let warning = match &prices {
            _ if free => None,
            Ok(p) if p.tariff(slug, &day).is_some() => None,
            Ok(_) => Some(format!("no price for `{slug}` on {day} in {}: written, and listed as unpriced", self.prices_path.display())),
            Err(e) => Some(format!("written, but the prices cannot be read: {e}")),
        };
        let _lock = self.lock(&month)?;
        let mut line = Line {
            t: crate::host_log::now_iso(),
            day,
            id: new_id(),
            kind: "reserve".into(),
            slug: slug.into(),
            provider: provider.into(),
            tokens: Some(tokens),
            free,
            batch,
            caller: caller.clone(),
        };
        self.append(&month, &line)?;
        line.kind = "settle".into();
        self.append(&month, &line)?;
        Ok(warning)
    }

    /// The check of a reservation, the month's lock held: `Ok(free)` or the refusal.
    #[allow(clippy::too_many_arguments)]
    fn check_locked(&self, day: &str, slug: &str, private: bool, bound: Tokens, caller: &Caller, batch: bool) -> Result<bool, String> {
        let month = &day[..7];
        self.write_default_prices();
        let prices = Prices::load(&self.prices_path);
        // Free only by what the file says: a file that cannot be read frees nothing.
        let free = prices.as_ref().is_ok_and(|p| private && p.free_private_hosts);
        if !free {
            let prices = prices.map_err(|e| format!("spending cap: {e}; a paid call is refused while the prices cannot be read"))?;
            let tariff = prices.tariff(slug, day).ok_or_else(|| {
                format!(
                    "spending cap: no price for `{slug}` on {day} in {}: a paid call without a price is refused, never counted at zero",
                    self.prices_path.display()
                )
            })?;
            let spent = self.spent(&prices, month);
            let at_most = bound.cost(tariff, batch);
            if spent.total + at_most > prices.monthly_cap && !caller.human {
                return Err(format!(
                    "spending cap reached for {month}: {:.2} spent and {:.2} at most for this call, over the cap of {:.2} {} ({}); a human's own call still passes",
                    spent.total,
                    at_most,
                    prices.monthly_cap,
                    prices.currency,
                    self.prices_path.display()
                ));
            }
        }
        Ok(free)
    }

    /// **Adds a tariff to `prices.json`** (22 November 2026, « un refus offre le geste »): the entry is appended to its
    /// slug's list, the rest of the file kept as it is (its `about`, its notes, the other slugs); the new file must pass
    /// the checks of [`Prices::load`] before it replaces the old one, written beside it then renamed. The caller says
    /// who gave it: the host writes a price only on a human's yes (`DaemonHost::set_price`).
    pub fn add_tariff(&self, slug: &str, tariff: &Tariff) -> Result<(), String> {
        if slug.trim().is_empty() {
            return Err("a price needs its model's slug".into());
        }
        self.write_default_prices();
        let _lock = self.lock("prices")?;
        let text = std::fs::read_to_string(&self.prices_path).map_err(|e| format!("{}: {e}", self.prices_path.display()))?;
        let mut v: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", self.prices_path.display()))?;
        let entry = serde_json::to_value(tariff).map_err(|e| e.to_string())?;
        let prices = v.as_object_mut().ok_or("prices.json is not an object")?.entry("prices").or_insert_with(|| serde_json::json!({}));
        let list = prices.as_object_mut().ok_or("`prices` is not an object")?.entry(slug.to_string()).or_insert_with(|| serde_json::json!([]));
        list.as_array_mut().ok_or_else(|| format!("the prices of `{slug}` are not a list"))?.push(entry);
        let new = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?;
        let parsed: Prices = serde_json::from_str(&new).map_err(|e| format!("the price of `{slug}`: {e}"))?;
        parsed.check().map_err(|e| format!("the price of `{slug}`: {e}"))?;
        let tmp = self.prices_path.with_extension("json.new");
        std::fs::write(&tmp, format!("{new}\n")).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &self.prices_path).map_err(|e| format!("{}: {e}", self.prices_path.display()))?;
        Ok(())
    }

    /// Writes the default `prices.json` when there is none (never over a file, even an unreadable one), and says so.
    /// **Whole or not at all**: written beside it, then put in place by a hard link, which refuses an existing file; a
    /// reader at the same instant sees no file or the whole one, never an empty one (two writers at the first call
    /// made a reader find the file created and not yet written, 23 November 2026).
    fn write_default_prices(&self) {
        if self.prices_path.exists() {
            return;
        }
        if let Some(parent) = self.prices_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let tmp = self.prices_path.with_extension(format!("json.{}.{}", std::process::id(), new_id()));
        if std::fs::write(&tmp, format!("{}\n", DEFAULT_PRICES.trim_end())).is_err() {
            return;
        }
        let placed = std::fs::hard_link(&tmp, &self.prices_path).is_ok();
        let _ = std::fs::remove_file(&tmp);
        if placed {
            eprintln!(
                "[usage] {} written: a cap of {DEFAULT_MONTHLY_CAP} {DEFAULT_CURRENCY} a month, private addresses free, the prices read on the providers' pages: a model it does not price is refused until its price is written there",
                self.prices_path.display()
            );
        }
    }

    /// Settles a reservation with what the provider reported (`None`: nothing, and the bound stands).
    pub fn settle(&self, r: &Reservation, tokens: Option<Tokens>) -> Result<(), String> {
        let month = r.line.day[..7].to_string();
        let mut line = r.line.clone();
        line.kind = "settle".into();
        line.t = crate::host_log::now_iso();
        line.tokens = tokens;
        self.append(&month, &line)
    }
}

/// **The slug a refusal names when its model has no price** (the reservation's « no price for `slug` »), so that a
/// caller can offer the gesture instead of the refusal.
pub fn unpriced(message: &str) -> Option<String> {
    let rest = message.split("no price for `").nth(1)?;
    rest.split('`').next().map(str::to_string).filter(|s| !s.is_empty())
}

/// **The user's ledger**, one per process ([`Ledger::of_user`]).
pub fn user_ledger() -> Arc<Ledger> {
    static LEDGER: std::sync::OnceLock<Arc<Ledger>> = std::sync::OnceLock::new();
    LEDGER.get_or_init(|| Arc::new(Ledger::of_user())).clone()
}

/// Whether an address is served on this machine or the local network (loopback, 10/8, 172.16/12, 192.168/16, a
/// `.local` or bare host name).
pub fn is_private_address(address: &str) -> bool {
    let rest = address.split("://").nth(1).unwrap_or(address);
    let host = rest.split(['/', '?']).next().unwrap_or("");
    let host = host.rsplit_once('@').map(|(_, h)| h).unwrap_or(host);
    let host = if host.starts_with('[') {
        host.trim_start_matches('[').split(']').next().unwrap_or("")
    } else {
        host.rsplit_once(':').map(|(h, _)| h).unwrap_or(host)
    };
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return match ip {
            std::net::IpAddr::V4(v) => v.is_loopback() || v.is_private() || v.is_link_local(),
            std::net::IpAddr::V6(v) => v.is_loopback(),
        };
    }
    host == "localhost" || host.ends_with(".local") || (!host.is_empty() && !host.contains('.'))
}

/// A language model whose every call is reserved, capped and settled in the ledger.
pub struct MeteredLlm {
    inner: Arc<dyn Llm>,
    ledger: Arc<Ledger>,
    slug: String,
    provider: String,
    private: bool,
}

impl MeteredLlm {
    pub fn new(inner: Arc<dyn Llm>, ledger: Arc<Ledger>, slug: &str, provider: &str, private: bool) -> Self {
        MeteredLlm { inner, ledger, slug: slug.into(), provider: provider.into(), private }
    }
}

/// The input's upper bound, in tokens: the length of its debug form, which is longer than its text; a tokenizer never
/// gives more tokens than bytes. A bound, never an estimate.
fn input_bound(turns: &[Turn], opts: &GenOptions) -> u64 {
    (format!("{turns:?}").len() + format!("{:?}", opts.tools).len() + format!("{:?}", opts.response_format).len()) as u64
}

impl Llm for MeteredLlm {
    fn generate(&self, turns: &[Turn], opts: &GenOptions, sink: &mut dyn TokenSink) -> Result<(Finish, Usage), LlmError> {
        let bound = Tokens { input: input_bound(turns, opts), output: opts.max_tokens as u64, ..Default::default() };
        let r = self
            .ledger
            .reserve(&self.slug, &self.provider, self.private, bound, &current_caller(), false)
            .map_err(|message| LlmError::Unavailable { status: None, quota: Quota::Exhausted, retries: 0, waited_ms: 0, message })?;
        let out = self.inner.generate(turns, opts, sink);
        let tokens = match &out {
            Ok((_, u)) => Tokens::from_usage(u),
            // Refused before any token (a quota, a service down): nothing billed.
            Err(LlmError::Unavailable { .. }) => Some(Tokens::default()),
            // Cut in the middle: what was billed is unknown, the bound stands.
            Err(_) => None,
        };
        if let Err(e) = self.ledger.settle(&r, tokens) {
            eprintln!("[usage] {}: the settle of {} could not be written ({e}): it counts at its bound", self.slug, r.line.id);
        }
        out
    }

    fn context_len(&self) -> usize {
        self.inner.context_len()
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}

#[cfg(test)]
mod tests;
