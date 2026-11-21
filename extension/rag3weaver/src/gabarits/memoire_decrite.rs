//! **Every parameter of the memory server says what it is** (Lucie's rule of 20 November 2026). A file of its own
//! (c7's rule, 21 November 2026): a new witness is never written in the body of a shared test module.

use super::jouable::manifeste_jouable;
use serde_json::Value;

/// The properties of a schema without a description, by their path (nested objects and list items included).
fn undescribed(schema: &Value, path: &str, out: &mut Vec<String>) {
    if let Some(props) = schema.get("properties").and_then(Value::as_object) {
        for (k, v) in props {
            let here = format!("{path}.{k}");
            if !v.get("description").and_then(Value::as_str).is_some_and(|d| !d.trim().is_empty()) {
                out.push(here.clone());
            }
            undescribed(v, &here, out);
        }
    }
    if let Some(items) = schema.get("items") {
        undescribed(items, &format!("{path}[]"), out);
    }
    for key in ["anyOf", "oneOf", "allOf"] {
        for (i, s) in schema.get(key).and_then(Value::as_array).into_iter().flatten().enumerate() {
            undescribed(s, &format!("{path}<{key}{i}>"), out);
        }
    }
}

/// **Every parameter of the memory server says what it is** (Lucie's rule of 20 November 2026: « on met pas
/// encore de description à nos paramètres ? »). The memory template, loaded as the MCP server serves it
/// (`describe`, the `inputSchema` of each tool): a property without a description, at any depth, is red, named
/// by its tool and its path.
#[test]
fn every_parameter_of_the_memory_server_is_described() {
    let gabarit = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/memory");
    let ou = std::env::temp_dir().join(format!("rag3weaver-memoire-decrite-{}", std::process::id()));
    std::fs::create_dir_all(&ou).unwrap();
    let manifeste = manifeste_jouable(&gabarit, &ou).unwrap();
    let backend = crate::backend::PreparedBackend::load(&manifeste).unwrap_or_else(|e| panic!("memory loads: {e}"));
    let described = backend.describe();
    let tools = described["tools"].as_array().cloned().unwrap_or_default();
    assert!(tools.len() >= 20, "the memory server exposes its tools: {}", tools.len());
    let mut missing = Vec::new();
    for t in &tools {
        undescribed(&t["inputSchema"], t["name"].as_str().unwrap_or("?"), &mut missing);
    }
    let _ = std::fs::remove_dir_all(&ou);
    assert!(missing.is_empty(), "{} parameter(s) of the memory server without a description:\n{}", missing.len(), missing.join("\n"));
}
