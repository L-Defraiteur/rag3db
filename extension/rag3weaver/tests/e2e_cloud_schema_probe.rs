//! Sonde : quelles variantes du jeu d'outils Vertex accepte-t-il ? Une
//! requête minuscule par variante. Sauté sans identifiants.
//!
//! Run with: ./run_e2e.sh --test e2e_cloud_schema_probe --features openai-llm
#![cfg(all(feature = "openai-llm", feature = "code"))]

use rag3weaver::dataflow::graph_tool::builtin_graph_tools;
use rag3weaver::llm::{GenOptions, Llm, StringSink, Turn};
use rag3weaver::openai_llm::OpenAiLlm;
use rag3weaver::tools::ToolDef;

fn vertex() -> Option<OpenAiLlm> {
    rag3weaver::regime::modele_agentique("schema-probe")
}

fn strip(v: &mut serde_json::Value, key: &str) {
    match v {
        serde_json::Value::Object(m) => {
            m.remove(key);
            for x in m.values_mut() {
                strip(x, key);
            }
        }
        serde_json::Value::Array(a) => a.iter_mut().for_each(|x| strip(x, key)),
        _ => {}
    }
}

fn set_defaults_nonempty(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(m) => {
            if let Some(d) = m.get_mut("default") {
                if d.as_str() == Some("") {
                    *d = serde_json::Value::String("-".into());
                }
            }
            for x in m.values_mut() {
                set_defaults_nonempty(x);
            }
        }
        serde_json::Value::Array(a) => a.iter_mut().for_each(set_defaults_nonempty),
        _ => {}
    }
}

#[test]
#[ignore]
fn which_tool_schemas_does_vertex_accept() {
    let Some(llm) = vertex() else { eprintln!("skipped"); return; };
    let (_, tools) = builtin_graph_tools().unwrap();
    let all: Vec<ToolDef> = rag3weaver::tools::graph_tool_defs(&tools);
    let only = |names: &[&str]| -> Vec<ToolDef> { all.iter().filter(|d| names.contains(&d.name.as_str())).cloned().collect() };
    let mapped = |defs: Vec<ToolDef>, f: &dyn Fn(&mut serde_json::Value)| -> Vec<ToolDef> {
        defs.into_iter().map(|mut d| { f(&mut d.parameters); d }).collect()
    };
    let variants: Vec<(&str, Vec<ToolDef>)> = vec![
        ("3 outils (grep read search)", only(&["grep", "read", "search"])),
        ("6 outils tels quels", all.clone()),
        ("list seul", only(&["list"])),
        ("edit seul", only(&["edit"])),
        ("6 outils, default \"\" → \"-\"", mapped(all.clone(), &set_defaults_nonempty)),
        ("6 outils sans default", mapped(all.clone(), &|v| strip(v, "default"))),
        ("6 outils sans additionalProperties", mapped(all.clone(), &|v| strip(v, "additionalProperties"))),
    ];
    let turns = vec![Turn::system("Answer in one word."), Turn::user("Say hi.")];
    for (label, defs) in variants {
        let opts = GenOptions::default().with_max_tokens(8).with_tools(defs);
        let mut sink = StringSink::default();
        match llm.generate(&turns, &opts, &mut sink) {
            Ok((finish, _)) => eprintln!("[probe] OK   {label} → {:?} {:?}", finish.reason, sink.text.trim()),
            Err(e) => eprintln!("[probe] FAIL {label} → {}", e.to_string().chars().take(160).collect::<String>().replace('\n', " ")),
        }
    }
}
