//! **The teammate's browser family** (tty, 21 November 2026; the framing is
//! `docs/tty/21-11-2026/04-cadrage-l-outil-navigateur-de-l-equipier.md`, its form agreed with archi).
//!
//! The teammate reads the browser through the bridge to Lucie's extension (`crate::pont_navigateur`): the tabs it may
//! take, a page (its accessibility tree, its text, its address) and its console. Three guards hold before anything is
//! sent to the extension:
//! - a page outside the declared domains (`teammate.browser.domains`; empty, nothing is allowed) is neither taken nor
//!   read;
//! - a secret field (a password, a one-time code, a token, a card) is never read: its value is masked;
//! - the page is read by code the host writes, never by code of the model: `exec` in the extension's engine world
//!   only calls the extension's own page functions.
//!
//! The family is absent unless the manifest declares `teammate.browser`.

use crate::backend::BrowserPolicy;
use crate::pont_navigateur::ecoute::Ecoute;
use crate::tools::ToolDef;
use serde_json::{json, Value};
use std::sync::Arc;

pub const BROWSER_TABS: &str = "browser_tabs";
pub const BROWSER_READ: &str = "browser_read";
pub const BROWSER_CONSOLE: &str = "browser_console";

/// **The family's tool definitions**, given only when the manifest declares `teammate.browser`.
pub fn family(policy: Option<&BrowserPolicy>) -> Vec<ToolDef> {
    if policy.is_none() {
        return Vec::new();
    }
    vec![
        ToolDef {
            name: BROWSER_TABS.into(),
            description: "List the browser tabs you may take, or take one (claim). A tab whose page is outside the declared domains (teammate.browser.domains) is listed as such and cannot be taken.".into(),
            parameters: json!({ "type": "object", "properties": {
                "claim": { "type": "string", "description": "the id of the tab to take (as browser_tabs lists it); absent, the tabs are only listed" }
            } }),
        },
        ToolDef {
            name: BROWSER_READ.into(),
            description: "Read a taken tab's page: its accessibility tree (aria, each element with its [ref=…], which the writing tools will take), its visible text, or only its address and title. Bounded at max_chars, and the cut is said. Secret fields (passwords, codes, tokens, cards) are never read: their value is masked, and the masked count is given.".into(),
            parameters: json!({ "type": "object", "properties": {
                "tab_id": { "type": "string", "description": "the id of a tab you took with browser_tabs" },
                "mode": { "type": "string", "enum": ["aria", "text", "url"], "description": "aria: the accessibility tree with its refs; text: the visible text; url: only the address and the title" },
                "max_chars": { "type": "integer", "description": "the most characters to bring back; default and ceiling: teammate.browser.max_chars (20 000)" }
            }, "required": ["tab_id", "mode"] }),
        },
        ToolDef {
            name: BROWSER_CONSOLE.into(),
            description: "Read a taken tab's console: its messages and uncaught errors, each with its time, its level and its text, from the sequence `since` on.".into(),
            parameters: json!({ "type": "object", "properties": {
                "tab_id": { "type": "string", "description": "the id of a tab you took with browser_tabs" },
                "since": { "type": "integer", "description": "the sequence after which to read (the `next` a previous read gave); absent, from the start" }
            }, "required": ["tab_id"] }),
        },
    ]
}

/// **The browser of a task**: its policy, the host's one listening bridge, and the task's session in the extension.
pub struct BrowserTools {
    pub policy: BrowserPolicy,
    pub ecoute: Arc<Ecoute>,
    pub session: String,
}

/// The code that reads a page's address and title: the host's own, never the model's.
const READ_URL: &str = "({ url: location.href, title: document.title })";
/// The accessibility tree, by the extension's own page function (its lines carry the [ref=…] the writing tools take).
const READ_ARIA: &str = "globalThis.__browsermcpPage.aria({})";
/// The visible text.
const READ_TEXT: &str = "(() => { void globalThis.__browsermcpPage; return document.body ? document.body.innerText : ''; })()";
/// What a secret field's name holds (a password, a one-time code, a token, a card): its value is never read.
const SECRET_NAMES: &[&str] = &["mot de passe", "password", "passcode", "passphrase", "code", "otp", "token", "secret", "carte", "card", "cvv", "cvc"];

