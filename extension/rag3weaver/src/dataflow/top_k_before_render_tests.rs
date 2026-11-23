//! **Nothing outside the top-k is shown** (Lucie, 23 November 2026: « si on a une limite demandée de top-k, elle cache
//! les morceaux qui ne rentrent pas dedans »). The search graph of `search_code` cuts at `limit` in `PaginateNode`,
//! before the nodes that resolve and render: every path that brings `results` to the render passes through it, or
//! comes from `ScanFilesNode`, which bounds itself by `limit` (its witness in `scan_nodes.rs`). A node placed later
//! between the fusion and the render, without passing through the cut, breaks this witness.

use std::collections::{HashMap, HashSet};

/// The `results` edges of a Mermaid graph (`a -->|results| b`, labels `results` or `results:<port>`), and the node
/// type of each name (`name["Type(…)"]`).
fn graph(text: &str) -> (Vec<(String, String)>, HashMap<String, String>) {
    let mut edges = Vec::new();
    let mut types = HashMap::new();
    for line in text.lines().map(str::trim) {
        if let Some((from, rest)) = line.split_once(" -->|") {
            if let Some((label, to)) = rest.split_once("| ") {
                if label == "results" || label.starts_with("results:") {
                    edges.push((from.trim().to_string(), to.trim().to_string()));
                }
            }
        } else if let Some((name, rest)) = line.split_once("[\"") {
            let ty: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            types.insert(name.trim().to_string(), ty);
        }
    }
    (edges, types)
}

#[test]
fn every_result_shown_by_search_code_has_passed_the_cut_at_limit() {
    let text = include_str!("../../templates/tools/search_workspace.mmd");
    let (edges, types) = graph(text);
    let named = |ty: &str| types.iter().find(|(_, t)| t.as_str() == ty).map(|(n, _)| n.clone());
    let render = named("RenderResultsNode").expect("the graph renders");
    let cut = named("PaginateNode").expect("the graph cuts at limit");
    let bounded: HashSet<String> = types.iter().filter(|(_, t)| t.as_str() == "ScanFilesNode").map(|(n, _)| n.clone()).collect();
    // Walk back from the render along the result edges; stop at the cut or at a node that bounds itself.
    let mut open = vec![render.clone()];
    let mut seen = HashSet::new();
    while let Some(node) = open.pop() {
        if !seen.insert(node.clone()) || node == cut || bounded.contains(&node) {
            continue;
        }
        let sources: Vec<&String> = edges.iter().filter(|(_, to)| *to == node).map(|(from, _)| from).collect();
        assert!(
            !sources.is_empty() || node == render,
            "« {node} » ({}) brings results to the render without passing through « {cut} » (PaginateNode): a hit outside the top-k would be shown",
            types.get(&node).map(String::as_str).unwrap_or("?")
        );
        open.extend(sources.into_iter().cloned());
    }
    assert!(seen.contains(&cut), "the render is reached from the cut at limit: {seen:?}");
}
