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

impl BrowserTools {
    /// Serve `name` with `args`; `None` when `name` is not of the family.
    pub fn call(&self, name: &str, _args: &Value) -> Option<Result<Value, String>> {
        match name {
            BROWSER_TABS | BROWSER_READ | BROWSER_CONSOLE => Some(Err("pas encore".into())),
            _ => None,
        }
    }
}
