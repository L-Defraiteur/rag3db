//! Mermaid flowchart parser and exporter for dataflow graphs.
//!
//! Supports a subset of Mermaid syntax:
//! - `graph LR` or `graph TD` header
//! - Node declarations: `instance_name["NodeType(key='value', ...)"]`
//! - Edge declarations: `from -->|port| to` or `from -->|from_port:to_port| to`
//! - Comments: `%% ...`
//! - Template variables: `$var_name` in config values

use std::collections::HashMap;
use std::fmt;

use super::checkpoint::{EdgeDef, GraphDefinition, NodeDef};

// ─── Error ───────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum MermaidError {
    MissingHeader,
    InvalidNodeDecl { line: usize, detail: String },
    InvalidEdge { line: usize, detail: String },
    UnknownVariable { line: usize, var: String },
    UnparsableLine { line: usize, content: String },
}

impl fmt::Display for MermaidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHeader => write!(f, "missing 'graph LR' or 'graph TD' header"),
            Self::InvalidNodeDecl { line, detail } => {
                write!(f, "line {line}: invalid node declaration: {detail}")
            }
            Self::InvalidEdge { line, detail } => {
                write!(f, "line {line}: invalid edge: {detail}")
            }
            Self::UnknownVariable { line, var } => {
                write!(f, "line {line}: unknown variable ${var}")
            }
            Self::UnparsableLine { line, content } => {
                write!(f, "line {line}: cannot parse: {content}")
            }
        }
    }
}

// ─── Parser ──────────────────────────────────────────────────────────────────

/// Parse a Mermaid flowchart string into a GraphDefinition.
pub fn parse_mermaid(input: &str) -> Result<GraphDefinition, MermaidError> {
    parse_mermaid_template(input, &HashMap::new())
}

/// Parse a Mermaid flowchart with variable substitution.
///
/// Variables in config values (`$var_name`) are replaced by entries in `vars`.
pub fn parse_mermaid_template(
    input: &str,
    vars: &HashMap<String, String>,
) -> Result<GraphDefinition, MermaidError> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut header_found = false;

    for (line_idx, raw_line) in input.lines().enumerate() {
        let line_num = line_idx + 1;

        // Strip comments
        let line = match raw_line.find("%%") {
            Some(pos) => &raw_line[..pos],
            None => raw_line,
        };
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        // Header
        if !header_found {
            if line.starts_with("graph ") {
                header_found = true;
                continue;
            }
            return Err(MermaidError::MissingHeader);
        }

        // Try edge first (contains -->)
        if line.contains("-->") {
            let edge = parse_edge(line, line_num)?;
            edges.push(edge);
            continue;
        }

        // Try node declaration (contains [")
        if line.contains("[\"") {
            let node = parse_node(line, line_num, vars)?;
            nodes.push(node);
            continue;
        }

        return Err(MermaidError::UnparsableLine {
            line: line_num,
            content: line.to_string(),
        });
    }

    if !header_found {
        return Err(MermaidError::MissingHeader);
    }

    Ok(GraphDefinition { nodes, edges })
}

/// Parse `instance_name["NodeType(key='value', key=123)"]`
fn parse_node(
    line: &str,
    line_num: usize,
    vars: &HashMap<String, String>,
) -> Result<NodeDef, MermaidError> {
    let err = |detail: &str| MermaidError::InvalidNodeDecl {
        line: line_num,
        detail: detail.to_string(),
    };

    // Split at ["
    let bracket_pos = line.find("[\"").ok_or_else(|| err("missing [\""))?;
    let name = line[..bracket_pos].trim();
    if name.is_empty() {
        return Err(err("empty node name"));
    }

    // Find closing "]
    let close = line.rfind("\"]").ok_or_else(|| err("missing \"]"))?;
    let inner = &line[bracket_pos + 2..close];

    // Split NodeType(params) or just NodeType
    let (node_type, config) = if let Some(paren_pos) = inner.find('(') {
        let close_paren = inner.rfind(')').ok_or_else(|| err("missing closing )"))?;
        let ntype = inner[..paren_pos].trim();
        let params_str = &inner[paren_pos + 1..close_paren];
        let config = parse_config_params(params_str, line_num, vars)?;
        (ntype, config)
    } else {
        (inner.trim(), serde_json::json!({}))
    };

    if node_type.is_empty() {
        return Err(err("empty node type"));
    }

    Ok(NodeDef {
        name: name.to_string(),
        node_type: node_type.to_string(),
        config,
    })
}

