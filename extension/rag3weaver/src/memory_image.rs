//! **The memory image** (B of the reference tools before the index, recherche and codeparsers, 18 November 2026,
//! page `docs/recherche/18-11-2026/02-les-outils-de-reference-avant-l-index.md`).
//!
//! The rows an ingestion would write — nodes by table, relations by name, with their properties and the same uuids —
//! kept in memory, compact: strings are interned, nodes and relations are `u32` ids. codeparsers fills it from the
//! analysis of a view, file by file, and resolves into it; a read face executes the IR of the tools on it. It is
//! indexed by uuid, by file, by `(relation, from)`, by `(relation, to)`, and by the fields declared with
//! [`MemoryImage::index_field`] (a symbol by its name). [`MemoryImage::remove_file`] takes a file out, both directions
//! of its relations included, and gives back the names of the symbols its nodes defined or mentioned, to resolve again.

use std::collections::{BTreeMap, BTreeSet};

use crate::connection::CypherValue;

/// A node of the image, as a reader sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeView {
    pub table: String,
    pub uuid: String,
    pub file: Option<String>,
    pub props: BTreeMap<String, CypherValue>,
}

/// What [`MemoryImage::remove_file`] took out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemovedFile {
    pub nodes: usize,
    pub relations: usize,
    /// The names of the symbols (see [`MemoryImage::set_symbol_table`]) linked to a removed node, in either direction.
    pub symbol_names: BTreeSet<String>,
}

/// The image. See the module.
#[derive(Debug, Default)]
pub struct MemoryImage {
    _lot: (),
}

impl MemoryImage {
    pub fn new() -> Self {
        Self::default()
    }

    /// The table of the symbols and the field of their name: [`remove_file`](Self::remove_file) gives back these
    /// names. Also indexes that field.
    pub fn set_symbol_table(&mut self, _table: &str, _name_field: &str) {
        unimplemented!("memory image: lot 2a")
    }

    /// Index `table.field` (a text field) for [`find`](Self::find).
    pub fn index_field(&mut self, _table: &str, _field: &str) {
        unimplemented!("memory image: lot 2a")
    }

    /// Insert a node, or replace the properties and file of the node of this uuid.
    pub fn insert_node(&mut self, _table: &str, _uuid: &str, _file: Option<&str>, _props: BTreeMap<String, CypherValue>) {
        unimplemented!("memory image: lot 2a")
    }

    /// Insert a relation between two nodes of the image, as a MERGE: a pair `(rel, from, to)` already there takes
    /// the new properties (the last wins) and is not doubled. `false` when one end is missing.
    pub fn insert_rel(&mut self, _rel: &str, _from: &str, _to: &str, _props: BTreeMap<String, CypherValue>) -> bool {
        unimplemented!("memory image: lot 2a")
    }

    pub fn node(&self, _uuid: &str) -> Option<NodeView> {
        unimplemented!("memory image: lot 2a")
    }

    /// The relations `rel` leaving `from`: the uuid reached and the properties of the relation.
    pub fn outgoing(&self, _rel: &str, _from: &str) -> Vec<(String, BTreeMap<String, CypherValue>)> {
        unimplemented!("memory image: lot 2a")
    }

    /// The relations `rel` reaching `to`: the uuid it leaves and the properties of the relation.
    pub fn incoming(&self, _rel: &str, _to: &str) -> Vec<(String, BTreeMap<String, CypherValue>)> {
        unimplemented!("memory image: lot 2a")
    }

    /// The uuids of the nodes of `table` whose `field` is `value` (by its index when declared, else by a scan).
    pub fn find(&self, _table: &str, _field: &str, _value: &CypherValue) -> Vec<String> {
        unimplemented!("memory image: lot 2a")
    }

    /// The uuids of the nodes of `table` (all of them, in insertion order).
    pub fn nodes_of_table(&self, _table: &str) -> Vec<String> {
        unimplemented!("memory image: lot 2a")
    }

    pub fn nodes_of_file(&self, _path: &str) -> Vec<String> {
        unimplemented!("memory image: lot 2a")
    }

    /// Take `path` out: its nodes and all their relations, both directions.
    pub fn remove_file(&mut self, _path: &str) -> RemovedFile {
        unimplemented!("memory image: lot 2a")
    }

    /// The live nodes and relations.
    pub fn counts(&self) -> (usize, usize) {
        unimplemented!("memory image: lot 2a")
    }

