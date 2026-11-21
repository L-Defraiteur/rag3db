//! **A run as a tree, read-only** (`GET /ide/run?backend=&run=`; the IDE's agent page and the
//! reviewer, 21 Nov 2026). A root run's journal holds its events and its children's: this module
//! gives them back as one tree, each run with its nodes played, its model calls and its tool calls
//! (arguments, result excerpt, outcome, duration). It writes nothing.

use serde_json::{json, Value};

use super::DaemonHost;

impl DaemonHost {
    /// The tree of the run `run` of the backend `backend`, from its journal.
    pub fn run_tree(&self, backend: &str, run: &str) -> Result<Value, String> {
        let journal = self
            .journal_des_runs()
            .lire(backend, run)
            .ok_or_else(|| format!("run « {run} » of {backend}: not in the runs journal (not written yet, or gone)"))?;
        let evenements = journal["evenements"].as_array().cloned().unwrap_or_default();
        let mut runs: std::collections::BTreeMap<String, Value> = std::collections::BTreeMap::new();
        let mut ordre: Vec<String> = Vec::new();
        let mut entree = |id: &str, runs: &mut std::collections::BTreeMap<String, Value>| {
            if !runs.contains_key(id) {
                ordre.push(id.to_string());
                runs.insert(id.to_string(), json!({ "run": id, "parent": null, "kind": null, "name": null, "target": null, "step": null,
                    "ok": null, "ms": null, "nodes": [], "llm_calls": [], "tool_calls": [], "waits": [], "children": [] }));
            }
        };
        for e in &evenements {
            let Some(id) = e["run"].as_str() else { continue };
            entree(id, &mut runs);
            let r = runs.get_mut(id).expect("just inserted");
            match e["kind"].as_str().unwrap_or_default() {
                "RunStarted" => {
                    r["parent"] = e["parent"].clone();
                    r["kind"] = e["run_kind"].clone();
                    r["name"] = e["name"].clone();
                    r["target"] = e["target"].clone();
                    r["step"] = e["step"].clone();
                }
                "RunFinished" => {
                    r["ok"] = e["ok"].clone();
                    r["ms"] = e["ms"].clone();
                    if !e["cause"].is_null() {
                        r["cause"] = e["cause"].clone();
                    }
                }
                "NodeRun" => push(r, "nodes", json!({ "node": e["node"], "node_type": e["node_type"], "ms": e["ms"], "error": e["error"] })),
                "LlmCall" => push(r, "llm_calls", json!({ "model": e["model"], "iteration": e["iteration"], "prompt_tokens": e["prompt_tokens"],
                    "completion_tokens": e["completion_tokens"], "ms": e["ms"], "finish": e["finish"], "tool_calls": e["tool_calls"] })),
                "ToolCallStarted" => {
                    let brut = e["arguments"].as_str().unwrap_or_default();
                    let arguments = serde_json::from_str::<Value>(brut).unwrap_or_else(|_| json!(brut));
                    push(r, "tool_calls", json!({ "call_id": e["call_id"], "tool": e["tool"], "arguments": arguments, "ok": null }));
                }
                "ToolCallFinished" => {
                    let appels = r["tool_calls"].as_array_mut().expect("array");
                    let fin = json!({ "ok": e["ok"], "error_kind": e["error_kind"], "ms": e["ms"], "bytes": e["bytes"],
                        "result_excerpt": e["result_excerpt"], "truncated": e["truncated"] });
                    match appels.iter_mut().rev().find(|a| a["call_id"] == e["call_id"]) {
                        Some(a) => a.as_object_mut().expect("object").extend(fin.as_object().cloned().unwrap_or_default()),
                        None => {
                            let mut a = json!({ "call_id": e["call_id"], "tool": e["tool"], "arguments": null });
                            a.as_object_mut().expect("object").extend(fin.as_object().cloned().unwrap_or_default());
                            appels.push(a);
                        }
                    }
                }
                "Waiting" | "Resumed" => push(r, "waits", e.clone()),
                _ => {}
            }
        }
        if !runs.contains_key(run) {
            return Err(format!("run « {run} » of {backend}: its journal holds no event of it"));
        }
        // Children under their parent, deepest first so that a child carries its own children.
        for id in ordre.iter().rev() {
            if id == run {
                continue;
            }
            let Some(parent) = runs.get(id).and_then(|r| r["parent"].as_str().map(str::to_string)) else { continue };
            if parent == *id || !runs.contains_key(&parent) {
                continue;
            }
            let enfant = runs.remove(id).expect("present");
            if let Some(p) = runs.get_mut(&parent) {
                p["children"].as_array_mut().expect("array").insert(0, enfant);
            }
        }
        let mut ajouts = journal.as_object().cloned().unwrap_or_default();
        ajouts.remove("evenements");
        Ok(json!({
            "run": run,
            "backend": backend,
            "started_at": journal["debut"],
            "tree": runs.remove(run).expect("present"),
            "added": ajouts,
        }))
    }
}

fn push(r: &mut Value, cle: &str, v: Value) {
    if let Some(a) = r[cle].as_array_mut() {
        a.push(v);
    }
}

#[cfg(test)]
#[path = "run_tree_tests.rs"]
mod tests;