/// The host of an address (`https://docs.example.org/guide` → `docs.example.org`).
fn host_of(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
    host.split(':').next().unwrap_or_default().to_ascii_lowercase()
}

/// **Mask the secret fields of an accessibility tree**: a field whose name holds a secret keeps its line and its ref,
/// and its value becomes « [masqué] ». Gives the tree and how many values were masked.
fn mask_secrets(tree: &str) -> (String, usize) {
    let mut masked = 0;
    let lines: Vec<String> = tree
        .lines()
        .map(|line| {
            let name = line.split('"').nth(1).unwrap_or_default().to_lowercase();
            let field = ["textbox", "searchbox", "combobox", "spinbutton"].iter().any(|r| line.trim_start().starts_with(&format!("- {r}")));
            match line.rfind("]: ") {
                Some(i) if field && SECRET_NAMES.iter().any(|s| name.contains(s)) => {
                    masked += 1;
                    format!("{}]: [masqué]", &line[..i])
                }
                _ => line.to_string(),
            }
        })
        .collect();
    (lines.join("\n"), masked)
}

impl BrowserTools {
    /// Serve `name` with `args`; `None` when `name` is not of the family.
    pub fn call(&self, name: &str, args: &Value) -> Option<Result<Value, String>> {
        match name {
            BROWSER_TABS => Some(self.tabs(args)),
            BROWSER_READ => Some(self.read(args)),
            BROWSER_CONSOLE => Some(self.console(args)),
            _ => None,
        }
    }

    /// The one browser the extension linked; said in clear when none is.
    fn bridge(&self) -> Result<crate::pont_navigateur::pont::Pont, String> {
        let ids = self.ecoute.navigateurs();
        let id = ids.first().ok_or("aucun navigateur relié : l'extension rag3weaver n'est pas ouverte dans Chrome (ou son hôte natif n'est pas installé)")?;
        self.ecoute.pont(id).ok_or_else(|| format!("le navigateur {id} s'est déconnecté"))
    }

    /// Send `command` in the task's session, and give its data.
    fn send(&self, mut command: crate::pont_navigateur::protocole::Commande) -> Result<Value, String> {
        command.session = Some(self.session.clone());
        let r = self.bridge()?.envoyer(command, std::time::Duration::from_secs(self.policy.timeout_s.max(1))).map_err(|e| format!("le navigateur refuse : {e}"))?;
        Ok(r.data.unwrap_or(Value::Null))
    }

    /// Whether `url`'s host is under a declared domain (a name allows its subdomains).
    fn allowed(&self, url: &str) -> bool {
        let host = host_of(url);
        !host.is_empty() && self.policy.domains.iter().map(|d| d.trim().trim_start_matches('.').to_ascii_lowercase()).any(|d| !d.is_empty() && (host == d || host.ends_with(&format!(".{d}"))))
    }

    fn refusal(&self, what: &str, url: &str) -> String {
        let domains = if self.policy.domains.is_empty() { "aucun".to_string() } else { self.policy.domains.join(", ") };
        format!("{what} est sur {}, hors des domaines déclarés (teammate.browser.domains : {domains})", host_of(url))
    }

    /// `exec` in the extension's engine world, of the host's own code.
    fn exec(&self, tab: &str, code: &str) -> Result<Value, String> {
        let mut c = crate::pont_navigateur::protocole::Commande::nouvelle("exec");
        c.page = Some(tab.to_string());
        c.world = Some("engine".into());
        c.code = Some(code.to_string());
        self.send(c)
    }

