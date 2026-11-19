//! **The memory image** (B of the reference tools before the index, recherche and codeparsers, 18 November 2026,
//! page `docs/recherche/18-11-2026/02-les-outils-de-reference-avant-l-index.md`).
//!
//! The rows an ingestion would write — nodes by table, relations by name, with their properties and the same uuids —
//! kept in memory, compact: strings are interned, nodes and relations are `u32` ids. codeparsers fills it from the
//! analysis of a view, file by file, and resolves into it; a read face executes the IR of the tools on it. It is
//! indexed by uuid, by file, by `(relation, from)`, by `(relation, to)`, and by the fields declared with
//! [`MemoryImage::index_field`] (a symbol by its name). [`MemoryImage::remove_file`] takes a file out, both directions
//! of its relations included, and gives back the names of the symbols its nodes defined or mentioned, to resolve again.

use std::collections::{BTreeMap, BTreeSet, HashMap};

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

type Sym = u32;

/// The strings of the image, each kept once.
#[derive(Debug, Default)]
struct Interner {
    ids: HashMap<Box<str>, Sym>,
    names: Vec<Box<str>>,
}

impl Interner {
    fn intern(&mut self, s: &str) -> Sym {
        if let Some(id) = self.ids.get(s) {
            return *id;
        }
        let id = self.names.len() as Sym;
        self.names.push(s.into());
        self.ids.insert(s.into(), id);
        id
    }

    fn get(&self, s: &str) -> Option<Sym> {
        self.ids.get(s).copied()
    }

    fn name(&self, id: Sym) -> &str {
        &self.names[id as usize]
    }
}

/// A property value, its text interned.
#[derive(Debug, Clone, PartialEq)]
enum Compact {
    Str(Sym),
    Other(CypherValue),
}

#[derive(Debug)]
struct Node {
    table: Sym,
    uuid: Sym,
    file: Option<Sym>,
    props: Vec<(Sym, Compact)>,
    alive: bool,
}

#[derive(Debug)]
struct Rel {
    rel: Sym,
    from: u32,
    to: u32,
    props: Vec<(Sym, Compact)>,
    alive: bool,
}

/// The image. See the module.
#[derive(Debug, Default)]
pub struct MemoryImage {
    strings: Interner,
    nodes: Vec<Node>,
    rels: Vec<Rel>,
    by_uuid: HashMap<Sym, u32>,
    by_file: HashMap<Sym, Vec<u32>>,
    by_table: HashMap<Sym, Vec<u32>>,
    outgoing: HashMap<(Sym, u32), Vec<u32>>,
    incoming: HashMap<(Sym, u32), Vec<u32>>,
    /// The relations touching each node, either end, by node id.
    touching: Vec<Vec<u32>>,
    /// `(table, field)` → value → nodes.
    indexes: HashMap<(Sym, Sym), HashMap<Sym, Vec<u32>>>,
    symbol_table: Option<(Sym, Sym)>,
}

impl MemoryImage {
    pub fn new() -> Self {
        Self::default()
    }

    /// The table of the symbols and the field of their name: [`remove_file`](Self::remove_file) gives back these
    /// names. Also indexes that field.
    pub fn set_symbol_table(&mut self, table: &str, name_field: &str) {
        self.index_field(table, name_field);
        self.symbol_table = Some((self.strings.intern(table), self.strings.intern(name_field)));
    }

    /// Index `table.field` (a text field) for [`find`](Self::find).
    pub fn index_field(&mut self, table: &str, field: &str) {
        let key = (self.strings.intern(table), self.strings.intern(field));
        if self.indexes.contains_key(&key) {
            return;
        }
        let mut index: HashMap<Sym, Vec<u32>> = HashMap::new();
        for id in self.by_table.get(&key.0).into_iter().flatten() {
            let node = &self.nodes[*id as usize];
            if let Some(Compact::Str(v)) = node.alive.then(|| prop(&node.props, key.1)).flatten() {
                index.entry(*v).or_default().push(*id);
            }
        }
        self.indexes.insert(key, index);
    }

    fn compact(&mut self, props: BTreeMap<String, CypherValue>) -> Vec<(Sym, Compact)> {
        props
            .into_iter()
            .map(|(k, v)| {
                let v = match v {
                    CypherValue::String(t) => Compact::Str(self.strings.intern(&t)),
                    other => Compact::Other(other),
                };
                (self.strings.intern(&k), v)
            })
            .collect()
    }