    /// The interned strings, to see that the image does not repeat them.
    pub fn interned_strings(&self) -> usize {
        unimplemented!("memory image: lot 2a")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props(pairs: &[(&str, CypherValue)]) -> BTreeMap<String, CypherValue> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    fn s(v: &str) -> CypherValue {
        CypherValue::String(v.to_string())
    }

    /// Two scopes in two files, a symbol defined by one and mentioned by the other, and a call between them.
    fn image() -> MemoryImage {
        let mut m = MemoryImage::new();
        m.set_symbol_table("Symbol", "name");
        m.insert_node("Scope", "s-a", Some("src/a.rs"), props(&[("name", s("alpha")), ("start_line", CypherValue::Int(1))]));
        m.insert_node("Scope", "s-b", Some("src/b.rs"), props(&[("name", s("beta")), ("start_line", CypherValue::Int(3))]));
        m.insert_node("Symbol", "y-beta", None, props(&[("name", s("beta"))]));
        assert!(m.insert_rel("DEFINES", "y-beta", "s-b", BTreeMap::new()));
        assert!(m.insert_rel("MENTIONS", "y-beta", "s-a", props(&[("line", CypherValue::Int(2))])));
        assert!(m.insert_rel("CONSUMES", "s-a", "s-b", props(&[("usage", s("call")), ("resolution", s("unique"))])));
        m
    }

    #[test]
    fn the_image_gives_back_its_nodes_and_both_directions_of_a_relation() {
        let m = image();
        let a = m.node("s-a").expect("s-a");
        assert_eq!((a.table.as_str(), a.file.as_deref(), a.props.get("name")), ("Scope", Some("src/a.rs"), Some(&s("alpha"))));
        assert_eq!(m.outgoing("CONSUMES", "s-a"), vec![("s-b".to_string(), props(&[("usage", s("call")), ("resolution", s("unique"))]))]);
        assert_eq!(m.incoming("CONSUMES", "s-b").into_iter().map(|(u, _)| u).collect::<Vec<_>>(), ["s-a"]);
        assert!(m.outgoing("CONSUMES", "s-b").is_empty(), "no relation from s-b");
        assert!(!m.insert_rel_missing_end_is_refused());
        assert_eq!(m.counts(), (3, 3));
    }

    #[test]
    fn a_symbol_is_found_by_its_name_and_a_table_lists_its_nodes() {
        let m = image();
        assert_eq!(m.find("Symbol", "name", &s("beta")), ["y-beta"]);
        assert!(m.find("Symbol", "name", &s("gamma")).is_empty());
        assert_eq!(m.find("Scope", "name", &s("alpha")), ["s-a"], "an undeclared field is found by a scan");
        assert_eq!(m.nodes_of_table("Scope"), ["s-a", "s-b"]);
        assert_eq!(m.nodes_of_file("src/b.rs"), ["s-b"]);
    }

    #[test]
    fn removing_a_file_takes_its_relations_both_ways_and_gives_back_its_symbol_names() {
        let mut m = image();
        let removed = m.remove_file("src/b.rs");
        assert_eq!(removed, RemovedFile { nodes: 1, relations: 2, symbol_names: BTreeSet::from(["beta".to_string()]) });
        assert!(m.node("s-b").is_none());
        assert!(m.outgoing("CONSUMES", "s-a").is_empty(), "the call into the removed file is gone");
        assert!(m.outgoing("DEFINES", "y-beta").is_empty());
        assert_eq!(m.outgoing("MENTIONS", "y-beta").len(), 1, "the mention from the other file stays");
        assert_eq!(m.counts(), (2, 1));
        // The file comes back.
        m.insert_node("Scope", "s-b", Some("src/b.rs"), props(&[("name", s("beta"))]));
        assert!(m.insert_rel("DEFINES", "y-beta", "s-b", BTreeMap::new()));
        assert_eq!(m.nodes_of_file("src/b.rs"), ["s-b"]);
        assert_eq!(m.counts(), (3, 2));
    }

    #[test]
    fn a_relation_is_merged_not_doubled() {
        let mut m = image();
        assert!(m.insert_rel("CONSUMES", "s-a", "s-b", props(&[("usage", s("call")), ("resolution", s("nom"))])));
        assert_eq!(m.outgoing("CONSUMES", "s-a"), vec![("s-b".to_string(), props(&[("usage", s("call")), ("resolution", s("nom"))]))]);
        assert_eq!(m.incoming("CONSUMES", "s-b").len(), 1, "the pair is not doubled");
        assert_eq!(m.counts(), (3, 3));
    }

    #[test]
    fn removing_a_file_keeps_the_symbols_without_a_file() {
        let mut m = image();
        let removed = m.remove_file("src/a.rs");
        assert_eq!(removed.symbol_names, BTreeSet::from(["beta".to_string()]), "s-a mentioned beta");
        assert!(m.node("y-beta").is_some(), "the symbol stays");
        assert_eq!(m.outgoing("DEFINES", "y-beta").len(), 1);
    }

    #[test]
    fn strings_are_interned_once() {
        let mut m = MemoryImage::new();
        for i in 0..100 {
            m.insert_node("Scope", &format!("s-{i}"), Some("src/same.rs"), props(&[("kind", s("function")), ("lang", s("rust"))]));
        }
        // Table, file, two keys, two values, and the hundred uuids.
        assert_eq!(m.interned_strings(), 106);
    }

    impl MemoryImage {
        fn insert_rel_missing_end_is_refused(&self) -> bool {
            let mut copy = MemoryImage::new();
            copy.insert_node("Scope", "only", None, BTreeMap::new());
            copy.insert_rel("CONSUMES", "only", "absent", BTreeMap::new())
        }
    }
}
