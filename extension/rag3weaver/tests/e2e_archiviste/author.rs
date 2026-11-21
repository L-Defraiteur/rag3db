//! **The author is set by the host** (M7 of the long memory; decided with c7, 21 November 2026): `writes.author`
//! fills a row's `author` with the key of the agent the call is made for (`Backend::call_as`), never with a value
//! the tool's caller gives. A row written by a call with no agent behind it (a direct MCP client) has no author: never
//! an invented key. An author given by the tool is refused, saying what to do. A file of its own (c7's rule).

use super::*;

const AGENT: &str = "agent:reviewer-a1b2c3";

fn fiche(b: &Backend, name: &str) -> Value {
    appel(b, "get_fiche", json!({"record": {"name": name}}))["result"].clone()
}

/// An author given by the tool's caller is refused, by its name, and the refusal says what to do.
#[test]
#[ignore]
fn an_author_given_by_the_tool_is_refused() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    let e = b.call("put_fiche", json!({"record": {"name": "spoof", "about": "x", "state": "current", "author": AGENT}})).expect_err("refused");
    assert!(e.contains("writes.author") && e.contains("remove it from the call"), "{e}");
}

/// A call made for an agent signs its rows with the agent's key: the fiche, and the Source and Apport it files.
#[test]
#[ignore]
fn a_call_made_for_an_agent_signs_its_rows_with_the_agent_key() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    b.call_as(Some(AGENT), "put_fiche", json!({"record": {"name": "signed", "about": "what the reviewer learned", "state": "current"}})).expect("put_fiche");
    assert!(fiche(&b, "signed").to_string().contains(AGENT), "the fiche carries the key: {}", fiche(&b, "signed"));
    b.call_as(Some(AGENT), "put_source", json!({"record": {"chemin": "sources/signed.md", "titre": "t", "contenu": "c", "origine": "envoi"}})).expect("put_source");
    b.call_as(Some(AGENT), "put_apport", json!({"record": {"fiche": "signed", "source": "sources/signed.md", "section": "plan", "texte": "t", "niveau": "L1", "state": "current"}})).expect("put_apport");
    let apports = appel(&b, "recall_apports", json!({"query": "t", "options": {"limit": 5, "filter_condition": {"field": {"key": "fiche", "value": "signed"}}}}));
    assert!(apports.to_string().contains(AGENT), "the Apport carries the key: {apports}");
}

/// A call with no agent behind it (a direct MCP client): no author on the row, never an invented one.
#[test]
#[ignore]
fn a_call_with_no_agent_writes_no_author() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    appel(&b, "put_fiche", json!({"record": {"name": "unsigned", "about": "x", "state": "current"}}));
    let f = fiche(&b, "unsigned");
    let author = f.pointer("/author").or_else(|| f.pointer("/data/author")).cloned().unwrap_or(Value::Null);
    assert!(author.is_null() || author == json!(""), "no author: {f}");
}