/// Parse `key='value', key=123, key=true` into JSON object.
fn parse_config_params(
    params: &str,
    line_num: usize,
    vars: &HashMap<String, String>,
) -> Result<serde_json::Value, MermaidError> {
    let mut map = serde_json::Map::new();
    let params = params.trim();
    if params.is_empty() {
        return Ok(serde_json::Value::Object(map));
    }

    // Split by commas, respecting quoted strings
    let parts = split_params(params);

    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let eq_pos = part.find('=').ok_or_else(|| MermaidError::InvalidNodeDecl {
            line: line_num,
            detail: format!("missing '=' in param: {part}"),
        })?;

        let key = part[..eq_pos].trim();
        let raw_value = part[eq_pos + 1..].trim();

        let value = parse_value(raw_value, line_num, vars)?;
        map.insert(key.to_string(), value);
    }

    Ok(serde_json::Value::Object(map))
}

/// Split params by comma, but not inside quotes.
fn split_params(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut in_quote = false;
    let mut quote_char = '\'';

    for (i, ch) in input.char_indices() {
        if !in_quote && (ch == '\'' || ch == '"') {
            in_quote = true;
            quote_char = ch;
        } else if in_quote && ch == quote_char {
            in_quote = false;
        } else if !in_quote && ch == ',' {
            parts.push(&input[start..i]);
            start = i + 1;
        }
    }
    parts.push(&input[start..]);
    parts
}

/// Parse a single config value: 'string', $var, number, bool.
fn parse_value(
    raw: &str,
    line_num: usize,
    vars: &HashMap<String, String>,
) -> Result<serde_json::Value, MermaidError> {
    // Quoted string
    if (raw.starts_with('\'') && raw.ends_with('\''))
        || (raw.starts_with('"') && raw.ends_with('"'))
    {
        let inner = &raw[1..raw.len() - 1];
        let resolved = substitute_vars(inner, line_num, vars)?;
        return Ok(serde_json::Value::String(resolved));
    }

    // Bare $variable
    if raw.starts_with('$') {
        let var_name = &raw[1..];
        let val = vars.get(var_name).ok_or_else(|| MermaidError::UnknownVariable {
            line: line_num,
            var: var_name.to_string(),
        })?;
        return Ok(serde_json::Value::String(val.clone()));
    }

    // Boolean
    if raw == "true" {
        return Ok(serde_json::Value::Bool(true));
    }
    if raw == "false" {
        return Ok(serde_json::Value::Bool(false));
    }

    // Integer
    if let Ok(n) = raw.parse::<i64>() {
        return Ok(serde_json::json!(n));
    }

    // Float
    if let Ok(n) = raw.parse::<f64>() {
        return Ok(serde_json::json!(n));
    }

    // Fallback: treat as string
    Ok(serde_json::Value::String(raw.to_string()))
}