    fn expand(&self, props: &[(Sym, Compact)]) -> BTreeMap<String, CypherValue> {
        props
            .iter()
            .map(|(k, v)| {
                let v = match v {
                    Compact::Str(t) => CypherValue::String(self.strings.name(*t).to_string()),
                    Compact::Other(o) => o.clone(),
                };
                (self.strings.name(*k).to_string(), v)
            })
            .collect()
    }

    fn live(&self, uuid: &str) -> Option<u32> {
        let id = *self.by_uuid.get(&self.strings.get(uuid)?)?;
        self.nodes[id as usize].alive.then_some(id)
    }

    fn unindex(&mut self, id: u32) {
        let (table, props) = {
            let n = &self.nodes[id as usize];
            (n.table, n.props.clone())
        };
        for ((t, field), index) in self.indexes.iter_mut() {
            if *t != table {
                continue;
            }
            if let Some(Compact::Str(v)) = prop(&props, *field) {
                if let Some(ids) = index.get_mut(v) {
                    ids.retain(|x| *x != id);
                }
            }
        }
    }

    /// Insert a node, or replace the properties and file of the node of this uuid.
    pub fn insert_node(&mut self, table: &str, uuid: &str, file: Option<&str>, props: BTreeMap<String, CypherValue>) {
        let table = self.strings.intern(table);
        let uuid = self.strings.intern(uuid);
        let file = file.map(|f| self.strings.intern(f));
        let props = self.compact(props);
        let id = match self.by_uuid.get(&uuid).copied() {
            Some(id) => {
                self.unindex(id);
                let old_file = self.nodes[id as usize].file;
                if let Some(f) = old_file {
                    if let Some(ids) = self.by_file.get_mut(&f) {
                        ids.retain(|x| *x != id);
                    }
                }
                let was_alive = self.nodes[id as usize].alive;
                let old_table = self.nodes[id as usize].table;
                if was_alive && old_table != table {
                    if let Some(list) = self.by_table.get_mut(&old_table) {
                        list.retain(|x| *x != id);
                    }
                }
                let n = &mut self.nodes[id as usize];
                n.table = table;
                n.file = file;
                n.props = props;
                n.alive = true;
                if !was_alive || old_table != table {
                    self.by_table.entry(table).or_default().push(id);
                }
                id
            }
            None => {
                let id = self.nodes.len() as u32;
                self.nodes.push(Node { table, uuid, file, props, alive: true });
                self.touching.push(Vec::new());
                self.by_uuid.insert(uuid, id);
                self.by_table.entry(table).or_default().push(id);
                id
            }
        };
        if let Some(f) = file {
            self.by_file.entry(f).or_default().push(id);
        }
        for ((t, field), index) in self.indexes.iter_mut() {
            if *t != table {
                continue;
            }
            if let Some(Compact::Str(v)) = prop(&self.nodes[id as usize].props, *field) {
                index.entry(*v).or_default().push(id);
            }
        }
    }

    /// Insert a relation between two nodes of the image, as a MERGE: a pair `(rel, from, to)` already there takes
    /// the new properties (the last wins) and is not doubled. `false` when one end is missing.
    pub fn insert_rel(&mut self, rel: &str, from: &str, to: &str, props: BTreeMap<String, CypherValue>) -> bool {
        let (Some(from), Some(to)) = (self.live(from), self.live(to)) else { return false };
        let rel = self.strings.intern(rel);
        let props = self.compact(props);
        let existing = self
            .outgoing
            .get(&(rel, from))
            .and_then(|ids| ids.iter().copied().find(|r| self.rels[*r as usize].to == to));
        match existing {
            Some(r) => self.rels[r as usize].props = props,
            None => {
                let r = self.rels.len() as u32;
                self.rels.push(Rel { rel, from, to, props, alive: true });
                self.outgoing.entry((rel, from)).or_default().push(r);
                self.incoming.entry((rel, to)).or_default().push(r);
                self.touching[from as usize].push(r);
                if to != from {
                    self.touching[to as usize].push(r);
                }
            }
        }
        true
    }

    pub fn node(&self, uuid: &str) -> Option<NodeView> {
        let n = &self.nodes[self.live(uuid)? as usize];
        Some(NodeView {
            table: self.strings.name(n.table).to_string(),
            uuid: self.strings.name(n.uuid).to_string(),
            file: n.file.map(|f| self.strings.name(f).to_string()),
            props: self.expand(&n.props),
        })
    }

