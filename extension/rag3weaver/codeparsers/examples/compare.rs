/// Compare Rust codeparsers output with TS codeparsers output.
/// Usage: cargo run --example compare -- <project_root>
///
/// Parses all .ts files in the project and outputs JSON to stdout.
/// Compare with the TS version's output.
use codeparsers::parallel::project_parser::{
    ProjectParser, ProjectParserOptions, ParseProjectOptions,
};

use std::env;
use std::path::Path;

fn find_ts_files(dir: &Path, results: &mut Vec<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "node_modules" || name == "dist" || name == "target" {
            continue;
        }
        if path.is_dir() {
            find_ts_files(&path, results);
        } else if name.ends_with(".ts") && !name.ends_with(".d.ts") {
            results.push(path.to_string_lossy().to_string());
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let root = if args.len() > 1 {
        args[1].clone()
    } else {
        eprintln!("Usage: cargo run --example compare -- <project_root>");
        std::process::exit(1);
    };

    // Find all .ts files
    let mut files = Vec::new();
    find_ts_files(Path::new(&root), &mut files);
    files.sort();

    eprintln!("Found {} TypeScript files in {}", files.len(), root);

    // Parse with Rust codeparsers
    let parser = ProjectParser::new(ProjectParserOptions { verbose: true });
    let result = parser.parse_project(ParseProjectOptions {
        root: root.clone(),
        files: files.clone(),
        content_map: None,
        resolve_relationships: Some(true),
        resolver_options: None,
    });

    // Output summary to stderr
    eprintln!("\n=== RUST CODEPARSERS RESULTS ===");
    eprintln!("Total files:      {}", result.stats.total_files);
    eprintln!("Successful files: {}", result.stats.successful_files);
    eprintln!("Failed files:     {}", result.stats.failed_files);
    eprintln!("Total scopes:     {}", result.stats.total_scopes);
    eprintln!("Parse time:       {}ms", result.stats.parse_time_ms);
    if let Some(rel_time) = result.stats.relationship_time_ms {
        eprintln!("Relationship time: {}ms", rel_time);
    }

    if !result.errors.is_empty() {
        eprintln!("\nErrors:");
        for e in &result.errors {
            eprintln!("  {}: {}", e.file, e.error);
        }
    }

    // Per-file scope summary
    eprintln!("\n=== PER-FILE SCOPES ===");
    let mut sorted_files: Vec<_> = result.files.iter().collect();
    sorted_files.sort_by_key(|(path, _)| path.clone());
    for (path, analysis) in &sorted_files {
        let rel = path.strip_prefix(&root).unwrap_or(path).trim_start_matches('/');
        eprintln!("{}: {} scopes, {} lines", rel, analysis.scopes.len(), analysis.total_lines);
        for scope in &analysis.scopes {
            eprintln!("  [{:?}] {} (L{}-L{})", scope.r#type, scope.name, scope.scope_start_line, scope.scope_end_line);
        }
    }

    if let Some(ref rels) = result.relationships {
        eprintln!("\n=== RELATIONSHIPS ===");
        eprintln!("Total: {}", rels.relationships.len());
        // Count by type
        let mut by_type = std::collections::HashMap::new();
        for rel in &rels.relationships {
            *by_type.entry(format!("{:?}", rel.r#type)).or_insert(0usize) += 1;
        }
        let mut by_type_sorted: Vec<_> = by_type.into_iter().collect();
        by_type_sorted.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        for (rtype, count) in by_type_sorted {
            eprintln!("  {}: {}", rtype, count);
        }
    }

    // Build normalized JSON: { files: { rel_path: { scopes: [...] } }, relationships: [...] }
    let mut out = serde_json::Map::new();

    // Files with scopes (relative paths, minimal fields)
    let mut files_obj = serde_json::Map::new();
    for (abs_path, analysis) in &result.files {
        let rel = abs_path.strip_prefix(&root).unwrap_or(abs_path).trim_start_matches('/');
        let scopes: Vec<serde_json::Value> = analysis.scopes.iter().map(|s| {
            serde_json::json!({
                "name": s.name,
                "type": serde_json::to_value(&s.r#type).unwrap(),
                "startLine": s.scope_start_line,
                "endLine": s.scope_end_line,
            })
        }).collect();
        files_obj.insert(rel.to_string(), serde_json::json!({ "scopes": scopes }));
    }
    out.insert("files".into(), serde_json::Value::Object(files_obj));

    // Relationships
    if let Some(ref rels) = result.relationships {
        let rels_arr: Vec<serde_json::Value> = rels.relationships.iter().map(|r| {
            serde_json::json!({
                "from": r.from_name,
                "to": r.to_name,
                "type": serde_json::to_value(&r.r#type).unwrap(),
                "fromFile": r.from_file,
                "toFile": r.to_file,
            })
        }).collect();
        out.insert("relationships".into(), serde_json::json!(rels_arr));
    }

    println!("{}", serde_json::to_string(&serde_json::Value::Object(out)).unwrap());
}
