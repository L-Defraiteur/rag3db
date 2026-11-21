//! The witness of the lot `scripted-error-names-file` (21 November 2026): an error of a scripted node names its script.
use super::*;

/// A declaration and its script, written in a folder of their own.
fn declared_node(dir: &std::path::Path, script: &str) -> std::path::PathBuf {
    std::fs::write(dir.join("broken.ts"), script).unwrap();
    let decl = dir.join("broken.node.json");
    std::fs::write(
        &decl,
        json!({"name": "Broken", "script": "broken.ts", "inputs": {"x": {"schema": {"type": "number"}, "required": true}}, "outputs": {"y": {"schema": {"type": "number"}}}}).to_string(),
    )
    .unwrap();
    decl
}

/// **An error of a scripted node names its script's file, and its line** (Lucie, 20 November 2026: « valider le
/// scripting […] et qu'il est pratique là où il est placé » ; the trial of the scripted node: the engine gave the line,
/// not the file, and its caller had to add it by hand). At the call, a script that throws; at the preparation, a script
/// that does not parse: both errors name `broken.ts`.
#[test]
fn an_error_of_a_scripted_node_names_its_script_file_and_line() {
    let d = tempfile::tempdir().unwrap();
    let decl = declared_node(d.path(), "function run({ inputs }) {\n  const x = inputs.x;\n  throw new Error(\"broken on purpose\");\n}\n");
    let fabrique = ScriptedNodeFactory::new(read_declaration(&decl).unwrap(), ScriptLimits::default()).unwrap();
    let mut n = fabrique.create("broken", &serde_json::Value::Null).unwrap();
    let mut ctx = NodeContext::new();
    ctx.set_input("x", PortValue::new(json!(1)));
    let e = n.execute(&mut ctx).err().expect("the script throws");
    assert!(e.contains("broken.ts") && e.contains("broken on purpose") && e.contains("line 3"), "the call's error names the file and the line: {e}");
    let decl = declared_node(d.path(), "function run({ inputs }) {\n  return { y: (inputs.x + };\n}\n");
    let e = ScriptedNodeFactory::new(read_declaration(&decl).unwrap(), ScriptLimits::default()).err().expect("the script does not parse");
    assert!(e.contains("broken.ts") && e.contains("line 2"), "the preparation's error names the file and the line: {e}");
}
