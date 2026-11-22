//! **A cited graph cites the neighbours of its own template** (22 November 2026: Lucie's long memory stopped loading).
//! A local manifest's graph cites a shipped graph (`@builtin:backends/memory/graphs/…`), and that graph cites its
//! sibling `@graphs/…`: the sibling is the shipped one, next to the graph that cites it, not a file next to the local
//! manifest. A file of its own (c7's rule).

use super::*;

const TOP: &str = "%% tool: archive\n%% description: d\n%% result: chaque.report\n\ngraph LR\n    chaque[\"ForEachNode(graph='@builtin:backends/memory/graphs/decide.mmd')\"]\n";
const DECIDE: &str = "%% tool: decide\n%% description: d\n%% param: element json! -- x\n%% result: inner.result\n\ngraph LR\n    inner[\"GraphNode(graph='@graphs/section.mmd', element=$element)\"]\n";

/// The shipped graph's `@graphs/section.mmd` reads the shipped `section.mmd`; the local manifest's folder has none.
#[test]
fn a_shipped_graph_cites_its_siblings_in_the_shipped_template() {
    let reg = registre();
    let read = |path: &str| -> Result<String, String> {
        match path {
            "builtin:backends/memory/graphs/decide.mmd" => Ok(DECIDE.to_string()),
            "builtin:backends/memory/graphs/section.mmd" => Ok(SECTION.to_string()),
            other => Err(format!("No such file or directory: {other}")),
        }
    };
    let mut tool = GraphTool::from_mermaid(TOP).unwrap();
    let types = resolve_cited_graphs_from(&mut tool, "graphs/archive.mmd", &read, &reg).unwrap_or_else(|e| panic!("the sibling is read in the shipped template: {e}"));
    assert!(types.iter().any(|(t, _)| t == "NoterNode"), "the sibling's nodes are there: {types:?}");
}

/// A local graph keeps citing its siblings next to the manifest, as before.
#[test]
fn a_local_graph_cites_its_siblings_next_to_the_manifest() {
    let reg = registre();
    let read = |path: &str| -> Result<String, String> {
        match path {
            "graphs/section.mmd" => Ok(SECTION.to_string()),
            other => Err(format!("No such file or directory: {other}")),
        }
    };
    let mut tool = GraphTool::from_mermaid("%% tool: archive\n%% description: d\n%% result: chaque.report\n\ngraph LR\n    chaque[\"ForEachNode(graph='@graphs/section.mmd')\"]\n").unwrap();
    resolve_cited_graphs_from(&mut tool, "graphs/archive.mmd", &read, &reg).expect("the local sibling");
}
