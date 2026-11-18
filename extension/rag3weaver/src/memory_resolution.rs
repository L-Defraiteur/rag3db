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

/// **The code schema for the reference tools**: files, scopes, libraries, symbols and their relations, without
/// the derived content (`ScopeContent`), chunks, vectors or full text. What the fallback needs, and nothing it
/// would have to derive.
///
/// No form stamp, no generations: the fallback is one image of the worktree as it is on disk, never written to a
/// base. The analysis version is the full schema's, so that a version written from here would be the same.
pub fn register_code_schema_for_references(catalog: &mut crate::Catalog) -> Result<(), crate::catalog::CatalogError> {
    use crate::code::{FILE, LIBRARY, RELATIONS, SCOPE, SYMBOL};
    let chunking = crate::code::default_scope_chunking();
    catalog.version_de_l_analyse = crate::code::version_de_l_analyse(&chunking);
    let (mut entities, mut relations) = crate::code::schema_configs(chunking);
    for name in [FILE, SCOPE, LIBRARY, SYMBOL] {
        let mut config = entities.remove(name).expect("an entity of the code schema");
        config.chunked = Some(false);
        config.signals = crate::search::SearchSignals::NONE;
        config.search_index = None;
        config.derived = None;
        catalog.register_entity(name, config)?;
    }
    for (name, _, _) in RELATIONS {
        let relation = relations.remove(name).expect("a relation of the code schema");
        catalog.register_relation_with(name, &relation.from, &relation.to, relation.properties.unwrap_or_default())?;
    }
    Ok(())
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

    fn catalog(d: &std::path::Path) -> Catalog {
        let conn = Rag3dbConnection::new(&d.join("base.rag3db")).expect("the base");
        let mut cat = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(8)), CatalogConfig { allow_mock_embedder: true, embedding_dim: 8, ..Default::default() });
        cat.initialize().unwrap();
        cat
    }

    /// `(from file, from name, to name, to file, mark)` of every `CONSUMES`, sorted.
    fn by_the_base(analysis: &CodeAnalysis) -> Vec<String> {
        let d = tempfile::tempdir().unwrap();
        let mut cat = catalog(d.path());
        crate::code::register_code_schema(&mut cat, crate::code::default_scope_chunking()).unwrap();
        cat.ingest_code(analysis).unwrap();
        consumes_of(&cat)
    }

    fn consumes_of(cat: &Catalog) -> Vec<String> {
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

    #[test]
    fn the_references_schema_draws_the_same_links_without_derived_content() {
        let analysis = crate::code::analyze("/virtual", project());
        let d = tempfile::tempdir().unwrap();
        let mut cat = catalog(d.path());
        register_code_schema_for_references(&mut cat).unwrap();
        assert!(cat.entity_config(crate::code::SCOPE_CONTENT).is_none(), "no derived content");
        for name in ["File", "Scope", "Library", "Symbol"] {
            let config = cat.entity_config(name).unwrap_or_else(|| panic!("{name} is registered"));
            assert!(config.chunked != Some(true) && config.derived.is_none(), "{name} has no chunks and derives nothing");
        }
        cat.ingest_code(&analysis).unwrap();
        assert_eq!(consumes_of(&cat), by_the_base(&analysis), "the same links as the full schema, mark for mark");
    }
}
