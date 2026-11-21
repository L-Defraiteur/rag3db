//! The witnesses of `GET /ide/run`, in a file of their own.

use super::*;

/// A root agent run and the graph run of its tool, written to the journal: the tree gives the root,
/// its step, its model call and its tool call (arguments, excerpt, outcome), and the child with its
/// nodes played; an unknown run is an error that names it.
#[test]
fn a_run_is_given_back_as_a_tree() {
    let d = tempfile::tempdir().unwrap();
    let host = DaemonHost::new().with_runs_root(d.path().join("runs"));
    let evenements = json!([
        { "kind": "RunStarted", "run": "r1", "parent": null, "run_kind": "agent", "name": "equipier", "step": { "task": "doc", "number": 2 } },
        { "kind": "LlmCall", "run": "r1", "agent": "equipier", "model": "faux", "iteration": 1, "prompt_tokens": 10, "completion_tokens": 2, "ms": 30, "finish": "tool_calls", "tool_calls": 1 },
        { "kind": "ToolCallStarted", "run": "r1", "agent": "equipier", "call_id": "c1", "tool": "run_command", "arguments": "{\"command\":\"cargo test\"}" },
        { "kind": "RunStarted", "run": "r2", "parent": "r1", "run_kind": "graph", "name": "Run", "target": { "tool": "run_command" } },
        { "kind": "NodeRun", "run": "r2", "node": "run", "node_type": "RunCommandNode", "ms": 900, "error": null },
        { "kind": "RunFinished", "run": "r2", "run_kind": "graph", "ms": 905, "ok": true },
        { "kind": "ToolCallFinished", "run": "r1", "agent": "equipier", "call_id": "c1", "tool": "run_command", "ok": true, "error_kind": null, "ms": 910, "bytes": 40, "result_excerpt": "code 0 · test result: ok", "truncated": false },
        { "kind": "RunFinished", "run": "r1", "run_kind": "agent", "ms": 1000, "ok": true }
    ]);
    host.journal_des_runs().deposer("b", "r1", json!({ "evenements": evenements })).unwrap();
    let t = host.run_tree("b", "r1").expect("the run is in the journal");
    assert_eq!(t["run"], json!("r1"));
    assert_eq!(t["backend"], json!("b"));
    let racine = &t["tree"];
    assert_eq!(racine["run"], json!("r1"));
    assert_eq!(racine["kind"], json!("agent"));
    assert_eq!(racine["step"], json!({ "task": "doc", "number": 2 }));
    assert_eq!(racine["ok"], json!(true));
    assert_eq!(racine["llm_calls"][0]["model"], json!("faux"));
    let appel = &racine["tool_calls"][0];
    assert_eq!(appel["call_id"], json!("c1"));
    assert_eq!(appel["tool"], json!("run_command"));
    assert_eq!(appel["arguments"], json!({ "command": "cargo test" }), "arguments parsed when they are JSON");
    assert_eq!(appel["ok"], json!(true));
    assert_eq!(appel["result_excerpt"], json!("code 0 · test result: ok"));
    let enfant = &racine["children"][0];
    assert_eq!(enfant["run"], json!("r2"));
    assert_eq!(enfant["target"], json!({ "tool": "run_command" }));
    assert_eq!(enfant["nodes"][0]["node_type"], json!("RunCommandNode"));
    let e = host.run_tree("b", "inconnu").unwrap_err();
    assert!(e.contains("inconnu"), "{e}");
}