    fn follow(&self, map: &HashMap<(Sym, u32), Vec<u32>>, rel: &str, at: &str, out: bool) -> Vec<(String, BTreeMap<String, CypherValue>)> {
        let (Some(rel), Some(at)) = (self.strings.get(rel), self.live(at)) else { return Vec::new() };
        map.get(&(rel, at))
            .into_iter()
            .flatten()
            .map(|r| &self.rels[*r as usize])
            .filter(|r| r.alive)
            .map(|r| {
                let other = if out { r.to } else { r.from };
                (self.strings.name(self.nodes[other as usize].uuid).to_string(), self.expand(&r.props))
            })
            .collect()
    }

    /// The relations `rel` leaving `from`: the uuid reached and the properties of the relation.
    pub fn outgoing(&self, rel: &str, from: &str) -> Vec<(String, BTreeMap<String, CypherValue>)> {
        self.follow(&self.outgoing, rel, from, true)
    }

    /// The relations `rel` reaching `to`: the uuid it leaves and the properties of the relation.
    pub fn incoming(&self, rel: &str, to: &str) -> Vec<(String, BTreeMap<String, CypherValue>)> {
        self.follow(&self.incoming, rel, to, false)
    }

    fn uuids(&self, ids: impl IntoIterator<Item = u32>) -> Vec<String> {
        ids.into_iter()
            .filter(|id| self.nodes[*id as usize].alive)
            .map(|id| self.strings.name(self.nodes[id as usize].uuid).to_string())
            .collect()
    }

    /// The uuids of the nodes of `table` whose `field` is `value` (by its index when declared, else by a scan).
    pub fn find(&self, table: &str, field: &str, value: &CypherValue) -> Vec<String> {
        let (Some(t), Some(f)) = (self.strings.get(table), self.strings.get(field)) else { return Vec::new() };
        if let (Some(index), CypherValue::String(v)) = (self.indexes.get(&(t, f)), value) {
            return match self.strings.get(v) {
                Some(v) => self.uuids(index.get(&v).into_iter().flatten().copied()),
                None => Vec::new(),
            };
        }
        let wanted: Option<Compact> = match value {
            CypherValue::String(v) => match self.strings.get(v) {
                Some(v) => Some(Compact::Str(v)),
                None => return Vec::new(),
            },
            other => Some(Compact::Other(other.clone())),
        };
        self.uuids(
            self.by_table
                .get(&t)
                .into_iter()
                .flatten()
                .copied()
                .filter(|id| prop(&self.nodes[*id as usize].props, f) == wanted.as_ref()),
        )
    }

    /// The uuids of the nodes of `table` (all of them, in insertion order).
    pub fn nodes_of_table(&self, table: &str) -> Vec<String> {
        let Some(t) = self.strings.get(table) else { return Vec::new() };
        self.uuids(self.by_table.get(&t).into_iter().flatten().copied().filter(|id| self.nodes[*id as usize].table == t))
    }

    pub fn nodes_of_file(&self, path: &str) -> Vec<String> {
        let Some(f) = self.strings.get(path) else { return Vec::new() };
        self.uuids(self.by_file.get(&f).into_iter().flatten().copied())
    }

    /// Take relations out (alive ones; duplicates allowed): from both indexes and from the nodes they touch. Gives
    /// back how many, and the names of the symbols at their ends.
    fn drop_rels(&mut self, mut rels: Vec<u32>) -> (usize, BTreeSet<String>) {
        rels.retain(|r| self.rels[*r as usize].alive);
        rels.sort_unstable();
        rels.dedup();
        let mut names = BTreeSet::new();
        for r in &rels {
            let (rel, from, to) = (self.rels[*r as usize].rel, self.rels[*r as usize].from, self.rels[*r as usize].to);
            for end in [from, to] {
                let n = &self.nodes[end as usize];
                if let Some((table, field)) = self.symbol_table {
                    if n.table == table {
                        if let Some(Compact::Str(name)) = prop(&n.props, field) {
                            names.insert(self.strings.name(*name).to_string());
                        }
                    }
                }
            }
            self.rels[*r as usize].alive = false;
            if let Some(list) = self.outgoing.get_mut(&(rel, from)) {
                list.retain(|x| x != r);
            }
            if let Some(list) = self.incoming.get_mut(&(rel, to)) {
                list.retain(|x| x != r);
            }
            for end in [from, to] {
                self.touching[end as usize].retain(|x| x != r);
            }
        }
        (rels.len(), names)
    }

