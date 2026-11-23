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

/// **The short texts of the teammate's loop** (the step's message, the relaunches), one key each, `{$name}` variables:
/// the shipped `texts.json`, each key the user's `texts.json` names taking its place (23 Nov 2026).
#[derive(Debug, Clone)]
pub(crate) struct Texts(std::collections::BTreeMap<String, String>);

const SHIPPED_TEXTS: &str = include_str!("../../templates/teammate/prompts/texts.json");

impl Texts {
    /// The shipped texts alone.
    pub(crate) fn shipped() -> Self {
        Self(serde_json::from_str(SHIPPED_TEXTS).expect("templates/teammate/prompts/texts.json parses"))
    }

    /// The key's text, its variables filled; the key itself when no text has it (said by its name, never empty).
    pub(crate) fn get(&self, key: &str, vars: &[(&str, &str)]) -> String {
        fill(self.0.get(key).map(String::as_str).unwrap_or(key), vars)
    }
}

impl DaemonHost {
    /// **The loop's texts, read now**: the shipped ones, with the keys of the user's `texts.json` in their place (a
    /// file that does not parse is said once on stderr, and the shipped texts stay).
    pub(crate) fn prompt_texts(&self) -> Texts {
        let mut t = Texts::shipped();
        if let Some(j) = self.user_prompt_file("texts.json") {
            match serde_json::from_str::<std::collections::BTreeMap<String, String>>(&j) {
                Ok(m) => t.0.extend(m),
                Err(e) => eprintln!("[équipier] texts.json de l'utilisatrice illisible ({e}) : les textes livrés restent"),
            }
        }
        t
    }

    /// The user's own prompt file named as it is (`texts.json`), read now.
    pub(crate) fn user_prompt_file(&self, file: &str) -> Option<String> {
        let roots = self.racines_de_code().ok()?;
        std::fs::read_to_string(roots.config.join(USER_PROMPTS).join(file)).ok().filter(|t| !t.trim().is_empty())
    }
}
