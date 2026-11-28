//! The witnesses of the workflow editor's first lot (28 November 2026): a declared graph read in the IDE, its nodes
//! with the typed ports their schemas declare. A real host, the shipped `memory` backend.

use serde_json::{json, Value};

/// A guarded host (token `j`) with the shipped `memory` backend opened by `describe`: its address, the backend's id.
struct Bench {
    _d: tempfile::TempDir,
    root: String,
    id: String,
    thread: std::thread::JoinHandle<()>,
}

fn get(root: &str, path: &str) -> (u16, Value) {
    let mut r = crate::daemon::agent_de_temoin()
        .get(format!("{root}{path}"))
        .header("Cookie", "rag3weaver_ide=j")
        .config()
        .http_status_as_error(false)
        .build()
        .call()
        .unwrap();
    (r.status().as_u16(), serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap())
}

fn bench() -> Bench {
    let d = tempfile::tempdir().unwrap();
    let base = d.path().join("memory");
    std::fs::create_dir_all(&base).unwrap();
    let template = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/memory");
    let manifest = crate::gabarits::jouable::manifeste_jouable(&template, &base).unwrap().to_string_lossy().to_string();
    let (tasks, chats) = (d.path().join("tasks"), d.path().join("chats"));
    let (address, thread) = super::tests::hote_garde_avec("j", |h| h.with_racine_des_taches(tasks).with_chats_root(chats));
    let root = format!("http://{address}");
    let mut r = crate::daemon::agent_de_temoin()
        .post(format!("{root}/ide/describe"))
        .header("Cookie", "rag3weaver_ide=j")
        .header("X-Rag3weaver-User", "j")
        .header("content-type", "application/json")
        .send(json!({ "manifest": manifest }).to_string())
        .unwrap();
    let described: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
    assert_eq!(described["ok"], true, "{described}");
    let (_, state) = get(&root, "/ide/etat");
    let id = state["backends"]
        .as_array()
        .and_then(|b| b.iter().find(|b| b["manifest"] == manifest.as_str()))
        .unwrap_or_else(|| panic!("the backend is not in the state: {state}"))["id"]
        .as_str()
        .unwrap()
        .to_string();
    Bench { _d: d, root, id, thread }
}

fn stop(b: Bench) {
    let (status, _) = super::tests::demander("POST", &format!("{}/arret", b.root), Some("j"));
    assert_eq!(status, 200);
    b.thread.join().unwrap();
}

/// The names of a node's ports on one side, as the served schema declares them.
fn ports(node: &Value, side: &str) -> Vec<String> {
    node["schema"][side].as_array().map(|a| a.iter().filter_map(|p| p["name"].as_str().map(str::to_string)).collect()).unwrap_or_default()
}

/// **Every edge of a shipped graph joins two ports its nodes declare** (the editor draws a wire from port to port):
/// the schema `/ide/graphe` serves for a node is its schema under its configuration, so a node whose ports follow its
/// configuration (a script's `inputs`, a route's `routes`, a collect's `inputs`) declares the ports its edges use.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn every_edge_of_a_shipped_graph_joins_two_declared_ports() {
    let b = bench();
    let mut missing = Vec::new();
    // `archive` routes by `Route(routes='none|create|fiche')`: its outputs are its routes, declared by its configuration.
    for tool in ["archive_slice", "archive"] {
        let (status, g) = get(&b.root, &format!("/ide/graphe?backend={}&tool={tool}", b.id));
        assert_eq!(status, 200, "{g}");
        let nodes = g["result"]["nodes"].as_array().unwrap();
        let node = |name: &str| nodes.iter().find(|n| n["name"] == name).unwrap_or_else(|| panic!("no node {name}: {g}"));
        for e in g["result"]["edges"].as_array().unwrap() {
            let (from, fp, to, tp) = (e["from_node"].as_str().unwrap(), e["from_port"].as_str().unwrap(), e["to_node"].as_str().unwrap(), e["to_port"].as_str().unwrap());
            if !ports(node(from), "outputs").iter().any(|p| p == fp) {
                missing.push(format!("{tool}: {from}.{fp} (out of {:?})", ports(node(from), "outputs")));
            }
            if !ports(node(to), "inputs").iter().any(|p| p == tp) {
                missing.push(format!("{tool}: {to}.{tp} (in of {:?})", ports(node(to), "inputs")));
            }
        }
        if tool == "archive_slice" {
            assert!(g["result"]["inputSchema"]["properties"].as_object().is_some_and(|p| p.contains_key("run")), "the tool's params: {g}");
        }
    }
    assert!(missing.is_empty(), "edges on ports the served schemas do not declare: {missing:#?}");
    stop(b);
}