/// Replace `$var` occurrences inside a string.
fn substitute_vars(
    input: &str,
    line_num: usize,
    vars: &HashMap<String, String>,
) -> Result<String, MermaidError> {
    if !input.contains('$') {
        return Ok(input.to_string());
    }

    let mut result = String::with_capacity(input.len());
    let mut chars = input.char_indices().peekable();

    while let Some((i, ch)) = chars.next() {
        if ch == '$' {
            // Collect variable name (alphanumeric + underscore)
            let var_start = i + 1;
            let mut var_end = var_start;
            while let Some(&(j, c)) = chars.peek() {
                if c.is_alphanumeric() || c == '_' {
                    var_end = j + c.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            let var_name = &input[var_start..var_end];
            if var_name.is_empty() {
                result.push('$');
                continue;
            }
            let val = vars.get(var_name).ok_or_else(|| MermaidError::UnknownVariable {
                line: line_num,
                var: var_name.to_string(),
            })?;
            result.push_str(val);
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

/// Parse `from -->|port| to` or `from -->|from_port:to_port| to`.
fn parse_edge(line: &str, line_num: usize) -> Result<EdgeDef, MermaidError> {
    let err = |detail: &str| MermaidError::InvalidEdge {
        line: line_num,
        detail: detail.to_string(),
    };

    let arrow_pos = line.find("-->").ok_or_else(|| err("missing -->"))?;
    let from_node = line[..arrow_pos].trim();
    if from_node.is_empty() {
        return Err(err("empty source node"));
    }

    let after_arrow = line[arrow_pos + 3..].trim();

    // -->|port| to  or  -->|from:to| to
    if after_arrow.starts_with('|') {
        let pipe_close = after_arrow[1..]
            .find('|')
            .ok_or_else(|| err("missing closing |"))?;
        let port_spec = &after_arrow[1..pipe_close + 1];
        let to_node = after_arrow[pipe_close + 2..].trim();
        if to_node.is_empty() {
            return Err(err("empty target node"));
        }

        let (from_port, to_port) = if let Some(colon_pos) = port_spec.find(':') {
            (
                port_spec[..colon_pos].trim().to_string(),
                port_spec[colon_pos + 1..].trim().to_string(),
            )
        } else {
            (port_spec.trim().to_string(), port_spec.trim().to_string())
        };

        Ok(EdgeDef {
            from_node: from_node.to_string(),
            from_port,
            to_node: to_node.to_string(),
            to_port,
        })
    } else {
        // --> to (no port spec — not supported, ports are required)
        Err(err("missing port label |port| on edge"))
    }
}

// ─── Export ──────────────────────────────────────────────────────────────────

/// Render a GraphDefinition as a Mermaid flowchart string.
pub fn to_mermaid(def: &GraphDefinition) -> String {
    let mut out = String::from("graph LR\n");

    // Nodes
    for node in &def.nodes {
        out.push_str("    ");
        out.push_str(&node.name);
        out.push_str("[\"");
        out.push_str(&node.node_type);

        let config = node.config.as_object();
        if let Some(obj) = config {
            if !obj.is_empty() {
                out.push('(');
                let params: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, format_value(v)))
                    .collect();
                out.push_str(&params.join(", "));
                out.push(')');
            }
        }

        out.push_str("\"]\n");
    }

    if !def.nodes.is_empty() && !def.edges.is_empty() {
        out.push('\n');
    }

    // Edges
    for edge in &def.edges {
        out.push_str("    ");
        out.push_str(&edge.from_node);
        out.push_str(" -->|");
        if edge.from_port == edge.to_port {
            out.push_str(&edge.from_port);
        } else {
            out.push_str(&edge.from_port);
            out.push(':');
            out.push_str(&edge.to_port);
        }
        out.push_str("| ");
        out.push_str(&edge.to_node);
        out.push('\n');
    }

    out
}

/// Format a JSON value for Mermaid config params.
fn format_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => format!("'{s}'"),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        other => format!("'{other}'"),
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_graph() {
        let input = r#"
graph LR
    a["ComposeNode"]
    b["PrimarySearchNode"]

    a -->|results| b
"#;
        let def = parse_mermaid(input).unwrap();
        assert_eq!(def.nodes.len(), 2);
        assert_eq!(def.edges.len(), 1);
        assert_eq!(def.nodes[0].name, "a");
        assert_eq!(def.nodes[0].node_type, "ComposeNode");
        assert_eq!(def.nodes[1].name, "b");
        assert_eq!(def.edges[0].from_node, "a");
        assert_eq!(def.edges[0].from_port, "results");
        assert_eq!(def.edges[0].to_port, "results");
        assert_eq!(def.edges[0].to_node, "b");
    }

    #[test]
    fn parse_node_with_config() {
        let input = r#"
graph LR
    f["FetchRelatedNode(relation='HAS_FILE', direction='Outgoing', limit=10)"]
"#;
        let def = parse_mermaid(input).unwrap();
        assert_eq!(def.nodes.len(), 1);
        let cfg = &def.nodes[0].config;
        assert_eq!(cfg["relation"], "HAS_FILE");
        assert_eq!(cfg["direction"], "Outgoing");
        assert_eq!(cfg["limit"], 10);
    }

    #[test]
    fn parse_node_no_config() {
        let input = "graph LR\n    x[\"ComposeNode\"]";
        let def = parse_mermaid(input).unwrap();
        assert_eq!(def.nodes[0].config, serde_json::json!({}));
    }

    #[test]
    fn parse_edge_explicit_ports() {
        let input = "graph LR\n    a -->|results:children| b";
        let def = parse_mermaid(input).unwrap();
        assert_eq!(def.edges[0].from_port, "results");
        assert_eq!(def.edges[0].to_port, "children");
    }

    #[test]
    fn parse_edge_shorthand_port() {
        let input = "graph LR\n    a -->|query| b";
        let def = parse_mermaid(input).unwrap();
        assert_eq!(def.edges[0].from_port, "query");
        assert_eq!(def.edges[0].to_port, "query");
    }

    #[test]
    fn parse_with_comments_and_blanks() {
        let input = r#"
graph LR
    %% This is a comment
    a["ComposeNode"]

    %% Another comment
    b["PrimarySearchNode"]
"#;
        let def = parse_mermaid(input).unwrap();
        assert_eq!(def.nodes.len(), 2);
    }

    #[test]
    fn parse_inline_comment() {
        let input = "graph LR\n    a[\"ComposeNode\"] %% inline comment";
        let def = parse_mermaid(input).unwrap();
        assert_eq!(def.nodes.len(), 1);
    }

    #[test]
    fn parse_template_variables() {
        let mut vars = HashMap::new();
        vars.insert("kb".to_string(), "TreeKB".to_string());
        vars.insert("q".to_string(), "hello world".to_string());

        let input = r#"
graph LR
    qs["QuerySourceNode(kb_name='$kb', query='$q')"]
"#;
        let def = parse_mermaid_template(input, &vars).unwrap();
        assert_eq!(def.nodes[0].config["kb_name"], "TreeKB");
        assert_eq!(def.nodes[0].config["query"], "hello world");
    }

    #[test]
    fn parse_template_unknown_var_errors() {
        let input = "graph LR\n    qs[\"QuerySourceNode(kb_name='$missing')\"]";
        let err = parse_mermaid_template(input, &HashMap::new()).unwrap_err();
        assert!(matches!(err, MermaidError::UnknownVariable { var, .. } if var == "missing"));
    }

    #[test]
    fn parse_missing_header_errors() {
        let err = parse_mermaid("a[\"Foo\"]").unwrap_err();
        assert!(matches!(err, MermaidError::MissingHeader));
    }

    #[test]
    fn parse_empty_errors() {
        let err = parse_mermaid("").unwrap_err();
        assert!(matches!(err, MermaidError::MissingHeader));
    }

    #[test]
    fn parse_edge_without_port_errors() {
        let input = "graph LR\n    a --> b";
        let err = parse_mermaid(input).unwrap_err();
        assert!(matches!(err, MermaidError::InvalidEdge { .. }));
    }

    #[test]
    fn parse_bool_and_float_config() {
        let input = "graph LR\n    n[\"EmbedRecordNode(gpu_batch_size=64, enabled=true)\"]";
        let def = parse_mermaid(input).unwrap();
        assert_eq!(def.nodes[0].config["gpu_batch_size"], 64);
        assert_eq!(def.nodes[0].config["enabled"], true);
    }

    #[test]
    fn to_mermaid_roundtrip() {
        let input = r#"graph LR
    query_source["QuerySourceNode(kb_name='TreeKB', query='test')"]
    primary_search["PrimarySearchNode"]
    fetch_0["FetchRelatedNode(direction='Outgoing', limit=10, relation='HAS_FILE')"]
    compose["ComposeNode"]

    query_source -->|query| primary_search
    primary_search -->|results| fetch_0
    primary_search -->|results| compose
    fetch_0 -->|children| compose
"#;
        let def = parse_mermaid(input).unwrap();
        let exported = to_mermaid(&def);
        let def2 = parse_mermaid(&exported).unwrap();

        assert_eq!(def.nodes.len(), def2.nodes.len());
        assert_eq!(def.edges.len(), def2.edges.len());
        for (a, b) in def.nodes.iter().zip(def2.nodes.iter()) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.node_type, b.node_type);
            assert_eq!(a.config, b.config);
        }
        for (a, b) in def.edges.iter().zip(def2.edges.iter()) {
            assert_eq!(a.from_node, b.from_node);
            assert_eq!(a.from_port, b.from_port);
            assert_eq!(a.to_node, b.to_node);
            assert_eq!(a.to_port, b.to_port);
        }
    }

    #[test]
    fn to_mermaid_explicit_ports() {
        let def = GraphDefinition {
            nodes: vec![],
            edges: vec![EdgeDef {
                from_node: "a".into(),
                from_port: "results".into(),
                to_node: "b".into(),
                to_port: "children".into(),
            }],
        };
        let mmd = to_mermaid(&def);
        assert!(mmd.contains("-->|results:children|"));
    }

    #[test]
    fn from_definition_with_registry() {
        use crate::dataflow::graph::DataflowGraph;
        use crate::dataflow::node_factories::register_builtins;
        use crate::dataflow::node_registry::NodeRegistry;

        let input = r#"
graph LR
    inserts["InsertRecordNode"]
    links["LinkRecordNode"]

    inserts -->|done:trigger| links
"#;
        let def = parse_mermaid(input).unwrap();

        let mut registry = NodeRegistry::new();
        register_builtins(&mut registry);

        let graph = DataflowGraph::from_definition(&def, &registry).unwrap();
        assert_eq!(graph.node_names().len(), 2);

        let order = graph.topological_sort().unwrap();
        assert_eq!(order, vec!["inserts", "links"]);
    }

    #[test]
    fn parse_graph_td_header() {
        let input = "graph TD\n    a[\"ComposeNode\"]";
        let def = parse_mermaid(input).unwrap();
        assert_eq!(def.nodes.len(), 1);
    }
}
