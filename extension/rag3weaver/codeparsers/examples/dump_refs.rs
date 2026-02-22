
use codeparsers::parallel::project_parser::{ProjectParser, ProjectParserOptions, ParseProjectOptions};

fn main() {
    let root = "/home/luciedefraiteur/LR_CodeRag/community-docs/packages/ragforge-core/packages/codeparsers/src";
    let mut files = Vec::new();
    fn find(dir: &std::path::Path, r: &mut Vec<String>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with('.') || n == "node_modules" || n == "dist" { continue; }
            if p.is_dir() { find(&p, r); }
            else if n.ends_with(".ts") && !n.ends_with(".d.ts") { r.push(p.to_string_lossy().to_string()); }
        }
    }
    find(std::path::Path::new(root), &mut files);
    let parser = ProjectParser::new(ProjectParserOptions { verbose: false });
    let result = parser.parse_project(ParseProjectOptions {
        root: root.to_string(), files, content_map: None, resolve_relationships: Some(true),
    });

    // Dump signatures for scopes involved in direction differences
    let targets = ["globalRegistry", "CSHARP_BUILTIN_IDENTIFIERS", "GO_BUILTIN_IDENTIFIERS",
                    "ParserRegistry", "BUILTIN_IDENTIFIERS"];
    for (_, analysis) in &result.files {
        for scope in &analysis.scopes {
            if targets.contains(&scope.name.as_str()) {
                eprintln!("SCOPE: {} [{:?}] sig={:?}", scope.name, scope.r#type, scope.signature);
            }
        }
    }

    // Dump the specific relationships
    if let Some(ref rels) = result.relationships {
        let matching: Vec<_> = rels.relationships.iter()
            .filter(|r| {
                (r.from_name == "globalRegistry" && r.to_name == "ParserRegistry")
                || (r.from_name == "CSHARP_BUILTIN_IDENTIFIERS" && r.to_name == "BUILTIN_IDENTIFIERS")
                || (r.from_name == "GO_BUILTIN_IDENTIFIERS" && r.to_name == "BUILTIN_IDENTIFIERS")
            })
            .collect();
        for r in &matching {
            eprintln!("REL: {:?} {} -> {}", r.r#type, r.from_name, r.to_name);
        }
    }
}