    /// Take a node out of its indexes (its relations already gone).
    fn drop_node(&mut self, id: u32) {
        self.unindex(id);
        self.nodes[id as usize].alive = false;
        let (table, file) = (self.nodes[id as usize].table, self.nodes[id as usize].file);
        if let Some(list) = self.by_table.get_mut(&table) {
            list.retain(|x| *x != id);
        }
        if let Some(f) = file {
            if let Some(list) = self.by_file.get_mut(&f) {
                list.retain(|x| *x != id);
            }
        }
    }

    /// Take `path` out: its nodes and all their relations, both directions.
    pub fn remove_file(&mut self, path: &str) -> RemovedFile {
        let mut removed = RemovedFile::default();
        let Some(f) = self.strings.get(path) else { return removed };
        let ids: Vec<u32> = self.by_file.get(&f).cloned().unwrap_or_default().into_iter().filter(|id| self.nodes[*id as usize].alive).collect();
        let rels: Vec<u32> = ids.iter().flat_map(|id| self.touching[*id as usize].clone()).collect();
        let (n, names) = self.drop_rels(rels);
        for id in &ids {
            self.drop_node(*id);
        }
        removed.nodes = ids.len();
        removed.relations = n;
        removed.symbol_names = names;
        removed
    }

    /// Take one node out, with all its relations, both directions (a `DETACH DELETE`). `false` when it is absent.
    pub fn remove_node(&mut self, uuid: &str) -> bool {
        let Some(id) = self.live(uuid) else { return false };
        let rels = self.touching[id as usize].clone();
        self.drop_rels(rels);
        self.drop_node(id);
        true
    }

    /// Take one relation out. `false` when it is absent.
    pub fn remove_rel(&mut self, rel: &str, from: &str, to: &str) -> bool {
        let (Some(rel), Some(from), Some(to)) = (self.strings.get(rel), self.live(from), self.live(to)) else { return false };
        let found = self.outgoing.get(&(rel, from)).and_then(|ids| ids.iter().copied().find(|r| self.rels[*r as usize].to == to));
        match found {
            Some(r) => self.drop_rels(vec![r]).0 == 1,
            None => false,
        }
    }

    /// The live nodes and relations.
    pub fn counts(&self) -> (usize, usize) {
        (self.nodes.iter().filter(|n| n.alive).count(), self.rels.iter().filter(|r| r.alive).count())
    }

    /// The interned strings, to see that the image does not repeat them.
    pub fn interned_strings(&self) -> usize {
        self.strings.names.len()
    }
}

fn prop(props: &[(Sym, Compact)], key: Sym) -> Option<&Compact> {
    props.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
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
    fn removing_a_node_takes_its_relations_both_ways() {
        let mut m = image();
        assert!(m.remove_node("s-b"));
        assert!(m.node("s-b").is_none());
        assert!(m.outgoing("CONSUMES", "s-a").is_empty(), "the call into s-b is gone");
        assert!(m.outgoing("DEFINES", "y-beta").is_empty(), "the definition of beta is gone");
        assert_eq!(m.counts(), (2, 1), "s-a, y-beta and the mention stay");
        assert!(m.nodes_of_file("src/b.rs").is_empty());
        assert!(!m.remove_node("s-b"), "a second removal finds nothing");
    }

    #[test]
    fn removing_a_relation_leaves_its_ends() {
        let mut m = image();
        assert!(m.remove_rel("CONSUMES", "s-a", "s-b"));
        assert!(m.outgoing("CONSUMES", "s-a").is_empty());
        assert!(m.incoming("CONSUMES", "s-b").is_empty());
        assert!(m.node("s-a").is_some() && m.node("s-b").is_some());
        assert_eq!(m.counts(), (3, 2));
        assert!(!m.remove_rel("CONSUMES", "s-a", "s-b"), "a second removal finds nothing");
        assert!(m.insert_rel("CONSUMES", "s-a", "s-b", BTreeMap::new()), "the pair can be linked again");
        assert_eq!(m.outgoing("CONSUMES", "s-a").len(), 1);
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
