//! **The code of a view as a [`MemoryImage`]** (B, page `docs/recherche/18-11-2026/02-les-outils-de-reference-avant-l-index.md`,
//! ticket `docs/recherche/tickets/2026-11-18-les-outils-de-reference-par-des-tables-a-soi-avant-l-index.md`).
//!
//! The reference tools answer before the index from an image built by the parsing: the same rows the ingestion
//! would write (files, scopes, libraries, symbols, the analysis' relations, the rendezvous), and the links the
//! resolution chooses across files, by the same choice functions as the ingestion.

use crate::code::CodeAnalysis;
use crate::memory_image::MemoryImage;

/// The image of an analysis: its rows, and the links the resolution draws between its files.
pub fn memory_image_of(_analysis: &CodeAnalysis) -> MemoryImage {
    MemoryImage::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedder::HashEmbedder;
    use crate::{Catalog, CatalogConfig, Rag3dbConnection};

    /// Two Rust files: a type and its method in one, a call across files in the other.
    fn project() -> Vec<(String, String)> {
        vec![
            ("settings.rs".into(), "pub struct Settings { pub depth: u32 }\nimpl Settings {\n    pub fn new() -> Self { Settings { depth: 4 } }\n    pub fn depth(&self) -> u32 { self.depth }\n}\npub fn helper(n: u32) -> u32 { n + 1 }\n".into()),
            ("main.rs".into(), "use crate::settings::{Settings, helper};\npub fn start() -> u32 {\n    let s = Settings::new();\n    helper(s.depth())\n}\n".into()),
        ]
    }

    /// `(from file, from name, to name, to file, mark)` of every `CONSUMES`, sorted.
    fn by_the_base(analysis: &CodeAnalysis) -> Vec<String> {
        let d = tempfile::tempdir().unwrap();
        let conn = Rag3dbConnection::new(&d.path().join("base.rag3db")).expect("the base");
        let mut cat = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(8)), CatalogConfig { allow_mock_embedder: true, embedding_dim: 8, ..Default::default() });
        cat.initialize().unwrap();
        crate::code::register_code_schema(&mut cat, crate::code::default_scope_chunking()).unwrap();
        cat.ingest_code(analysis).unwrap();
        let r = cat.execute_raw("MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) RETURN a.file_path, a.name, b.name, b.file_path, r.resolution").unwrap();
        let mut out: Vec<String> = r.rows.iter().map(|x| (0..5).map(|i| x.get(i).and_then(|v| v.as_str()).unwrap_or("").to_string()).collect::<Vec<_>>().join("\t")).collect();
        out.sort();
        out
    }

    fn by_the_image(image: &MemoryImage) -> Vec<String> {
        let text = |props: &std::collections::BTreeMap<String, crate::connection::CypherValue>, k: &str| props.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let mut out = Vec::new();
        for from in image.nodes_of_table("Scope") {
            let a = image.node(&from).expect("a scope");
            for (to, props) in image.outgoing("CONSUMES", &from) {
                let b = image.node(&to).expect("its target");
                out.push([text(&a.props, "file_path"), text(&a.props, "name"), text(&b.props, "name"), text(&b.props, "file_path"), text(&props, "resolution")].join("\t"));
            }
        }
        out.sort();
        out
    }

    #[test]
    fn the_image_draws_the_same_links_as_the_base() {
        let analysis = crate::code::analyze("/virtual", project());
        let base = by_the_base(&analysis);
        assert!(base.iter().any(|l| l.contains("\tstart\thelper\t")), "the base links the call across files: {base:#?}");
        let image = by_the_image(&memory_image_of(&analysis));
        assert_eq!(image, base, "the image has the base's links, mark for mark");
    }
}
