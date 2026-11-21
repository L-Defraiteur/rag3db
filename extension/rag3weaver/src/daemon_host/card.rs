//! **The approval card** (page 03 of rag3weaver-archi, 21 Nov 2026): one card per action waiting
//! for a decision. It opens with the tool, its arguments, the action's class and the scope asked;
//! its state changes as the gate goes (looking for an authorization, the agent's appeal, waiting
//! for the survivor); it closes **once**, and says who decided. A later closing (a late timer, a
//! recall after the user answered) is written as such and changes nothing. The card lives in the
//! task's mailbox: its lines are the mailbox's lines, published on the task's bus like any other.

use serde_json::{json, Value};

use super::teammate::TeammateFiles;

/// Closing a card is checked and written under one lock: two deciders at the same instant (the
/// user and the recall) cannot both close it.
static FERMETURES: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Opens a card: an `approval` line from the teammate, with the tool, its arguments, the class and
/// the scope (`once` or `task`). A `run_command` card keeps `command`, as the cards before it did.
pub fn open_card(files: &TeammateFiles, tool: &str, args: &Value, class: &str, scope: &str) -> Result<String, String> {
    if !matches!(scope, "once" | "task") {
        return Err(format!("a card's scope is once or task, not « {scope} »"));
    }
    let mut extra = json!({ "tool": tool, "args": args, "class": class, "scope": scope });
    let commande = (tool == "run_command").then(|| args["command"].as_str().map(str::to_string)).flatten();
    if let Some(c) = &commande {
        extra["command"] = json!(c);
    }
    let texte = match &commande {
        Some(c) => format!("commande en attente d'accord : {c}"),
        None => format!("{tool} en attente d'accord : {args}"),
    };
    let ligne = files.append_with("teammate", "approval", None, &texte, &extra)?;
    ligne["id"].as_str().map(str::to_string).ok_or_else(|| "the card's line has no id".into())
}

/// Says where the card is: an `approval_state` line from the host (`stage`, a text, an optional
/// deadline in ms). Refused on a closed card.
pub fn card_state(files: &TeammateFiles, card: &str, stage: &str, text: &str, until: Option<u64>) -> Result<(), String> {
    let _garde = FERMETURES.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(f) = card_closing(files, card) {
        return Err(format!("card {card} is closed (by {}): no more state", f["from"].as_str().unwrap_or("?")));
    }
    files.append_with("host", "approval_state", None, text, &json!({ "reply_to": card, "stage": stage, "until": until }))?;
    Ok(())
}

/// The closing of the card, if it is closed: the first `answer` that replies to it.
pub fn card_closing(files: &TeammateFiles, card: &str) -> Option<Value> {
    files.mailbox().into_iter().find(|l| l["kind"] == "answer" && l["reply_to"] == card)
}

/// Closes the card once: `true` when this closing is the one, `false` when the card was already
/// closed (then a host line says who had closed it, and this late one is kept without effect).
pub fn close_card(files: &TeammateFiles, card: &str, from: &str, approved: bool, extra: &Value) -> Result<bool, String> {
    let _garde = FERMETURES.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(f) = card_closing(files, card) {
        let qui = f["from"].as_str().unwrap_or("?").to_string();
        files.append_with(
            "host",
            "report",
            None,
            &format!("card {card} already closed by {qui}: the closing of {from} is kept without effect"),
            &json!({ "reply_to": card, "late_from": from, "late_approved": approved }),
        )?;
        return Ok(false);
    }
    let mut ligne = json!({ "reply_to": card, "approved": approved });
    if let (Some(l), Some(e)) = (ligne.as_object_mut(), extra.as_object()) {
        for (k, v) in e {
            if k != "reply_to" && k != "approved" {
                l.insert(k.clone(), v.clone());
            }
        }
    }
    let texte = if approved { format!("card {card}: approved by {from}") } else { format!("card {card}: refused by {from}") };
    files.append_with(from, "answer", None, &texte, &ligne)?;
    Ok(true)
}
