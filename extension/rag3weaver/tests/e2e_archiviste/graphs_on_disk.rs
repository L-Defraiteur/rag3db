//! Les graphes de l'archiviste sur disque.
//!
//! Un fichier à ce lot (règle de c7, 21 nov. 2026) : un témoin neuf ne s'écrit jamais dans le corps d'un fichier de
//! tests partagé.

use super::*;

/// **The cited graphs are on disk, in the template** (20 November 2026: a graph cited by `@builtin:` came from the
/// binary, and an agent that edited the template's copy edited nothing): a cited graph edited in the template's
/// folder changes what the call plays, and the version of the template. Here `turns_select.mmd`, cited by
/// read_turns: limited to one row, a turn of two messages shows one.
#[test]
#[ignore]
fn a_cited_graph_edited_on_disk_changes_what_the_call_plays() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    for seq in 0..2 {
        appel(&b, "put_turn", json!({"run": "chat:g", "turn": 1, "seq": seq, "at": "", "role": "user", "text": format!("message {seq}")}));
    }
    let before = appel(&b, "read_turns", json!({"run": "chat:g", "from_turn": 1, "to_turn": 1}));
    assert_eq!(before["result"]["turns"][0]["messages"].as_array().map(Vec::len), Some(2), "{before}");
    let h0 = before["metadata"]["template"]["hash"].clone();
    let cited = dir.path().join("templates/backends/memory/graphs/turns_select.mmd");
    let text = std::fs::read_to_string(&cited).unwrap();
    let edited = text.replace("limit=0", "limit=1");
    assert_ne!(edited, text, "the witness edits the cited graph");
    std::fs::write(&cited, edited).unwrap();
    b.reload().expect("the edited template reloads");
    let after = appel(&b, "read_turns", json!({"run": "chat:g", "from_turn": 1, "to_turn": 1}));
    assert_eq!(after["result"]["turns"][0]["messages"].as_array().map(Vec::len), Some(1), "the edited graph is played: {after}");
    assert_ne!(after["metadata"]["template"]["hash"], h0, "and the version changed");
}
