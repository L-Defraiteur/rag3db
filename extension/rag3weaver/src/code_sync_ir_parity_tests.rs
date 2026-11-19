//! **The writer by the IR, at parity by result** (2.11, lot A; c7, 19 November 2026). Each raw Cypher of the
//! generations writer is kept here as it was, and run beside the IR form that replaces it, on the same base
//! with history (closed rows, a published generation): the same rows for a read, with the same kind of plan
//! (`EXPLAIN`); the same state for a write, generations and their counts included. The rendered texts are
//! printed for the review, never compared.

use super::*;
use crate::code_tools::{edit_file, EditOp, Snapshot};

/// A code catalogue with generations and history: three files indexed, the generation published, then an edit
/// that closes `beta` and opens `gamma` at the pending rank.
pub(super) fn base_with_history() -> (Catalog, Snapshot) {
    let conn = crate::Rag3dbConnection::in_memory().expect("in-memory base");
    let config = crate::CatalogConfig { name: Some("ir-parity".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(crate::embedder::MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    crate::code::register_code_schema(&mut catalog, crate::code::default_scope_chunking()).unwrap();
    let snapshot = Snapshot::new(
        "depot",
        [
            ("a.rs".to_string(), "pub fn alpha() {}\npub struct Point;\nimpl Point { pub fn norme(&self) {} }\n".to_string()),
            ("b.rs".to_string(), "pub fn beta() { alpha(); }\npub fn beta_bis() {}\n".to_string()),
            ("c.rs".to_string(), "pub fn gamma() { alpha(); beta(); }\n".to_string()),
        ],
    );
    sync_source(&mut catalog, &snapshot, &SourceSyncOptions::default(), &mut |_| {}).expect("the first index");
    catalog.publier_la_generation("c1", 0).expect("publish");
    edit_file(&snapshot, Some(&mut catalog), "b.rs", &EditOp::Replace { old: "pub fn beta()".into(), new: "pub fn delta()".into() }).expect("edit");
    (catalog, snapshot)
}

/// The source id of the base's files (one source here).
pub(super) fn source_of(catalog: &Catalog) -> String {
    let lu = catalog.execute_raw(&format!("MATCH (f:{FILE}) RETURN DISTINCT f.source")).unwrap();
    lu.rows.first().and_then(|l| l.first()).and_then(|v| v.as_str()).expect("one source").to_string()
}

/// The operators of a plan, in order: the kind of each step, not its estimates. Two reads of the same kind
/// scan, filter and join alike.
pub(super) fn operators(plan: &str) -> Vec<String> {
    let mut v = Vec::new();
    for mot in plan.split(|c: char| !(c.is_ascii_uppercase() || c == '_')) {
        if mot.len() >= 4 && mot.chars().any(|c| c.is_ascii_uppercase()) {
            v.push(mot.to_string());
        }
    }
    v
}

/// The rows of a query, sorted: the parity of a read does not depend on an order it does not ask for.
pub(super) fn rows(catalog: &Catalog, query: &str, params: &[crate::connection::QueryParam]) -> Vec<String> {
    let lu = catalog.execute_raw_with_params(query, params).unwrap_or_else(|e| panic!("{query}: {e}"));
    let mut v: Vec<String> = lu.rows.iter().map(|l| format!("{l:?}")).collect();
    v.sort();
    v
}

/// The plan of a query, by `EXPLAIN`.
pub(super) fn plan(catalog: &Catalog, query: &str, params: &[crate::connection::QueryParam]) -> String {
    let lu = catalog.execute_raw_with_params(&format!("EXPLAIN {query}"), params).unwrap_or_else(|e| panic!("EXPLAIN {query}: {e}"));
    lu.rows.iter().map(|l| format!("{l:?}")).collect::<Vec<_>>().join("\n")
}

/// The access steps of a plan: how the rows are reached (scans, lookups, joins), not the filters on them.
pub(super) fn access(plan: &str) -> Vec<String> {
    operators(plan).into_iter().filter(|o| ["SCAN", "LOOKUP", "JOIN", "INDEX", "EXTEND", "UNWIND"].iter().any(|k| o.contains(k))).collect()
}

/// **A read at parity**: the same rows, the same access steps. Both texts are printed for the review.
pub(super) fn same_read(catalog: &Catalog, site: &str, before: &str, after: &str, params: &[crate::connection::QueryParam]) {
    eprintln!("[{site}] before: {before}\n[{site}] after:  {after}");
    let (lignes_avant, lignes_apres) = (rows(catalog, before, params), rows(catalog, after, params));
    assert!(!lignes_avant.is_empty(), "[{site}] the witness base gives the old read something to return");
    assert_eq!(lignes_apres, lignes_avant, "[{site}] the same rows");
    let (avant, apres) = (access(&plan(catalog, before, params)), access(&plan(catalog, after, params)));
    assert_eq!(apres, avant, "[{site}] the same access steps");
}

fn param(name: &str, value: CypherValue) -> crate::connection::QueryParam {
    crate::connection::QueryParam::new(name, value)
}

fn text(s: &str) -> CypherValue {
    CypherValue::String(s.to_string())
}

/// `synchroniser_par_generations`: the files of a source visible at the pending rank.
#[test]
fn the_indexed_files_read_by_the_ir_at_parity() {
    let (catalog, _) = base_with_history();
    let v = catalog.generations.clone().unwrap();
    for rang in [v.rang, v.rang - 1] {
        let visible = catalog.dialect_arc().condition_visible_sur("f", &v.couche, rang).unwrap();
        let before = format!("MATCH (f:{FILE}) WHERE f.source = $src AND f._couche = $couche AND {visible} RETURN f.path, f.content_hash, f.analyse");
        let after = indexed_files_read(&catalog, rang).unwrap();
        same_read(&catalog, "indexed files", &before, &after, &[param("src", text(&source_of(&catalog))), param("couche", text(&v.couche))]);
    }
}

/// `lignes_visibles_des_fichiers`: the rows of given paths, `File` by `path` and `Scope` by `file_path`.
#[test]
fn the_rows_of_paths_read_by_the_ir_at_parity() {
    let (catalog, _) = base_with_history();
    let chemins = CypherValue::List(vec![text("a.rs"), text("b.rs")]);
    for (table, champ) in [(FILE, "path"), (SCOPE, "file_path")] {
        let visible = catalog.condition_visible("x").map(|c| format!(" AND {c}")).unwrap_or_default();
        let before = format!("MATCH (x:`{table}`) WHERE x.source = $src AND x.`{champ}` IN $chemins{visible} RETURN x._uuid ORDER BY x._uuid");
        let after = rows_of_paths_read(&catalog, table, champ).unwrap();
        same_read(&catalog, &format!("rows of paths, {table}"), &before, &after, &[param("src", text(&source_of(&catalog))), param("chemins", chemins.clone())]);
    }
}

/// `decide_methods_defined_elsewhere`: the visible classes of a source named among given names.
#[test]
fn the_visible_classes_read_by_the_ir_at_parity() {
    let (catalog, _) = base_with_history();
    let v = catalog.generations.clone().unwrap();
    let visible = catalog.dialect_arc().condition_visible_sur("s", &v.couche, v.rang).unwrap();
    let before = format!("MATCH (s:`{SCOPE}`) WHERE s.source = $src AND s.scope_type = 'class' AND s.name IN $names AND s._couche = $couche AND ({visible}) RETURN s.name, s.key, s.file_path");
    let after = visible_classes_read(&catalog).unwrap();
    let params = [
        param("src", text(&source_of(&catalog))),
        param("classe", text("class")),
        param("names", CypherValue::List(vec![text("Point"), text("Absent")])),
        param("couche", text(&v.couche)),
    ];
    same_read(&catalog, "visible classes", &before, &after, &params);
}

/// `ecrire_la_table`: the whole rows of given paths, visible at the pending rank or retired at it.
#[test]
fn the_rows_to_compare_read_by_the_ir_at_parity() {
    let (catalog, _) = base_with_history();
    let v = catalog.generations.clone().unwrap();
    let visible = catalog.dialect_arc().condition_visible_sur("s", &v.couche, v.rang).unwrap();
    for (table, champ) in [(SCOPE, "file_path"), (FILE, "path")] {
        let before = format!("MATCH (s:`{table}`) WHERE s.source = $src AND s.`{champ}` IN $chemins AND s._couche = $couche AND (({visible}) OR s._jusqu_a = $w) RETURN s");
        let after = rows_to_compare_read(&catalog, table, champ).unwrap();
        let params = [
            param("src", text(&source_of(&catalog))),
            param("chemins", CypherValue::List(vec![text("a.rs"), text("b.rs")])),
            param("couche", text(&v.couche)),
            param("w", CypherValue::Int(v.rang)),
        ];
        same_read(&catalog, &format!("rows to compare, {table}"), &before, &after, &params);
    }
}

/// `completer_les_naissances`: the key and birth rank of visible rows among given keys.
#[test]
fn the_births_read_by_the_ir_at_parity() {
    let (catalog, _) = base_with_history();
    let v = catalog.generations.clone().unwrap();
    let visible = catalog.dialect_arc().condition_visible_sur("s", &v.couche, v.rang).unwrap();
    let cles_de = |table: &str, champ: &str| -> CypherValue {
        let lu = catalog.execute_raw(&format!("MATCH (s:`{table}`) RETURN DISTINCT s.`{champ}`")).unwrap();
        CypherValue::List(lu.rows.iter().filter_map(|l| l.first().cloned()).collect())
    };
    for (table, champ) in [(SCOPE, "key"), (FILE, "path")] {
        let before = format!("MATCH (s:`{table}`) WHERE s.`{champ}` IN $cles AND s._couche = $couche AND {visible} RETURN s.`{champ}`, s.{}", crate::generations::DEPUIS);
        let after = births_read(&catalog, table, champ).unwrap();
        same_read(&catalog, &format!("births, {table}"), &before, &after, &[param("cles", cles_de(table, champ)), param("couche", text(&v.couche))]);
    }
}
