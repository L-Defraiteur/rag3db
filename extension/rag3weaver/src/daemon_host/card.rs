//! **The approval card** (page 03 of rag3weaver-archi, 21 Nov 2026): one card per action waiting
//! for a decision. It opens with the tool, its arguments, the action's class and the scope asked;
//! its state changes as the gate goes (looking for an authorization, the agent's appeal, waiting
//! for the survivor); it closes **once**, and says who decided. A later closing (a late timer, a
//! recall after the user answered) is written as such and changes nothing. The card lives in the
//! task's mailbox: its lines are the mailbox's lines, published on the task's bus like any other.

use serde_json::{json, Value};

use super::teammate::TeammateFiles;

/// Opens a card: an `approval` line from the teammate, with the tool, its arguments, the class and
/// the scope (`once` or `task`). A `run_command` card keeps `command`, as the cards before it did.
pub fn open_card(files: &TeammateFiles, tool: &str, args: &Value, class: &str, scope: &str) -> Result<String, String> {
    let _ = (files, tool, args, class, scope);
    Err("not yet".into())
}

/// Says where the card is: a `approval_state` line from the host (`stage`, a text, an optional
/// deadline in ms). Refused on a closed card.
pub fn card_state(files: &TeammateFiles, card: &str, stage: &str, text: &str, until: Option<u64>) -> Result<(), String> {
    let _ = (files, card, stage, text, until);
    Err("not yet".into())
}

/// The closing of the card, if it is closed.
pub fn card_closing(files: &TeammateFiles, card: &str) -> Option<Value> {
    let _ = (files, card);
    None
}

/// Closes the card once: `true` when this closing is the one, `false` when the card was already
/// closed (then a host line says who had closed it, and this late one is kept without effect).
pub fn close_card(files: &TeammateFiles, card: &str, from: &str, approved: bool, extra: &Value) -> Result<bool, String> {
    let _ = (files, card, from, approved, extra);
    Err("not yet".into())
}
