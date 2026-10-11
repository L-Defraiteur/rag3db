//! Les témoins du nœud entièrement scripté (lot 2 du proto « tout
//! déclaratif »).
use super::*;
use crate::dataflow::checkpoint::{EdgeDef, GraphDefinition, NodeDef};
use crate::dataflow::graph::DataflowGraph;
use crate::dataflow::node_factories::register_builtins;
use crate::dataflow::report::NodeStatus;
use crate::dataflow::runtime::DataflowRuntime;
use crate::dataflow::NodeRegistry;
use serde_json::json;

const TS: &str = "interface Product { name: string; price: number }
function run({ inputs, config }: { inputs: { product: Product }; config: { rate: number } }) {
  const p = inputs.product;
  return { priced: { name: p.name, price: p.price, with_tax: p.price * (1 + config.rate) } };
}";

const RHAI: &str = "let p = input.inputs.product;
#{priced: #{name: p.name, price: p.price, with_tax: p.price * (1.0 + input.config.rate)}}";

fn decl(language: &str, source: &str) -> ScriptedNodeDecl {
    ScriptedNodeDecl {
        name: "PriceWithTax".into(),
        description: "adds the tax to a product's price".into(),
        language: language.into(),
        source: source.into(),
        inputs: vec![DeclaredPort {
            name: "product".into(),
            schema: json!({
                "type": "object",
                "properties": {"name": {"type": "string"}, "price": {"type": "number"}},
                "required": ["name", "price"]
            }),
            required: true,
        }],
        outputs: vec![DeclaredPort {
            name: "priced".into(),
            schema: json!({
                "type": "object",
                "properties": {"with_tax": {"type": "number"}},
                "required": ["with_tax"]
            }),
            required: true,
        }],
        config: vec![DeclaredParam {
            name: "rate".into(),
            schema: json!({"type": "number", "minimum": 0}),
            required: false,
            default: Some(json!(0.25)),
            description: "the tax rate".into(),
        }],
    }
}

fn registry_with(decl: ScriptedNodeDecl) -> NodeRegistry {
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    registry.register(Box::new(
        ScriptedNodeFactory::new(decl, ScriptLimits::default()).unwrap(),
    ));
    registry
}

/// Une source (un nœud rhai fourni qui rend sa valeur) branchée sur le port
/// `product` du nœud déclaré.
fn graph(product: Value, config: Value) -> GraphDefinition {
    GraphDefinition {
        nodes: vec![
            NodeDef {
                name: "source".into(),
                node_type: "RhaiNode".into(),
                config: json!({"script": "input", "value": product}),
            },
            NodeDef {
                name: "price".into(),
                node_type: "PriceWithTax".into(),
                config,
            },
        ],
        edges: vec![EdgeDef {
            from_node: "source".into(),
            from_port: "result".into(),
            to_node: "price".into(),
            to_port: "product".into(),
        }],
    }
}

fn run(
    registry: &NodeRegistry,
    definition: &GraphDefinition,
) -> Result<(Value, crate::dataflow::ExecutionReport), String> {
    let mut graph = DataflowGraph::from_definition(definition, registry)?;
    let (output, report) = DataflowRuntime::new(100).execute_with_report(&mut graph)?;
    let priced = output
        .get("price", "priced")
        .and_then(|v| v.downcast::<Value>())
        .cloned()
        .ok_or("no output on price.priced")?;
    Ok((priced, report))
}

#[test]
fn a_declared_node_runs_in_a_graph_and_is_recorded_like_the_others() {
    let registry = registry_with(decl("typescript", TS));
    let (priced, report) = run(
        &registry,
        &graph(json!({"name": "chair", "price": 100}), json!({})),
    )
    .unwrap();
    assert_eq!(
        priced,
        json!({"name": "chair", "price": 100, "with_tax": 125})
    );
    let node = report
        .nodes
        .iter()
        .find(|n| n.name == "price")
        .expect("the scripted node is in the report");
    assert!(
        matches!(node.status, NodeStatus::Completed),
        "{:?}",
        node.status
    );
    assert!(
        !node.inputs.is_empty() && !node.outputs.is_empty(),
        "{node:?}"
    );
}

#[test]
fn its_inputs_come_from_ports_and_its_schema_is_the_declared_one() {
    let registry = registry_with(decl("typescript", TS));
    let schema = registry.schema("PriceWithTax").unwrap();
    assert_eq!(schema.node_type, "PriceWithTax");
    assert_eq!(schema.inputs.len(), 1);
    assert_eq!(schema.inputs[0].name, "product");
    assert_eq!(schema.outputs[0].name, "priced");
    assert_eq!(schema.config_params[0].name, "rate");
    assert_eq!(
        schema.config_params[0].json_schema,
        Some(json!({"type": "number", "minimum": 0}))
    );
    let node = registry.create("PriceWithTax", "p", &json!({})).unwrap();
    assert_eq!(node.inputs()[0].name, "product");
}