/// **The workflow editor reads a tool's graph**: at `/ide/code/<id>/workflow/tool/<name>`, the graph's nodes; in the
/// Edit view their ports as `name : type`, a wired port's dot filled; a click on a port lights the ports it could join;
/// the Simple view hides the plumbing; what is not there yet is in place, disabled, with « bientôt ».
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
#[test]
fn the_workflow_editor_shows_a_tool_graph_with_its_typed_ports() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binary) = navigateur_ou_saut("the_workflow_editor_shows_a_tool_graph_with_its_typed_ports") else {
        return;
    };
    let b = bench();
    let mut n = Navigateur::lancer(&binary);
    n.aller(&format!("{}/ide/entrer?jeton=j", b.root), "location.pathname.startsWith('/ide/')");
    n.aller(&format!("{}/ide/code/{}/workflow/tool/archive_slice", b.root, b.id), "!!document.querySelector('#workflow [data-node]')");
    let names = n.evaluer("[...document.querySelectorAll('#workflow [data-node]')].map(e => e.dataset.node).sort().join(',')");
    assert_eq!(names, json!("core,done,render,slice"), "the Simple view shows the graph's nodes");
    assert!(n.evaluer("document.querySelector('#workflow').innerText").as_str().unwrap_or_default().contains("archive_slice.mmd"), "the file is named");

    n.cliquer("#workflow [data-view=edit]");
    n.attendre("document.querySelectorAll('#workflow [data-port]').length > 0", "the ports in the Edit view");
    let label = n.evaluer("document.querySelector('#workflow [data-port=\"core:in:elements\"]')?.innerText ?? ''");
    assert!(label.as_str().unwrap_or_default().starts_with("elements : "), "a port reads name : type: {label}");
    assert_eq!(n.evaluer("document.querySelector('#workflow [data-port=\"core:in:elements\"]').dataset.wired"), json!("true"), "a wired port");
    n.cliquer("#workflow [data-port=\"slice:out:result\"]");
    n.attendre("document.querySelectorAll('#workflow [data-port][data-lit=\"true\"]').length > 0", "the compatible ports lit");
    let lit = n.evaluer("[...document.querySelectorAll('#workflow [data-port][data-lit=\"true\"]')].map(e => e.dataset.port)");
    assert!(lit.as_array().unwrap().iter().all(|p| p.as_str().unwrap().contains(":in:") && !p.as_str().unwrap().starts_with("slice:")), "only inputs of other nodes light: {lit}");

    let soon = n.evaluer("[...document.querySelectorAll('#workflow [data-soon]')].map(e => [e.dataset.soon, e.disabled || e.getAttribute('aria-disabled') === 'true', e.textContent.includes('bientôt')])");
    let soon = soon.as_array().unwrap();
    assert!(soon.len() >= 4, "the pieces still to come, in place: {soon:?}");
    assert!(soon.iter().all(|s| s[1] == json!(true) && s[2] == json!(true)), "each disabled and saying « bientôt »: {soon:?}");
    drop(n);
    stop(b);
}

