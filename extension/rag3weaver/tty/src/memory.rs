//! **The L1 of a run, and the turns they came from** (vision of the archivist, 18 nov. 2026,
//! `docs/memoire/18-11-2026/02-l-archiviste-en-deux-etapes.md`, « La chaîne, du L1 jusqu'au tour ») :
//! `/memory` lists the L1 of the run with their range of turns and their fiche, `/turns <n>` opens the
//! turns of one in the viewer, each with its state as the host says it (`indexed`, `from_chat_file`,
//! `gone`). The host's routes (`GET /ide/l1`, `POST /ide/turns/read`) are archi's; the terminal reads
//! the shape agreed on 18 nov. and follows archi's final names.

use crate::vue::{Genre, Validee};
use serde_json::Value;

/// The lines `/memory` says, from `GET /ide/l1` (`{l1: [{apport, fiche, section, run, from_turn, to_turn}]}`).
pub fn l1_lines(v: &Value) -> Vec<String> {
    let l1 = v["l1"].as_array().cloned().unwrap_or_default();
    if l1.is_empty() {
        return vec!["aucun L1 pour ce run".to_string()];
    }
    let run = l1[0]["run"].as_str().unwrap_or("?");
    let mut out = vec![format!("les L1 du run {run} (/turns <n> ouvre ses tours) :")];
    for (i, x) in l1.iter().enumerate() {
        let title = x["section"].as_str().map(|s| s.split_once(" · ").map_or(s, |(_, t)| t)).unwrap_or("");
        out.push(format!("  {}. fiche:{} — {title} · {}", i + 1, x["fiche"].as_str().unwrap_or("?"), range(x)));
    }
    out
}

/// « tours 12 à 15 », or « tour 16 » for a single one; an L1 drawn from a document has no range.
pub fn range(x: &Value) -> String {
    match (x["from_turn"].as_u64(), x["to_turn"].as_u64()) {
        (Some(a), Some(b)) if a == b => format!("tour {a}"),
        (Some(a), Some(b)) => format!("tours {a} à {b}"),
        _ => "sans plage de tours".to_string(),
    }
}

/// The turns of an L1, for the viewer, from `POST /ide/turns/read` (`{turns: [{turn, state, messages}]}`).
pub fn turns_lines(v: &Value) -> Vec<Validee> {
    let line = |genre: Genre, texte: String| Validee { genre, texte, langue: String::new(), tete: 0 };
    let mut out = Vec::new();
    for t in v["turns"].as_array().into_iter().flatten() {
        let turn = t["turn"].as_u64().unwrap_or(0);
        let state = t["state"].as_str().unwrap_or("?");
        if state == "gone" {
            out.push(line(Genre::Note, format!("tour {turn} · gone : ni dans l'index, ni dans le fichier du chat")));
            continue;
        }
        out.push(line(Genre::Note, format!("tour {turn} · {state}")));
        for m in t["messages"].as_array().into_iter().flatten() {
            let text = m["text"].as_str().unwrap_or("");
            match m["role"].as_str() {
                Some("tool") => out.push(line(Genre::Outil, format!("  ⎿ {} ({}) {text}", m["tool_name"].as_str().unwrap_or("?"), m["status"].as_str().unwrap_or("?")))),
                role => out.push(line(Genre::Boite, format!("[{}] {text}", role.unwrap_or("?")))),
            }
        }
    }
    out
}

/// **What `/turns <n>` asks of `POST /ide/turns/read`** for the `n`-th L1 (from 1): the host's route takes
/// `{run, from, to}`, the L1's run and range (archi's route on master, 19 nov.: by apport it is refused, until the
/// memory's trace_turn).
pub fn turns_request(v: &Value, n: usize) -> Option<Value> {
    let l1 = v["l1"].as_array()?.get(n.checked_sub(1)?)?;
    Some(serde_json::json!({ "run": l1["run"].as_str()?, "from": l1["from_turn"].as_u64()?, "to": l1["to_turn"].as_u64()? }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The host's route takes the L1's run and range, not its apport (refused on master, 19 nov.).
    #[test]
    fn turns_ask_the_host_for_the_run_and_range_of_the_l1() {
        let v = json!({ "l1": [
            { "apport": "a-1", "run": "t:d", "from_turn": 12, "to_turn": 15 },
            { "apport": "a-2", "run": "t:d", "from_turn": 16, "to_turn": 16 }
        ] });
        assert_eq!(turns_request(&v, 1), Some(json!({ "run": "t:d", "from": 12, "to": 15 })));
        assert_eq!(turns_request(&v, 2), Some(json!({ "run": "t:d", "from": 16, "to": 16 })));
        assert_eq!(turns_request(&v, 3), None, "no third L1");
        assert_eq!(turns_request(&json!({ "l1": [{ "apport": "a-3", "run": "t:d" }] }), 1), None, "an L1 without a range asks nothing");
    }

    fn l1s() -> Value {
        json!({ "l1": [
            { "apport": "a-1", "fiche": "tty-terminal", "section": "turns 12-15 · le mode plain attend la fin", "run": "agent-1a", "from_turn": 12, "to_turn": 15 },
            { "apport": "a-2", "fiche": "archiviste", "section": "turns 16-16 · une question sur les L1", "run": "agent-1a", "from_turn": 16, "to_turn": 16 }
        ] })
    }

    /// `/memory` says each L1 of the run, numbered, its fiche in the margin, its range of turns in clear.
    #[test]
    fn memory_lists_the_l1_of_the_run_with_their_range_and_fiche() {
        assert_eq!(l1_lines(&l1s()), vec![
            "les L1 du run agent-1a (/turns <n> ouvre ses tours) :".to_string(),
            "  1. fiche:tty-terminal — le mode plain attend la fin · tours 12 à 15".to_string(),
            "  2. fiche:archiviste — une question sur les L1 · tour 16".to_string(),
        ]);
        assert_eq!(l1_lines(&json!({ "l1": [] })), vec!["aucun L1 pour ce run".to_string()]);
    }

    /// The turns of an L1, each with its state said as the host says it; a turn gone is said, not empty.
    #[test]
    fn the_turns_of_an_l1_say_their_state_and_a_gone_turn_is_said() {
        let v = json!({ "turns": [
            { "turn": 12, "state": "indexed", "messages": [
                { "role": "user", "text": "lance le plain" },
                { "role": "tool", "tool_name": "run_command", "status": "ok", "text": "fait" },
                { "role": "assistant", "text": "Voici." }
            ] },
            { "turn": 13, "state": "from_chat_file", "messages": [{ "role": "user", "text": "encore" }] },
            { "turn": 14, "state": "gone", "messages": [] }
        ] });
        let lines: Vec<String> = turns_lines(&v).into_iter().map(|l| l.texte).collect();
        assert_eq!(lines, vec![
            "tour 12 · indexed".to_string(),
            "[user] lance le plain".to_string(),
            "  ⎿ run_command (ok) fait".to_string(),
            "[assistant] Voici.".to_string(),
            "tour 13 · from_chat_file".to_string(),
            "[user] encore".to_string(),
            "tour 14 · gone : ni dans l'index, ni dans le fichier du chat".to_string(),
        ]);
    }
}
