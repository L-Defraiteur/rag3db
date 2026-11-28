//! **The editor's edits, applied to a `.mmd` line by line** (the workflow editor's third lot, 28 November 2026). The
//! `.mmd` is the truth (the ticket of 26 November): an edit rewrites only the line it touches — a node's setting in its
//! declaration, an edge added after the last edge, an edge's line removed — and every other line stays as it was, its
//! comments and its spacing with it, so that the diff says the change and nothing else.

use serde::Deserialize;
use serde_json::Value;

use super::mermaid::split_params;

/// One edit of the editor's draft.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Edit {
    /// A setting of a node: its value written in the node's declaration (a `$param` stays a reference).
    Set { node: String, key: String, value: Value },
    /// A setting removed from a node's declaration.
    Unset { node: String, key: String },
    /// An edge from an output to an input.
    Connect { from_node: String, from_port: String, to_node: String, to_port: String },
    /// An edge removed.
    Disconnect { from_node: String, from_port: String, to_node: String, to_port: String },
}

/// A value as a node's declaration writes it: a `$param` as it is, a text between apostrophes — between quotes when it
/// holds an apostrophe (the parser takes what lies between the two, escaping nothing) — the rest as JSON. A text that
/// holds both cannot be written, and says so.
fn written(v: &Value) -> Result<String, String> {
    Ok(match v {
        Value::String(s) if s.starts_with('$') && s[1..].chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') => s.clone(),
        Value::String(s) if !s.contains('\'') => format!("'{s}'"),
        Value::String(s) if !s.contains('"') => format!("\"{s}\""),
        Value::String(s) => return Err(format!("a text holding both an apostrophe and a quote cannot be written in a declaration: {s}")),
        other => other.to_string(),
    })
}

/// The code of a line, before its `%%` comment.
fn code(line: &str) -> &str {
    line.split("%%").next().unwrap_or("")
}

fn node_line(lines: &[String], node: &str) -> Result<usize, String> {
    lines
        .iter()
        .position(|l| code(l).trim_start().strip_prefix(node).is_some_and(|rest| rest.starts_with("[\"")))
        .ok_or_else(|| format!("no node « {node} » in this graph"))
}

/// The edge a line declares, if it declares one: (from, from_port, to, to_port).
fn edge_of(line: &str) -> Option<(String, String, String, String)> {
    let c = code(line).trim();
    let (from, rest) = c.split_once("-->")?;
    let rest = rest.trim().strip_prefix('|')?;
    let (ports, to) = rest.split_once('|')?;
    let (fp, tp) = ports.split_once(':').unwrap_or((ports, ports));
    Some((from.trim().into(), fp.trim().into(), to.trim().into(), tp.trim().into()))
}

/// Writes `key=value` in a node's declaration line: replaced where the key is, added after the others otherwise; or
/// removed (`value: None`).
fn set_in_line(line: &str, key: &str, value: Option<&Value>) -> Result<String, String> {
    let open = line.find("[\"").ok_or("a node's declaration without [\"")?;
    let close = line.rfind("\"]").ok_or("a node's declaration without \"]")?;
    let label = &line[open + 2..close];
    let (node_type, args) = match label.find('(') {
        Some(p) => (&label[..p], &label[p + 1..label.rfind(')').ok_or("a node's settings without their )")?]),
        None => (label, ""),
    };
    let mut parts: Vec<String> = split_params(args).into_iter().map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect();
    let at = parts.iter().position(|p| p.split_once('=').is_some_and(|(k, _)| k.trim() == key));
    match (at, value) {
        (Some(i), Some(v)) => parts[i] = format!("{key}={}", written(v)?),
        (None, Some(v)) => parts.push(format!("{key}={}", written(v)?)),
        (Some(i), None) => {
            parts.remove(i);
        }
        (None, None) => return Err(format!("the setting « {key} » is not in this node's declaration")),
    }
    let settings = if parts.is_empty() { String::new() } else { format!("({})", parts.join(", ")) };
    Ok(format!("{}[\"{}{settings}\"]{}", &line[..open], node_type.trim_end(), &line[close + 2..]))
}

/// Applies `edits` to the text of a `.mmd`, in order; every line an edit does not touch is kept as it was.
pub fn apply(text: &str, edits: &[Edit]) -> Result<String, String> {
    let trailing_newline = text.ends_with('\n');
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    for e in edits {
        match e {
            Edit::Set { node, key, value } => {
                let i = node_line(&lines, node)?;
                lines[i] = set_in_line(&lines[i], key, Some(value))?;
            }
            Edit::Unset { node, key } => {
                let i = node_line(&lines, node)?;
                lines[i] = set_in_line(&lines[i], key, None)?;
            }
            Edit::Connect { from_node, from_port, to_node, to_port } => {
                node_line(&lines, from_node)?;
                node_line(&lines, to_node)?;
                let wanted = (from_node.clone(), from_port.clone(), to_node.clone(), to_port.clone());
                if lines.iter().any(|l| edge_of(l).as_ref() == Some(&wanted)) {
                    return Err(format!("the edge {from_node}.{from_port} → {to_node}.{to_port} is already there"));
                }
                // After the last edge, with its indentation; after the last line otherwise.
                let last = lines.iter().rposition(|l| edge_of(l).is_some());
                let indent = last.map(|i| lines[i].len() - lines[i].trim_start().len()).unwrap_or(4);
                let ports = if from_port == to_port { from_port.clone() } else { format!("{from_port}:{to_port}") };
                let new = format!("{}{from_node} -->|{ports}| {to_node}", " ".repeat(indent));
                lines.insert(last.map_or(lines.len(), |i| i + 1), new);
            }
            Edit::Disconnect { from_node, from_port, to_node, to_port } => {
                let wanted = (from_node.clone(), from_port.clone(), to_node.clone(), to_port.clone());
                let i = lines
                    .iter()
                    .position(|l| edge_of(l).as_ref() == Some(&wanted))
                    .ok_or_else(|| format!("no edge {from_node}.{from_port} → {to_node}.{to_port} in this graph"))?;
                lines.remove(i);
            }
        }
    }
    let mut out = lines.join("\n");
    if trailing_newline {
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
#[path = "mermaid_edit/tests.rs"]
mod tests;