/// **A graph node opens the graph it cites** (the editor's second lot, 28 November 2026): `/ide/graphe` with
/// `node=<a path of graph nodes>` serves the cited graph as a tool's — its nodes with their schemas under their
/// configuration, its edges on declared ports, its `%% param` as `inputSchema`, and the values the citing node passes;
/// through two levels (`archive_slice` → `core` → `decide`). A node that is not a graph node, or that does not exist, is
/// refused by its name.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_graph_node_opens_the_graph_it_cites() {
    let b = bench();
    let (status, g) = get(&b.root, &format!("/ide/graphe?backend={}&tool=archive_slice&node=core/decide", b.id));
    assert_eq!(status, 200, "{g}");
    let r = &g["result"];
    assert_eq!(r["kind"], json!("subgraph"), "{g}");
    assert!(r["name"].as_str().unwrap_or_default().ends_with("archive_decide.mmd"), "the cited graph: {g}");
    assert_eq!(r["path"], json!(["core", "decide"]), "{g}");
    let nodes = r["nodes"].as_array().unwrap();
    assert!(!nodes.is_empty() && nodes.iter().all(|n| n["schema"].is_object() && n["line"].is_u64()), "{g}");
    let node = |name: &str| nodes.iter().find(|n| n["name"] == name).unwrap_or_else(|| panic!("no node {name}: {g}"));
    for e in r["edges"].as_array().unwrap() {
        let (from, fp, to, tp) = (e["from_node"].as_str().unwrap(), e["from_port"].as_str().unwrap(), e["to_node"].as_str().unwrap(), e["to_port"].as_str().unwrap());
        assert!(ports(node(from), "outputs").iter().any(|p| p == fp) && ports(node(to), "inputs").iter().any(|p| p == tp), "{from}.{fp} → {to}.{tp}");
    }
    let params = r["inputSchema"]["properties"].as_object().unwrap();
    assert!(params.contains_key("candidates") && params.contains_key("instruction"), "its %% param: {g}");
    assert_eq!(r["passed"]["candidates"], json!("$candidates"), "what the citing node passes: {g}");

    let (_, refused) = get(&b.root, &format!("/ide/graphe?backend={}&tool=archive_slice&node=slice", b.id));
    assert!(refused["error"].as_str().unwrap_or_default().contains("« slice »"), "not a graph node, named: {refused}");
    let (_, refused) = get(&b.root, &format!("/ide/graphe?backend={}&tool=archive_slice&node=core/nowhere", b.id));
    assert!(refused["error"].as_str().unwrap_or_default().contains("« nowhere »"), "an unknown node, named: {refused}");
    stop(b);
}

/// **In the editor, a graph node opens its cited graph, and the crumbs lead back**: from `archive_slice`, the node `core`
/// opens `archive_slice_core.mmd` (its nodes, `decide` among them), then `decide` opens `archive_decide.mmd`; the crumbs
/// say the path, and the form says what the citing node passes.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
#[test]
fn the_editor_opens_a_graph_nodes_cited_graph() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binary) = navigateur_ou_saut("the_editor_opens_a_graph_nodes_cited_graph") else {
        return;
    };
    let b = bench();
    let mut n = Navigateur::lancer(&binary);
    n.aller(&format!("{}/ide/entrer?jeton=j", b.root), "location.pathname.startsWith('/ide/')");
    n.aller(&format!("{}/ide/code/{}/workflow/tool/archive_slice", b.root, b.id), "!!document.querySelector('#workflow [data-node=\"core\"]')");
    n.cliquer("#workflow [data-node=\"core\"] .wf-node-head");
    n.attendre("!!document.querySelector('#workflow [data-open-subgraph=\"core\"]')", "the link that opens core's cited graph");
    n.cliquer("#workflow [data-open-subgraph=\"core\"]");
    n.attendre("document.querySelector('#workflow [data-crumbs]')?.dataset.crumbs === 'core' && !!document.querySelector('#workflow [data-node=\"decide\"]')", "core's cited graph, decide among its nodes");
    n.cliquer("#workflow [data-node=\"decide\"] .wf-node-head");
    n.attendre("!!document.querySelector('#workflow [data-open-subgraph=\"decide\"]')", "the link that opens decide's cited graph");
    n.cliquer("#workflow [data-open-subgraph=\"decide\"]");
    n.attendre("document.querySelector('#workflow [data-crumbs]')?.dataset.crumbs === 'core/decide'", "the crumbs of two levels");
    assert!(n.evaluer("document.querySelector('#workflow').innerText").as_str().unwrap_or_default().contains("archive_decide"), "the cited graph is named");
    assert!(n.evaluer("!!document.querySelector('#workflow [data-passed=\"candidates\"]')").as_bool().unwrap_or(false), "the form says what the citing node passes");
    drop(n);
    stop(b);
}

