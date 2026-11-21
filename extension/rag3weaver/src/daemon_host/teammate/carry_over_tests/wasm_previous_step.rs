//! The witness of the lot `wasm-node-trial` (21 November 2026): the « Previous step » block as a WebAssembly node.
use super::*;

/// The WebAssembly node, copied with its declaration into a folder of its own: the tests edit their copy.
fn wasm_nodes() -> (tempfile::TempDir, PathBuf) {
    let d = tempfile::tempdir().unwrap();
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wasm_nodes");
    let dir = d.path().join("nodes");
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["previous_step_wasm.node.json", "previous_step.wasm"] {
        std::fs::copy(src.join(f), dir.join(f)).unwrap();
    }
    (d, dir)
}

/// **The WebAssembly node gives the Rust block, to the byte**, on the seven states of the scripted node's witness, the
/// cut included (accents, characters outside the BMP); **a broken module never breaks the step**: the Rust block
/// serves, and the error names the module's file.
#[test]
fn the_wasm_previous_step_is_the_rust_one_and_a_broken_module_falls_back() {
    let (d, dir) = wasm_nodes();
    for (i, (w, text, budget)) in works().into_iter().enumerate() {
        let rust = w.block(3, &text, budget);
        let wasm = carry_over_by_node(&dir, "PreviousStepWasm", &Value::Null, w.to_json(3, &text, budget))
            .unwrap_or_else(|e| panic!("state {i}: the WebAssembly node failed: {e}"));
        assert!(wasm == rust, "state {i}: the WebAssembly block differs from the Rust one ({} and {} characters)", wasm.chars().count(), rust.chars().count());
    }
    std::fs::write(dir.join("previous_step.wasm"), b"\0asm not a module").unwrap();
    let step = Step::new(TeammateFiles::new(d.path().join("box"), "t"), "t", "r", None, 3);
    step.work.lock().unwrap().read.push("src/a.rs (lines 1-3)".into());
    let (block, error) = step.carry_over_with("done", 8_000, Some((&dir, "PreviousStepWasm", &Value::Null)));
    assert_eq!(block, step.carry_over("done", 8_000), "the Rust block serves");
    let e = error.expect("the error is said");
    assert!(e.contains("previous_step.wasm"), "the error names the module's file: {e}");
}

/// **The cost of the WebAssembly block, against the Rust one and the TypeScript one** (a measure, not a witness:
/// `--ignored`, in release). The state of the scripted node's measure (200 reads, 50 writes, a command and its 1 500
/// last characters, a last text of 3 000 characters, over the 8 000 budget): the Rust block; the node as a step calls
/// it (the declaration read and the module prepared again); the module prepared once.
#[test]
#[ignore]
fn measure_the_wasm_previous_step_against_the_rust_one() {
    let (_d, dir) = wasm_nodes();
    let w = StepWork {
        read: (0..200).map(|i| format!("src/module_{i}/fichier_é.rs (lines {i}-{})", i + 40)).collect(),
        wrote: (0..50).map(|i| format!("src/écrit_{i}.rs")).collect(),
        command: Some(("cargo test --lib".into(), "test ok 😀\n".repeat(150))),
    };
    let text = "Je relis le module, puis j'édite la fonction. ".repeat(65);
    let n = 200;
    let median = |mut v: Vec<std::time::Duration>| {
        v.sort();
        v[v.len() / 2]
    };
    let rust: Vec<_> = (0..n).map(|_| { let t = std::time::Instant::now(); std::hint::black_box(w.block(7, &text, 8_000)); t.elapsed() }).collect();
    let node: Vec<_> = (0..n)
        .map(|_| {
            let t = std::time::Instant::now();
            std::hint::black_box(carry_over_by_node(&dir, "PreviousStepWasm", &Value::Null, w.to_json(7, &text, 8_000)).unwrap());
            t.elapsed()
        })
        .collect();
    use crate::dataflow::node_registry::NodeFactory;
    let decl = crate::dataflow::scripted_node::read_declaration(&dir.join("previous_step_wasm.node.json")).unwrap();
    let factory = crate::dataflow::scripted_node::ScriptedNodeFactory::new(decl, crate::script::ScriptLimits::default()).unwrap();
    let prepared: Vec<_> = (0..n)
        .map(|_| {
            let mut node = factory.create("m", &Value::Null).unwrap();
            let mut ctx = crate::dataflow::node::NodeContext::new();
            ctx.set_input("work", crate::dataflow::port::PortValue::new(w.to_json(7, &text, 8_000)));
            let t = std::time::Instant::now();
            node.execute(&mut ctx).unwrap();
            t.elapsed()
        })
        .collect();
    assert_eq!(carry_over_by_node(&dir, "PreviousStepWasm", &Value::Null, w.to_json(7, &text, 8_000)).unwrap(), w.block(7, &text, 8_000));
    eprintln!(
        "[measure] Previous step, medians of {n} calls: Rust {:?}; WebAssembly node as a step calls it (read and prepared) {:?}; module prepared once {:?}",
        median(rust),
        median(node),
        median(prepared)
    );
}