#[test]
fn the_same_node_serves_two_graphs() {
    let registry = registry_with(decl("typescript", TS));
    let (a, _) = run(
        &registry,
        &graph(json!({"name": "a", "price": 10}), json!({})),
    )
    .unwrap();
    let (b, _) = run(
        &registry,
        &graph(json!({"name": "b", "price": 10}), json!({"rate": 0.5})),
    )
    .unwrap();
    assert_eq!(a["with_tax"], json!(12.5));
    assert_eq!(b["with_tax"], json!(15));
}

#[test]
fn the_same_node_in_rhai_and_typescript_gives_the_same_outputs() {
    let product = json!({"name": "lamp", "price": 40});
    let (ts, _) = run(
        &registry_with(decl("typescript", TS)),
        &graph(product.clone(), json!({})),
    )
    .unwrap();
    let (rhai, _) = run(
        &registry_with(decl("rhai", RHAI)),
        &graph(product, json!({})),
    )
    .unwrap();
    assert_eq!(ts, rhai);
    // Une seule forme pour un nombre, quel que soit le langage.
    assert_eq!(rhai["with_tax"], json!(50));
    assert!(rhai["with_tax"].is_i64());
}

#[test]
fn a_bad_declaration_is_refused_by_name() {
    let refused =
        |decl: ScriptedNodeDecl| match ScriptedNodeFactory::new(decl, ScriptLimits::default()) {
            Ok(_) => panic!("accepted"),
            Err(e) => e,
        };
    let mut d = decl("typescript", TS);
    d.outputs.clear();
    assert!(refused(d).contains("no output port"));

    let mut d = decl("typescript", TS);
    d.name = "price with tax".into();
    assert!(refused(d).contains("identifier"));

    let mut d = decl("typescript", TS);
    d.inputs.push(d.inputs[0].clone());
    assert!(refused(d).contains("input port 'product'"));

    let mut d = decl("typescript", TS);
    d.outputs[0].schema = json!({"type": 12});
    assert!(refused(d).contains("output port 'priced'"));

    let mut d = decl("typescript", TS);
    d.outputs[0].schema = json!("an object");
    assert!(refused(d).contains("output port 'priced'"));

    let mut d = decl("typescript", TS);
    d.config[0].default = Some(json!("a lot"));
    assert!(refused(d).contains("config parameter 'rate'"));

    let mut d = decl("typescript", "function run(x) { return x +; }");
    d.name = "Broken".into();
    let e = refused(d);
    assert!(
        e.contains("Broken") && e.contains("script") && e.contains("line 1"),
        "{e}"
    );

    let e = refused(decl("python", "x"));
    assert!(e.contains("python"), "{e}");
}

#[test]
fn a_value_that_does_not_match_its_port_is_refused_at_the_frontier() {
    let registry = registry_with(decl("typescript", TS));
    let e = run(&registry, &graph(json!({"name": "chair"}), json!({}))).unwrap_err();
    assert!(
        e.contains("input port 'product'") && e.contains("price"),
        "{e}"
    );

    let wrong_output = "function run({ inputs }) { return { priced: { with_tax: 'free' } }; }";
    let registry = registry_with(decl("typescript", wrong_output));
    let e = run(
        &registry,
        &graph(json!({"name": "a", "price": 1}), json!({})),
    )
    .unwrap_err();
    assert!(
        e.contains("output port 'priced'") && e.contains("with_tax"),
        "{e}"
    );

    let unknown = "function run({ inputs }) { return { priced: { with_tax: 1 }, extra: 1 }; }";
    let registry = registry_with(decl("typescript", unknown));
    let e = run(
        &registry,
        &graph(json!({"name": "a", "price": 1}), json!({})),
    )
    .unwrap_err();
    assert!(e.contains("'extra'"), "{e}");

    let missing = "function run({ inputs }) { return {}; }";
    let registry = registry_with(decl("typescript", missing));
    let e = run(
        &registry,
        &graph(json!({"name": "a", "price": 1}), json!({})),
    )
    .unwrap_err();
    assert!(e.contains("output port 'priced'"), "{e}");
}

#[test]
fn its_configuration_is_checked_when_the_node_is_created() {
    let registry = registry_with(decl("typescript", TS));
    let e = registry
        .create("PriceWithTax", "p", &json!({"rate": -1}))
        .err()
        .unwrap();
    assert!(e.contains("config parameter 'rate'"), "{e}");
    let e = registry
        .create("PriceWithTax", "p", &json!({"rat": 0.1}))
        .err()
        .unwrap();
    assert!(e.contains("unknown config parameter 'rat'"), "{e}");
}

// ─── La forme lue ───────────────────────────────────────────────────────────