fn post(root: &str, path: &str, body: Value) -> Value {
    let mut r = crate::daemon::agent_de_temoin()
        .post(format!("{root}{path}"))
        .header("Cookie", "rag3weaver_ide=j")
        .header("content-type", "application/json")
        .send(body.to_string())
        .unwrap();
    serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
}

/// **The editor's draft is applied to the `.mmd` line by line and checked as a load checks it** (the third lot): no
/// edit gives the same text, loadable; an edge between compatible ports adds one line and loads; an edge between two
/// declared types that do not connect is refused by the check, naming both ports; nothing is written to the disk.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn the_editors_draft_is_applied_line_by_line_and_checked() {
    let b = bench();
    let edit = |edits: Value| post(&b.root, "/ide/graphe/edit", json!({ "backend": b.id, "tool": "archive", "edits": edits }));
    let same = edit(json!([]));
    assert_eq!(same["ok"], json!(true), "{same}");
    let r = &same["result"];
    assert_eq!(r["before"], r["after"], "no edit, the same text");
    assert_eq!(r["valid"], json!(true), "{same}");

    // A pair of ports the served schemas declare: one that connects (same type or Any), one that does not.
    let nodes = r["nodes"].as_array().unwrap().clone();
    let typed = |side: &str| -> Vec<(String, String, String)> {
        nodes.iter().flat_map(|n| n["schema"][side].as_array().cloned().unwrap_or_default().into_iter().map(move |p| (n["name"].as_str().unwrap().to_string(), p["name"].as_str().unwrap().to_string(), p["type"].as_str().unwrap().to_string()))).collect()
    };
    let (outs, ins) = (typed("outputs"), typed("inputs"));
    let joins = |o: &(String, String, String), i: &(String, String, String)| o.2 == i.2 || o.2 == "Any" || i.2 == "Any";
    let wired = |o: &(String, String, String), i: &(String, String, String)| r["edges"].as_array().unwrap().iter().any(|e| e["from_node"] == o.0.as_str() && e["from_port"] == o.1.as_str() && e["to_node"] == i.0.as_str() && e["to_port"] == i.1.as_str());
    let pick = |ok: bool| outs.iter().flat_map(|o| ins.iter().map(move |i| (o, i))).find(|(o, i)| o.0 != i.0 && joins(o, i) == ok && !wired(o, i)).map(|(o, i)| (o.clone(), i.clone()));
    let connect = |(o, i): &((String, String, String), (String, String, String))| json!([{ "op": "connect", "from_node": o.0, "from_port": o.1, "to_node": i.0, "to_port": i.1 }]);

    let good = pick(true).expect("two ports that connect");
    let added = edit(connect(&good));
    assert_eq!(added["result"]["valid"], json!(true), "{added}");
    let (before, after) = (added["result"]["before"].as_str().unwrap(), added["result"]["after"].as_str().unwrap());
    assert_eq!(after.lines().count(), before.lines().count() + 1, "one line more");
    // Two declared ports whose types do not connect: looked for in every tool of the backend (archive's are all Map or
    // Any; a search's carry Results and Query).
    let (_, state) = get(&b.root, "/ide/etat");
    let manifest = state["backends"].as_array().unwrap().iter().find(|x| x["id"] == b.id.as_str()).unwrap()["manifest"].clone();
    let described = post(&b.root, "/ide/describe", json!({ "manifest": manifest }));
    let tools: Vec<String> = described["result"]["tools"].as_array().or(described["tools"].as_array()).unwrap().iter().filter_map(|t| t["name"].as_str().map(str::to_string)).collect();
    let found = tools.iter().find_map(|tool| {
        let (_, g) = get(&b.root, &format!("/ide/graphe?backend={}&tool={tool}", b.id));
        let nodes = g["result"]["nodes"].as_array()?.clone();
        let edges = g["result"]["edges"].as_array()?.clone();
        let typed = |side: &str| -> Vec<(String, String, String)> {
            nodes.iter().flat_map(|n| n["schema"][side].as_array().cloned().unwrap_or_default().into_iter().map(move |p| (n["name"].as_str().unwrap().to_string(), p["name"].as_str().unwrap().to_string(), p["type"].as_str().unwrap().to_string()))).collect()
        };
        let (outs, ins) = (typed("outputs"), typed("inputs"));
        outs.iter()
            .flat_map(|o| ins.iter().map(move |i| (o.clone(), i.clone())))
            .find(|(o, i)| o.0 != i.0 && !joins(o, i) && !edges.iter().any(|e| e["from_node"] == o.0.as_str() && e["from_port"] == o.1.as_str() && e["to_node"] == i.0.as_str() && e["to_port"] == i.1.as_str()))
            .map(|pair| (tool.clone(), pair))
    });
    let (tool, bad) = found.unwrap_or_else(|| panic!("two declared ports whose types do not connect, in one of {tools:?}"));
    let refused = post(&b.root, "/ide/graphe/edit", json!({ "backend": b.id, "tool": tool, "edits": connect(&bad) }));
    assert_eq!(refused["result"]["valid"], json!(false), "{refused}");
    let why = refused["result"]["error"].as_str().unwrap_or_default();
    assert!(why.contains(&format!("{}.{}", bad.0 .0, bad.0 .1)) && why.contains(&format!("{}.{}", bad.1 .0, bad.1 .1)), "the refusal names both ports: {why}");

    let file = std::path::Path::new(b._d.path()).join("memory");
    let on_disk: Vec<String> = walkdir_mmd(&file);
    assert!(on_disk.iter().all(|t| !t.contains(&format!("{} -->|{}:{}| {}", good.0 .0, good.0 .1, good.1 .1, good.1 .0))), "nothing written");
    stop(b);
}

