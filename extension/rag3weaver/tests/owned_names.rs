//! Un nom déclaré (type de nœud, port, paramètre) appartient à ce qui le
//! déclare : reconstruire une déclaration mille fois — ce que fera le
//! rechargement à chaud — ne doit pas faire grandir la mémoire. Un allocateur
//! qui compte les octets vivants le mesure ; ce fichier est sa propre cible de
//! test, pour que rien d'autre n'alloue pendant la mesure.
use rag3weaver::dataflow::checkpoint::{EdgeDef, GraphDefinition, NodeDef};
use rag3weaver::dataflow::graph_node::GraphNodeFactory;
use rag3weaver::dataflow::node_factories::register_builtins;
use rag3weaver::dataflow::{NodeFactory, NodeRegistry};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Arc;

struct Counting;

static LIVE: AtomicIsize = AtomicIsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LIVE.fetch_add(layout.size() as isize, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size() as isize, Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        LIVE.fetch_add(
            new_size as isize - layout.size() as isize,
            Ordering::Relaxed,
        );
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn registry() -> Arc<NodeRegistry> {
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    Arc::new(registry)
}

/// Deux nœuds fournis, des ports libres des deux côtés.
fn definition() -> GraphDefinition {
    GraphDefinition {
        nodes: vec![
            NodeDef {
                name: "inserts".into(),
                node_type: "InsertRecordNode".into(),
                config: serde_json::json!({}),
            },
            NodeDef {
                name: "links".into(),
                node_type: "LinkRecordNode".into(),
                config: serde_json::json!({}),
            },
        ],
        edges: vec![EdgeDef {
            from_node: "inserts".into(),
            from_port: "done".into(),
            to_node: "links".into(),
            to_port: "trigger".into(),
        }],
    }
}

/// Les octets vivants gagnés par `rounds` passages de `step`, après dix
/// passages d'échauffement.
fn growth(rounds: usize, mut step: impl FnMut()) -> isize {
    for _ in 0..10 {
        step();
    }
    let before = LIVE.load(Ordering::Relaxed);
    for _ in 0..rounds {
        step();
    }
    LIVE.load(Ordering::Relaxed) - before
}

#[test]
fn a_thousand_rebuilds_of_a_named_subgraph_keep_memory_stable() {
    let registry = registry();
    let definition = definition();

    // Une déclaration rechargée : la fabrique est rebâtie, l'ancienne jetée.
    let rebuilt = growth(1000, || {
        let factory = GraphNodeFactory::templated(
            "DeclaredSubgraph",
            "a sub-graph declared under a name",
            definition.clone(),
            vec![],
            registry.clone(),
        )
        .unwrap();
        drop(factory.create("node", &serde_json::json!({})).unwrap());
    });

    // Une déclaration en service : un nœud par instanciation de graphe.
    let factory = GraphNodeFactory::templated(
        "DeclaredSubgraph",
        "a sub-graph declared under a name",
        definition.clone(),
        vec![],
        registry.clone(),
    )
    .unwrap();
    let created = growth(1000, || {
        drop(factory.create("node", &serde_json::json!({})).unwrap());
    });

    assert!(
        rebuilt < 4096 && created < 4096,
        "live bytes grew by {rebuilt} over 1000 rebuilds and by {created} over 1000 nodes created"
    );
}