const FILE: &str = r#"{
  "name": "PriceWithTax",
  "description": "adds the tax to a product's price",
  "script": "price_with_tax.ts",
  "inputs": {
    "product": { "schema": { "type": "object", "required": ["name", "price"] } }
  },
  "outputs": {
    "priced": { "schema": { "type": "object", "required": ["with_tax"] } }
  },
  "config": {
    "rate": { "schema": { "type": "number" }, "default": 0.25, "description": "the tax rate" }
  }
}"#;

fn folder(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (name, text) in files {
        std::fs::write(dir.path().join(name), text).unwrap();
    }
    dir
}

#[test]
fn a_declaration_folder_is_read_and_its_nodes_run() {
    let dir = folder(&[
        ("price_with_tax.node.json", FILE),
        ("price_with_tax.ts", TS),
    ]);
    let factories = discover(dir.path(), &ScriptLimits::default()).unwrap();
    assert_eq!(factories.len(), 1);
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    for factory in factories {
        registry.register(Box::new(factory));
    }
    let (priced, _) = run(
        &registry,
        &graph(json!({"name": "a", "price": 4}), json!({})),
    )
    .unwrap();
    assert_eq!(priced["with_tax"], json!(5));
}

#[test]
fn a_missing_folder_declares_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let factories = discover(&dir.path().join("nodes"), &ScriptLimits::default()).unwrap();
    assert!(factories.is_empty());
}

#[test]
fn a_bad_declaration_file_is_refused_by_name() {
    let escape = FILE.replace("price_with_tax.ts", "../price_with_tax.ts");
    let absolute = FILE.replace("price_with_tax.ts", "/etc/passwd");
    let unknown_key = FILE.replace("\"description\"", "\"descripton\"");
    let no_language = FILE.replace("price_with_tax.ts", "price_with_tax.py");
    for (text, expected) in [
        (escape.as_str(), "inside the declaration's folder"),
        (absolute.as_str(), "inside the declaration's folder"),
        (unknown_key.as_str(), "descripton"),
        (no_language.as_str(), "no language declared"),
    ] {
        let dir = folder(&[
            ("n.node.json", text),
            ("price_with_tax.ts", TS),
            ("price_with_tax.py", ""),
        ]);
        let errors = discover(dir.path(), &ScriptLimits::default())
            .err()
            .expect("refused");
        assert!(
            errors
                .iter()
                .any(|e| e.contains(expected) && e.contains("n.node.json")),
            "{errors:?}"
        );
    }
}

#[test]
fn a_type_declared_twice_is_refused() {
    let dir = folder(&[
        ("a.node.json", FILE),
        ("b.node.json", FILE),
        ("price_with_tax.ts", TS),
    ]);
    let errors = discover(dir.path(), &ScriptLimits::default())
        .err()
        .expect("refused");
    assert!(
        errors.iter().any(|e| e.contains("declared twice")),
        "{errors:?}"
    );
}

/// Un point de reprise réécrit le graphe depuis ses nœuds (`to_definition`) :
/// le type déclaré doit y survivre, sinon la restauration ne le retrouve pas.
#[test]
fn a_scripted_node_keeps_its_declared_type_through_a_definition_round_trip() {
    let registry = registry_with(decl("typescript", TS));
    let first = DataflowGraph::from_definition(
        &graph(json!({"name": "a", "price": 10}), json!({"rate": 0.5})),
        &registry,
    )
    .unwrap();
    let definition = first.to_definition();
    let price = definition.nodes.iter().find(|n| n.name == "price").unwrap();
    assert_eq!(price.node_type, "PriceWithTax");
    assert_eq!(price.config, json!({"rate": 0.5}));
    let (priced, _) = run(&registry, &definition).unwrap();
    assert_eq!(priced["with_tax"], json!(15));
    let text = crate::dataflow::mermaid::to_mermaid(&definition);
    let parsed = crate::dataflow::mermaid::parse_mermaid(&text).unwrap();
    let price = parsed.nodes.iter().find(|n| n.name == "price").unwrap();
    assert_eq!(price.node_type, "PriceWithTax", "{text}");
}

#[test]
fn a_config_parameter_takes_the_type_of_its_schema() {
    let mut d = decl("typescript", TS);
    for (name, schema) in [
        ("label", json!({"type": "string"})),
        ("count", json!({"type": "integer"})),
        ("ratio", json!({"type": "number"})),
        ("strict", json!({"type": "boolean"})),
        ("extra", json!({"type": "object"})),
    ] {
        d.config.push(DeclaredParam {
            name: name.into(),
            schema,
            required: false,
            default: None,
            description: String::new(),
        });
    }
    let schema = ScriptedNodeFactory::new(d, ScriptLimits::default()).unwrap().schema();
    let types: Vec<_> = schema
        .config_params
        .iter()
        .map(|p| format!("{}:{:?}", p.name, p.param_type))
        .collect();
    assert_eq!(
        types,
        ["rate:Float", "label:String", "count:Int", "ratio:Float", "strict:Bool", "extra:Json"]
    );
}