fn walkdir_mmd(dir: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walkdir_mmd(&p));
            } else if p.extension().is_some_and(|x| x == "mmd") {
                out.push(std::fs::read_to_string(&p).unwrap_or_default());
            }
        }
    }
    out
}

/// **A port is a target, and the draft says its diff** (the third lot): in the Edit view, an output then a lit input
/// make an edge; the panel « La modification » shows the `.mmd`'s new line and that the graph loads; « Annuler les
/// changements » empties the draft. Nothing is written.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
#[test]
fn a_port_is_a_target_and_the_draft_shows_its_diff() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binary) = navigateur_ou_saut("a_port_is_a_target_and_the_draft_shows_its_diff") else {
        return;
    };
    let b = bench();
    let mut n = Navigateur::lancer(&binary);
    n.aller(&format!("{}/ide/entrer?jeton=j", b.root), "location.pathname.startsWith('/ide/')");
    n.aller(&format!("{}/ide/code/{}/workflow/tool/archive_slice", b.root, b.id), "!!document.querySelector('#workflow [data-node=\"render\"]')");
    n.cliquer("#workflow [data-view=edit]");
    n.attendre("!!document.querySelector('#workflow [data-port=\"render:out:result\"]')", "render's output");
    n.cliquer("#workflow [data-port=\"render:out:result\"]");
    n.attendre("document.querySelector('#workflow [data-port=\"done:in:value\"]')?.dataset.lit === 'true'", "done's input lit");
    n.cliquer("#workflow [data-port=\"done:in:value\"]");
    n.attendre("(document.querySelector('#workflow [data-diff]')?.innerText ?? '').includes('render -->|result:value| done')", "the new edge in the diff");
    assert_eq!(n.evaluer("document.querySelector('#workflow [data-draft-valid]')?.dataset.draftValid"), json!("true"), "the edited graph loads");
    n.cliquer("#workflow [data-action=reset]");
    n.attendre("!document.querySelector('#workflow [data-diff]')", "the draft emptied");
    drop(n);
    stop(b);
}
