//! **The teammate's prompt, as template files** (Lucie, 23 Nov 2026: « comment ça se fait que son invite ne soit pas
//! un gabarit modifiable sans tout relancer ? »). Each text that reaches the model is a file shipped under
//! `templates/teammate/prompts/<name>.md`; the user's own file, `<config>/templates/teammate/prompts/<name>.md`, takes
//! its place; it is read again at each step, so that an edit is in the next turn. Variables are written `{$name}`.

use super::DaemonHost;

/// The folder of the user's own prompt files, under the host's configuration.
const USER_PROMPTS: &str = "templates/teammate/prompts";

/// `{$name}` replaced by its value, for each pair; an unknown variable stays as it is, said by nothing.
pub(crate) fn fill(text: &str, vars: &[(&str, &str)]) -> String {
    let mut out = text.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{${k}}}"), v);
    }
    out
}

impl DaemonHost {
    /// **The user's own prompt file** `<name>.md`, read now; `None` when she has none (the shipped text applies).
    pub(crate) fn user_prompt(&self, name: &str) -> Option<String> {
        let roots = self.racines_de_code().ok()?;
        std::fs::read_to_string(roots.config.join(USER_PROMPTS).join(format!("{name}.md"))).ok().filter(|t| !t.trim().is_empty())
    }
}
