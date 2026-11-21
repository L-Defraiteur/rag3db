//! **The title of a human** (21 Nov 2026, the gate of the auto mode): a mailbox line counts as the
//! user's only when the host says so, from WHO calls, never from the body. The user's token
//! (`daemon_lifecycle::user_token`) is that title: sent by the terminal in `X-Rag3weaver-User`
//! after a keystroke, or carried by the IDE's user cookie, which a one-use entry code gives.

use serde_json::{json, Value};

use super::DaemonHost;

/// The life of an entry code: long enough to open the URL, short enough not to matter in a history.
pub const ENTRY_CODE_TTL_MS: u128 = 60_000;

impl DaemonHost {
    /// The host that knows the user's token.
    pub fn with_user_token(mut self, token: String) -> Self {
        self.user_token = Some(token);
        self
    }

    /// A one-use entry code for the IDE, asked with the user's token (`X-Rag3weaver-User`): an agent
    /// that has only the host's token gets none.
    pub fn issue_entry_code(&self, user_header: Option<&str>) -> Result<String, String> {
        if !self.is_user_token(user_header) {
            return Err("an entry code is asked with the user's token (X-Rag3weaver-User): `rag3weaver-backend ide` reads it".into());
        }
        let code = crate::daemon_lifecycle::new_token()?;
        let maintenant = now_ms();
        let mut codes = self.entry_codes.lock().unwrap_or_else(|p| p.into_inner());
        codes.retain(|_, fin| *fin > maintenant);
        codes.insert(code.clone(), maintenant + ENTRY_CODE_TTL_MS);
        Ok(code)
    }

    /// Redeems an entry code once: the user's token for the cookie, or `None` (unknown, used, expired).
    pub fn redeem_entry_code(&self, code: &str) -> Option<String> {
        let fin = self.entry_codes.lock().unwrap_or_else(|p| p.into_inner()).remove(code)?;
        (fin > now_ms()).then(|| self.user_token.clone()).flatten()
    }

    /// Whether `given` is the user's token (compared as the host compares its own).
    pub fn is_user_token(&self, given: Option<&str>) -> bool {
        match (&self.user_token, given) {
            (Some(u), Some(g)) => super::same_token(&format!("Bearer {g}"), &format!("Bearer {u}")),
            _ => false,
        }
    }

    /// **Who wrote a line**: `{kind: "human" | "agent", client}`, from the user cookie or the
    /// `X-Rag3weaver-User` header, never from the body.
    pub fn by_of(&self, user_cookie: Option<&str>, user_header: Option<&str>, client: &str) -> Value {
        let humain = self.is_user_token(user_cookie) || self.is_user_token(user_header);
        json!({ "kind": if humain { "human" } else { "agent" }, "client": client })
    }
}

fn now_ms() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}

#[cfg(test)]
#[path = "user_title_tests.rs"]
mod tests;