    /// The address and the title of a taken tab, refused outside the declared domains before anything of the page is
    /// read.
    fn guarded(&self, tab: &str) -> Result<(String, String), String> {
        let v = self.exec(tab, READ_URL)?;
        let url = v["url"].as_str().unwrap_or_default().to_string();
        if !self.allowed(&url) {
            return Err(self.refusal(&format!("la page de l'onglet {tab}"), &url));
        }
        Ok((url, v["title"].as_str().unwrap_or_default().to_string()))
    }

    fn tabs(&self, args: &Value) -> Result<Value, String> {
        let listed = self.send(crate::pont_navigateur::protocole::Commande::nouvelle("user-tabs"))?;
        let tabs: Vec<Value> = listed["tabs"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|t| {
                let url = t["url"].as_str().unwrap_or_default();
                json!({ "tab_id": t["tabId"].as_u64().map(|i| i.to_string()).unwrap_or_default(), "url": url, "title": t["title"], "allowed": self.allowed(url) })
            })
            .collect();
        let Some(claim) = args["claim"].as_str().or_else(|| args["claim"].as_u64().map(|_| "")).map(str::to_string) else {
            return Ok(json!({ "tabs": tabs }));
        };
        let claim = if claim.is_empty() { args["claim"].to_string() } else { claim };
        let tab = tabs.iter().find(|t| t["tab_id"] == claim.as_str()).ok_or_else(|| format!("aucun onglet {claim} : browser_tabs liste ceux qui existent"))?;
        let url = tab["url"].as_str().unwrap_or_default();
        if !self.allowed(url) {
            return Err(self.refusal(&format!("l'onglet {claim}"), url));
        }
        let mut c = crate::pont_navigateur::protocole::Commande::nouvelle("claim");
        c.claim = Some(crate::pont_navigateur::protocole::Claim { tab_id: claim.parse().ok(), expected_url: Some(url.to_string()), ..Default::default() });
        let taken = self.send(c)?;
        Ok(json!({ "tab_id": claim, "url": taken["url"].as_str().unwrap_or(url), "title": taken["title"].as_str().or(tab["title"].as_str()) }))
    }

    fn read(&self, args: &Value) -> Result<Value, String> {
        let tab = args["tab_id"].as_str().ok_or("browser_read : tab_id manque (un onglet pris par browser_tabs)")?;
        let mode = args["mode"].as_str().unwrap_or("aria");
        let (url, title) = self.guarded(tab)?;
        let raw = match mode {
            "url" => String::new(),
            "aria" => self.exec(tab, READ_ARIA)?.as_str().unwrap_or_default().to_string(),
            "text" => self.exec(tab, READ_TEXT)?.as_str().unwrap_or_default().to_string(),
            other => return Err(format!("browser_read : mode « {other} » inconnu (aria, text ou url)")),
        };
        let (content, masked) = if mode == "aria" { mask_secrets(&raw) } else { (raw, 0) };
        let max = args["max_chars"].as_u64().map_or(self.policy.max_chars, |m| (m as usize).min(self.policy.max_chars)).max(1);
        let truncated = content.chars().count() > max;
        let content = if truncated { format!("{} […]", content.chars().take(max).collect::<String>()) } else { content };
        Ok(json!({ "url": url, "title": title, "content": content, "truncated": truncated, "masked_fields": masked }))
    }

    fn console(&self, args: &Value) -> Result<Value, String> {
        let tab = args["tab_id"].as_str().ok_or("browser_console : tab_id manque (un onglet pris par browser_tabs)")?;
        self.guarded(tab)?;
        let mut c = crate::pont_navigateur::protocole::Commande::nouvelle("console");
        c.page = Some(tab.to_string());
        c.stream_read = Some(match args["since"].as_u64() {
            Some(s) => json!({ "afterSequence": s }),
            None => json!({}),
        });
        let v = self.send(c)?;
        let messages: Vec<Value> = v["items"].as_array().into_iter().flatten().map(|m| json!({ "at": m["at"], "level": m["level"], "text": m["text"] })).collect();
        Ok(json!({ "messages": messages, "next": v["next"] }))
    }
}
