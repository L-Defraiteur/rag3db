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
    // `alpha` replaced in place: same key, a new occurrence; `gamma` (c.rs) still consumes it from outside.
    edit_file(&snapshot, Some(&mut catalog), "a.rs", &EditOp::Replace { old: "pub fn alpha() {}".into(), new: "pub fn alpha() { let _x = 1; }".into() }).expect("edit");
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

/// An edge row without its internal ids (`_id`, `_src`, `_dst`): positions in the store, not data.
fn strip_internal_ids(row: &str) -> String {
    let mut s = row.to_string();
    for cle in ["_id", "_src", "_dst"] {
        while let Some(i) = s.find(&format!("\"{cle}\": ")) {
            let fin = s[i..].find("), ").map(|j| i + j + 3).or_else(|| s[i..].find(')').map(|j| i + j + 1)).unwrap_or(s.len());
            s.replace_range(i..fin, "");
        }
    }
    s
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

/// **The state a write leaves**: every row of the generations tables with its rank fields, every edge between
/// them, and the generations with their counts. Sorted: two equal states compare equal.
pub(super) fn state(catalog: &Catalog) -> Vec<String> {
    let mut v = Vec::new();
    for (table, nom) in [(SCOPE, "key"), (FILE, "path")] {
        v.extend(rows(catalog, &format!("MATCH (s:`{table}`) RETURN '{table}', s._uuid, s.`{nom}`, s._couche, s._depuis, s._jusqu_a"), &[]));
    }
    // Each edge with its properties: a copy carries those of the edge it copies.
    v.extend(rows(catalog, "MATCH (a)-[r]->(b) RETURN label(r), a._uuid, b._uuid, r", &[]).into_iter().map(|l| strip_internal_ids(&l)));
    // Every field of a generation but its publication time (a clock, different between two bases).
    v.extend(rows(
        catalog,
        &format!(
            "MATCH (g:{}) RETURN g._key, g.couche, g.rang, g.commit, g.base, g.recette, g.parente_couche, g.parente_rang, g.tombstones, g.chemins_sales, g.aretes_recopiees",
            crate::generations::TABLE
        ),
        &[],
    ));
    v
}

/// **A write at parity**: the same write by its raw Cypher on one base and by its IR form on another, both built
/// the same way; then the same state. Both texts are printed for the review.
pub(super) fn same_write(site: &str, before: &str, after: &str, params: impl Fn(&Catalog) -> Vec<crate::connection::QueryParam>) {
    eprintln!("[{site}] before: {before}\n[{site}] after:  {after}");
    let (avant, _) = base_with_history();
    let (apres, _) = base_with_history();
    assert_eq!(state(&apres), state(&avant), "[{site}] the two bases start equal");
    let depart = state(&avant);
    avant.execute_raw_with_params(before, &params(&avant)).unwrap_or_else(|e| panic!("[{site}] {before}: {e}"));
    apres.execute_raw_with_params(after, &params(&apres)).unwrap_or_else(|e| panic!("[{site}] {after}: {e}"));
    assert_ne!(state(&avant), depart, "[{site}] the old write changes something on the witness base");
    assert_eq!(state(&apres), state(&avant), "[{site}] the same state");
}

/// [`same_write`] for a write whose setup is done by `params` itself, on both bases alike; `changes`: whether
/// the old write is expected to change something.
pub(super) fn same_write_from(site: &str, before: &str, after: &str, params: &dyn Fn(&Catalog) -> Vec<crate::connection::QueryParam>, changes: bool) {
    eprintln!("[{site}] before: {before}\n[{site}] after:  {after}");
    let (avant, _) = base_with_history();
    let (apres, _) = base_with_history();
    let (pa, pb) = (params(&avant), params(&apres));
    assert_eq!(state(&apres), state(&avant), "[{site}] the two bases start equal");
    let depart = state(&avant);
    let na = avant.execute_raw_with_params(before, &pa).unwrap_or_else(|e| panic!("[{site}] {before}: {e}"));
    let nb = apres.execute_raw_with_params(after, &pb).unwrap_or_else(|e| panic!("[{site}] {after}: {e}"));
    let trie = |r: &crate::connection::QueryResult| {
        let mut v: Vec<String> = r.rows.iter().map(|l| format!("{l:?}")).collect();
        v.sort();
        v
    };
    assert_eq!(trie(&nb), trie(&na), "[{site}] the same rows returned");
    if changes {
        assert_ne!(state(&avant), depart, "[{site}] the old write changes something on the witness base");
    }
    assert_eq!(state(&apres), state(&avant), "[{site}] the same state");
}

/// `ecrire_la_table`: the retirement rank of given rows, set then reopened with a typed NULL.
#[test]
fn the_retirement_write_by_the_ir_at_parity() {
    for table in [SCOPE, FILE] {
        let before = format!("UNWIND $uuids AS u MATCH (s:`{table}` {{_uuid: u}}) SET s.{} = $valeur", crate::generations::JUSQU_A);
        let (catalog, _) = base_with_history();
        let after = retirement_write(&catalog, table).unwrap();
        let rang = catalog.generations.clone().unwrap().rang;
        let vivantes = move |c: &Catalog| -> CypherValue {
            let lu = c.execute_raw(&format!("MATCH (s:`{table}`) WHERE s._jusqu_a IS NULL RETURN s._uuid ORDER BY s._uuid LIMIT 2")).unwrap();
            CypherValue::List(lu.rows.iter().filter_map(|l| l.first().cloned()).collect())
        };
        same_write(&format!("retire, {table}"), &before, &after, |c| vec![param("uuids", vivantes(c)), param("valeur", CypherValue::Int(rang))]);
        let fermees = move |c: &Catalog| -> CypherValue {
            let lu = c.execute_raw(&format!("MATCH (s:`{table}`) WHERE s._jusqu_a IS NOT NULL RETURN s._uuid ORDER BY s._uuid")).unwrap();
            CypherValue::List(lu.rows.iter().filter_map(|l| l.first().cloned()).collect())
        };
        let null_entier = || CypherValue::Typed { value: Box::new(CypherValue::Null), field_type: crate::config::FieldType::Int64 };
        if table == SCOPE {
            same_write("reopen, Scope", &before, &after, |c| vec![param("uuids", fermees(c)), param("valeur", null_entier())]);
        }
    }
}

/// The pairs (replaced row, its replacement) of `Scope` at the pending rank, as `$paires` maps.
fn replacement_pairs(catalog: &Catalog) -> (CypherValue, CypherValue) {
    let lu = catalog
        .execute_raw(&format!("MATCH (o:`{SCOPE}`), (n:`{SCOPE}`) WHERE o.key = n.key AND o._jusqu_a IS NOT NULL AND n._jusqu_a IS NULL AND n._depuis = o._jusqu_a RETURN o._uuid, n._uuid"))
        .unwrap();
    let paires = lu.rows.iter().map(|l| CypherValue::Map(BTreeMap::from([("a".to_string(), l[0].clone()), ("n".to_string(), l[1].clone())]))).collect();
    let neuves = lu.rows.iter().map(|l| l[1].clone()).collect();
    (CypherValue::List(paires), CypherValue::List(neuves))
}

/// `recopier_les_aretes_entrantes`: the edges of the replaced rows copied to their replacements, those that
/// arrive (each relation into `Scope`) and the `CONSUMED_BY` mirror that leaves; outside the batch (`a.rs`).
#[test]
fn the_edge_copies_by_the_ir_at_parity() {
    let (catalog, _) = base_with_history();
    let v = catalog.generations.clone().unwrap();
    let (paires, _) = replacement_pairs(&catalog);
    assert!(matches!(&paires, CypherValue::List(l) if !l.is_empty()), "the witness base replaces a row in place");
    let source = source_of(&catalog);
    let hors_du_lot = "NOT (x.source = $lot_s0 AND x.file_path IN $lot_p0)";
    // The old copy of the relation's properties, as the writer wrote it.
    let copie_de = |rel: &str| -> String {
        let props: Vec<String> = catalog.get_relation_def(rel).and_then(|r| r.properties.as_ref()).map(|m| m.keys().cloned().collect()).unwrap_or_default();
        if props.is_empty() {
            String::new()
        } else {
            format!(" {{{}}}", props.iter().map(|k| format!("`{k}`: r.`{k}`")).collect::<Vec<_>>().join(", "))
        }
    };
    let mut cas: Vec<(String, String, String, rag3weaver_ir::Direction, String)> = Vec::new();
    for (rel, from, to) in crate::code::RELATIONS {
        if to != SCOPE || rel == "CONSUMED_BY" {
            continue;
        }
        let visible = if v.tables.contains(from) { catalog.dialect_arc().condition_visible_sur("x", &v.couche, v.rang) } else { None };
        let mut clauses: Vec<String> = visible.into_iter().collect();
        if from == SCOPE {
            clauses.push(hors_du_lot.to_string());
        }
        clauses.push(format!("NOT EXISTS {{ MATCH (x)-[:`{rel}`]->(n) }}"));
        let before = format!("UNWIND $paires AS p MATCH (x:`{from}`)-[r:`{rel}`]->(o:`{to}` {{_uuid: p.a}}), (n:`{to}` {{_uuid: p.n}}) WHERE {} CREATE (x)-[:`{rel}`{}]->(n) RETURN count(*)", clauses.join(" AND "), copie_de(rel));
        let after = edge_copy_write(&catalog, rel, to, from, rag3weaver_ir::Direction::Incoming, v.tables.contains(from).then_some(v.rang), usize::from(from == SCOPE)).unwrap();
        cas.push((rel.to_string(), before, after, rag3weaver_ir::Direction::Incoming, format!("MATCH (x)-[r:`{rel}`]->(n:`{SCOPE}`) WHERE n._uuid IN $neuves DELETE r")));
    }
    let visible = catalog.dialect_arc().condition_visible_sur("x", &v.couche, v.rang).unwrap();
    let before = format!("UNWIND $paires AS p MATCH (o:`{SCOPE}` {{_uuid: p.a}})-[r:CONSUMED_BY]->(x:`{SCOPE}`), (n:`{SCOPE}` {{_uuid: p.n}}) WHERE {visible} AND {hors_du_lot} AND NOT EXISTS {{ MATCH (n)-[:CONSUMED_BY]->(x) }} CREATE (n)-[:CONSUMED_BY{}]->(x) RETURN count(*)", copie_de("CONSUMED_BY"));
    let after = edge_copy_write(&catalog, "CONSUMED_BY", SCOPE, SCOPE, rag3weaver_ir::Direction::Outgoing, Some(v.rang), 1).unwrap();
    cas.push(("CONSUMED_BY".to_string(), before, after, rag3weaver_ir::Direction::Outgoing, format!("MATCH (n:`{SCOPE}`)-[r:CONSUMED_BY]->(x) WHERE n._uuid IN $neuves DELETE r")));
    let mut copiees = 0;
    for (rel, before, after, _, retirer) in cas {
        // The copies the writer already made, undone alike on both bases: the copy has something to do.
        let params = |c: &Catalog| {
            let (paires, neuves) = replacement_pairs(c);
            c.execute_raw_with_params(&retirer, &[param("neuves", neuves)]).unwrap();
            vec![param("paires", paires), param("lot_s0", text(&source)), param("lot_p0", CypherValue::List(vec![text("a.rs")]))]
        };
        let (avant, _) = base_with_history();
        let depart = state(&avant);
        let p = params(&avant);
        let lu = avant.execute_raw_with_params(&before, &p).unwrap();
        let n = lu.rows.first().and_then(|l| l.first()).and_then(|x| x.as_i64()).unwrap_or(0);
        copiees += n;
        let _ = depart;
        same_write_from(&format!("copy {rel}"), &before, &after, &params, n > 0);
    }
    assert!(copiees > 0, "the witness base has edges to copy");
}

/// `forget_edges_of_files`: the edges of the scopes of given files undone, only those whose other end is visible
/// at the pending rank, returning the undone pairs; each relation that leaves a scope, and `CONSUMED_BY` that
/// arrives.
#[test]
fn the_detaches_by_the_ir_at_parity() {
    use rag3weaver_ir::{Direction, Predicate};
    let (catalog, _) = base_with_history();
    let v = catalog.generations.clone().unwrap();
    let filtre = Predicate::AllOf(vec![
        Predicate::Equals { field: "source".into(), param: "source".into() },
        Predicate::In { field: "file_path".into(), param: "paths".into() },
        Predicate::Visible { couche: v.couche.clone(), rang: v.rang, mask: Default::default() },
    ]);
    let mut cas: Vec<(&str, Direction, &str)> = crate::code::RELATIONS.iter().filter(|(r, f, _)| *f == SCOPE && *r != "CONSUMED_BY").map(|(r, _, t)| (*r, Direction::Outgoing, *t)).collect();
    cas.push(("CONSUMED_BY", Direction::Incoming, SCOPE));
    let mut defaites = 0;
    for (rel, side, autre) in cas {
        let d = catalog.dialect_arc();
        // The old text: the detach rendered without its other end, then the clause pasted before ` DELETE r`.
        let sans_bout = d.detach_edges_returning(rel, SCOPE, side, &filtre, None).unwrap();
        let before = match v.tables.contains(autre).then(|| d.condition_visible_sur("o", &v.couche, v.rang)).flatten() {
            Some(c) => sans_bout.replacen(" DELETE r", &format!(" AND ({c}) DELETE r"), 1),
            None => sans_bout,
        };
        let bout = edge_end_visible(catalog.generations.as_ref(), autre);
        let after = d.detach_edges_returning(rel, SCOPE, side, &filtre, bout.as_ref().map(|(t, p)| (t.as_str(), p))).unwrap();
        let params = |c: &Catalog| vec![param("source", text(&source_of(c))), param("paths", CypherValue::List(vec![text("a.rs"), text("c.rs")]))];
        let (essai, _) = base_with_history();
        defaites += essai.execute_raw_with_params(&before, &params(&essai)).unwrap().rows.len();
        same_write_from(&format!("detach {rel}"), &before, &after, &params, false);
    }
    assert!(defaites > 0, "the witness base has edges to undo");
}

/// `Catalog::poser_les_generations`: one row not visible at the rank after the pending one (a base with a history).
#[test]
fn the_hidden_row_read_by_the_ir_at_parity() {
    let (catalog, _) = base_with_history();
    let v = catalog.generations.clone().unwrap();
    for rang in [v.rang, v.rang + 1] {
        for table in [SCOPE, FILE] {
            let visible = catalog.dialect_arc().condition_visible_sur("s", &v.couche, rang).unwrap();
            let before = format!("MATCH (s:`{table}`) WHERE NOT ({visible}) RETURN s._uuid LIMIT 1");
            let after = catalog.hidden_row_read(table, &v.couche, rang).unwrap().unwrap();
            // A LIMIT 1 without order picks any one row: the parity is on its existence and on the plan.
            let (a, b) = (rows(&catalog, &before, &[]).len(), rows(&catalog, &after, &[]).len());
            eprintln!("[hidden row, {table}, {rang}] before: {before}\n[hidden row, {table}, {rang}] after:  {after}");
            assert_eq!(b, a, "[hidden row, {table}] the same existence");
            assert_eq!(access(&plan(&catalog, &after, &[])), access(&plan(&catalog, &before, &[])), "[hidden row, {table}] the same access steps");
        }
    }
    assert_eq!(rows(&catalog, &catalog.hidden_row_read(SCOPE, &v.couche, v.rang + 1).unwrap().unwrap(), &[]).len(), 1, "the witness base has a history");
}

/// `Catalog::publier_la_generation`: the counts of the published generation.
#[test]
fn the_generation_counts_by_the_dialect_at_parity() {
    let before = format!("MATCH (g:{} {{_key: $key}}) SET g.chemins_sales = $sales, g.aretes_recopiees = $recopiees", crate::generations::TABLE);
    let (catalog, _) = base_with_history();
    let after = catalog.dialect_arc().set_generation_counts().unwrap();
    let cle = |c: &Catalog| {
        let lu = c.execute_raw(&format!("MATCH (g:{}) RETURN g._key ORDER BY g._key LIMIT 1", crate::generations::TABLE)).unwrap();
        lu.rows[0][0].clone()
    };
    same_write("generation counts", &before, &after, |c| vec![param("key", cle(c)), param("sales", CypherValue::Int(7)), param("recopiees", CypherValue::Int(3))]);
}
