//! Même API publique, mêmes données : seule la forme initiale diffère.
use std::cell::RefCell;
use std::collections::BTreeSet;
use super::*;
use crate::code::{default_scope_chunking, register_code_schema, scope_chunks, SCOPE_FORM_GENERATIONS, SCOPE_FORM_KEY, SCOPE_FORM_VERSIONS};
use crate::code_tools::Snapshot;
use crate::connection::DbConnection;
use crate::embedder::HashEmbedder;
use crate::fts_handle::{index_document, FtsStorage};
use crate::{CatalogConfig, Rag3dbConnection};
thread_local! {
    static TRACE: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}
pub(super) fn record(event: String) {
    TRACE.with(|t| { if let Some(events) = t.borrow_mut().as_mut() { events.push(event); } });
}
struct Capture;
impl Capture {
    fn start() -> Self {
        TRACE.with(|t| assert!(t.borrow_mut().replace(Vec::new()).is_none()));
        Self
    }
    fn take(&self) -> Vec<String> {
        TRACE.with(|t| std::mem::take(t.borrow_mut().as_mut().unwrap()))
    }
}
impl Drop for Capture {
    fn drop(&mut self) { TRACE.with(|t| *t.borrow_mut() = None); }
}
fn graph_ids(c: &Catalog, table: &str) -> Vec<u64> {
    let sql = format!("MATCH (n:`{table}`) WHERE n._org = 'default' AND n._project = 'default' RETURN OFFSET(id(n)) ORDER BY OFFSET(id(n))");
    let ids: Vec<u64> = c.conn().execute(&sql).expect("offsets physiques").rows.iter()
        .map(|row| u64::try_from(row[0].as_i64().expect("offset entier")).expect("offset positif")).collect();
    assert!(!ids.is_empty());
    assert_eq!(ids.iter().copied().collect::<BTreeSet<_>>().len(), ids.len());
    ids
}
fn healthy_rows(c: &Catalog, table: &str) -> Vec<Vec<CypherValue>> {
    let mut rows = c.conn().execute("MATCH (n:Scope) RETURN n._uuid, n.key, n.file_path, n.start_byte, n.end_byte, n.start_line, n.end_line ORDER BY n._uuid").expect("UUID et positions").rows;
    assert!(!rows.is_empty());
    for row in &rows { assert!(row[0].as_str().is_some_and(|v| !v.is_empty())); }
    let contents = c.conn().execute(&format!("MATCH (n:`{table}`) RETURN n._uuid, n._content_hash, n.content ORDER BY n._uuid")).expect("H et contenu physiques").rows;
    assert!(!contents.is_empty());
    for row in &contents {
        assert!(row[0].as_str().is_some_and(|v| !v.is_empty()));
        assert!(row[1].as_str().is_some_and(|v| !v.is_empty()));
    }
    rows.extend(contents);
    rows
}
fn sync_and_trace(c: &mut Catalog, src: &Snapshot, capture: &Capture, stage: &str) -> Vec<String> {
    let options = SourceSyncOptions { batch_files: 32, relations: Some(RelationsMode::Bulk), exige: crate::disponibilite::Disponibilites::RECHERCHE_TEXTE, force: true, ..Default::default() };
    let result = sync_source(c, src, &options, &mut |_| {});
    let trace = capture.take();
    eprintln!("FTS_DOUBLE_TRACE {stage} {trace:?} result={result:?}");
    let report = result.expect("API publique sync_source");
    assert_eq!(report.failed, 0, "{report:?}");
    trace
}
fn fixture(generations: bool, label: &str) -> (Catalog, Snapshot) {
    // La recette doit fournir une racine privée disque ; aucune utilisation implicite de /tmp.
    let root = std::path::PathBuf::from(std::env::var_os("RAG3WEAVER_FTS_DOUBLE_ROOT").expect("racine privée DOUBLE"));
    assert!(root.is_absolute() && root.is_dir() && !root.is_symlink());
    let case = root.join(label);
    std::fs::create_dir(&case).expect("fixture neuve, aucun réemploi");
    let conn = Rag3dbConnection::new(case.join("base.rag3db")).expect("base neuve");
    let extension = crate::backend::vector_extension_of_this_process().expect("extension du SDK courant");
    conn.execute(&format!("LOAD EXTENSION '{}'", extension.display())).expect("extension vectorielle");
    let mut c = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(16)), CatalogConfig { name: Some("fts-double".into()), embedding_dim: 16, allow_mock_embedder: true, ..Default::default() });
    c.set_fts_storage(FtsStorage::BlobBacked { lazy: false });
    c.initialize().expect("initialiser");
    let form = if generations { SCOPE_FORM_GENERATIONS } else { SCOPE_FORM_VERSIONS };
    c.persist_meta_key(SCOPE_FORM_KEY, form).expect("forme AVANT schéma/donnée de code");
    register_code_schema(&mut c, default_scope_chunking()).expect("schéma");
    assert_eq!(c.generations.is_some(), generations);
    let src = Snapshot::new("fts-double", [("src/a.rs".to_string(), "pub fn alpha() -> u32 { 7 }\n".to_string())]);
    (c, src)
}
fn run_case(generations: bool) {
    let (mut c, src) = fixture(generations, if generations { "generations" } else { "classic" });
    let form = if generations { SCOPE_FORM_GENERATIONS } else { SCOPE_FORM_VERSIONS };
    let capture = Capture::start();
    let initial = sync_and_trace(&mut c, &src, &capture, "initial");
    assert_eq!(initial.first().map(String::as_str), Some("dispatch:classic"));
    let table = scope_chunks(&c).expect("table physique").parent;
    assert_eq!(table, "ScopeContent");
    let graph_before = graph_ids(&c, &table);
    let (fts_before, token_before) = c.fts_live_ids(&table).expect("FTS physique");
    assert_eq!(fts_before, graph_before, "égalité initiale d'ensembles");
    let healthy_before = healthy_rows(&c, &table);
    const GHOST: u64 = 9_000_000;
    assert!(!graph_before.contains(&GHOST) && !fts_before.contains(&GHOST));
    {
        let h = c.fts_handle(&table).expect("handle monté");
        index_document(&h, &[("content".into(), "pub phantom".into())], GHOST).expect("injection");
        h.commit().expect("commit fantôme RÉUSSI avant sync");
    }
    let (injected, token_injected) = c.fts_live_ids(&table).expect("FTS après commit");
    let mut expected = graph_before.clone(); expected.push(GHOST); expected.sort_unstable();
    assert_eq!(injected, expected);
    assert_ne!(token_before, token_injected);
    assert_eq!(graph_ids(&c, &table), graph_before);
    eprintln!("FTS_DOUBLE_BEFORE form={form} table={table} graph={graph_before:?} fts={injected:?} token={token_injected:?}");
    let trace = sync_and_trace(&mut c, &src, &capture, "unchanged");
    let branch = if generations { "dispatch:generations" } else { "dispatch:classic" };
    assert_eq!(trace.first().map(String::as_str), Some(branch), "branche réellement parcourue");
    let graph_after = graph_ids(&c, &table);
    let (fts_after, token_after) = c.fts_live_ids(&table).expect("FTS après sync");
    let gs: BTreeSet<_> = graph_after.iter().copied().collect();
    let fs: BTreeSet<_> = fts_after.iter().copied().collect();
    eprintln!("FTS_DOUBLE_AFTER form={form} graph={graph_after:?} fts={fts_after:?} token={token_after:?} fts_without_row={:?} rows_without_fts={:?}", fs.difference(&gs).collect::<Vec<_>>(), gs.difference(&fs).collect::<Vec<_>>());
    assert_eq!(healthy_rows(&c, &table), healthy_before, "UUID/H/positions conservés");
    assert_eq!(graph_after, graph_before);
    assert_eq!(fs.len(), fts_after.len(), "pas de doublon FTS");
    assert_eq!(fts_after, graph_after, "la même API doit retirer le fantôme dans les deux formes");
    assert!(!fts_after.contains(&GHOST));
    assert_ne!(token_injected, token_after);
    // Aucun nettoyage : conserver les bases pour preuve et inventaire.
}
#[test]
fn classic_sync_removes_a_committed_fts_ghost() { run_case(false); }
#[test]
fn generations_sync_removes_a_committed_fts_ghost() { run_case(true); }

#[test]
fn generations_sync_rebuilds_pending_fts_even_when_counts_match() {
    let (mut c, src) = fixture(true, "pending-same-count");
    let capture = Capture::start();
    sync_and_trace(&mut c, &src, &capture, "initial-pending");
    let table = scope_chunks(&c).unwrap().parent;
    let graph = graph_ids(&c, &table);
    assert_eq!(c.fts_live_ids(&table).unwrap().0, graph);
    let healthy = healthy_rows(&c, &table);
    c.mark_fts_pending(&["Scope"]).unwrap();
    assert!(c.fts_pending_entities().unwrap().contains(&"Scope".to_string()));
    let trace = sync_and_trace(&mut c, &src, &capture, "pending-same-count");
    assert_eq!(trace.first().map(String::as_str), Some("dispatch:generations"));
    assert!(trace.contains(&"counts:end:0".to_string()), "{trace:?}");
    assert!(trace.contains(&"rebuild:end:1".to_string()), "{trace:?}");
    assert!(c.fts_pending_entities().unwrap().is_empty());
    assert_eq!(c.fts_live_ids(&table).unwrap().0, graph);
    assert_eq!(healthy_rows(&c, &table), healthy);
}

struct RefusingList<'a> {
    source: &'a Snapshot,
    calls: std::sync::atomic::AtomicUsize,
}
impl FileSource for RefusingList<'_> {
    fn cursor(&self) -> String { self.source.cursor() }
    fn list(&self) -> Result<Vec<String>, String> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Err("source-list-refused-witness".to_string())
    }
    fn read(&self, _: &str) -> Result<Option<String>, String> {
        panic!("une liste refusée ne lit aucun fichier")
    }
}

#[test]
fn generations_maintenance_error_prevents_source_listing_and_preserves_pending() {
    let (mut c, src) = fixture(true, "pending-error");
    let capture = Capture::start();
    sync_and_trace(&mut c, &src, &capture, "initial-error");
    c.mark_fts_pending(&["UnknownPhysicalTable"]).unwrap();
    let source = RefusingList { source: &src, calls: std::sync::atomic::AtomicUsize::new(0) };
    let error = sync_source(&mut c, &source, &SourceSyncOptions::default(), &mut |_| {}).unwrap_err();
    let trace = capture.take();
    eprintln!("FTS_PENDING_ERROR {trace:?} error={error}");
    assert!(!error.contains("source-list-refused-witness"), "{error}");
    assert_eq!(source.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert!(trace.contains(&"rebuild:begin".to_string()), "{trace:?}");
    assert!(!trace.iter().any(|event| event.starts_with("rebuild:end:")), "{trace:?}");
    assert!(c.fts_pending_entities().unwrap().contains(&"UnknownPhysicalTable".to_string()));
}

#[test]
fn generations_maintenance_precedes_an_early_source_error() {
    let (mut c, src) = fixture(true, "early-source-error");
    let capture = Capture::start();
    sync_and_trace(&mut c, &src, &capture, "initial-source-error");
    let table = scope_chunks(&c).unwrap().parent;
    let graph = graph_ids(&c, &table);
    let healthy = healthy_rows(&c, &table);
    c.mark_fts_pending(&["Scope"]).unwrap();
    let source = RefusingList { source: &src, calls: std::sync::atomic::AtomicUsize::new(0) };
    let error = sync_source(&mut c, &source, &SourceSyncOptions::default(), &mut |_| {}).unwrap_err();
    let trace = capture.take();
    eprintln!("FTS_EARLY_ERROR {trace:?} error={error}");
    assert!(error.contains("source-list-refused-witness"), "{error}");
    assert_eq!(source.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(trace.contains(&"rebuild:end:1".to_string()), "{trace:?}");
    assert!(c.fts_pending_entities().unwrap().is_empty());
    assert_eq!(c.fts_live_ids(&table).unwrap().0, graph);
    assert_eq!(healthy_rows(&c, &table), healthy);
}


// Témoins de chemin publiés AVANT le microfix ; aucun environnement global muté.
fn fixture_path_test_root() -> std::path::PathBuf {
    let target = fixture_disk_target();
    std::fs::create_dir_all(&target).unwrap();
    tempfile::Builder::new().prefix("fts-path-witness-").tempdir_in(target).unwrap().keep()
}

#[test]
fn explicit_fixture_root_keeps_unique_cases_and_prior_bytes() {
    let root = fixture_path_test_root();
    let first = fixture_case(Some(&root), &root.join("unused-target"), "generations").unwrap();
    std::fs::write(first.join("sentinel"), b"prior-proof").unwrap();
    let second = fixture_case(Some(&root), &root.join("unused-target"), "generations").unwrap();
    assert_ne!(first, second);
    assert_eq!(first.parent(), Some(root.as_path()));
    assert_eq!(second.parent(), Some(root.as_path()));
    assert_eq!(std::fs::read(first.join("sentinel")).unwrap(), b"prior-proof");
    assert!(!root.join("unused-target").exists());
}

#[test]
fn absent_fixture_root_uses_only_the_private_disk_target() {
    let root = fixture_path_test_root();
    let target = root.join("disk-target");
    let first = fixture_case(None, &target, "generations").unwrap();
    let second = fixture_case(None, &target, "generations").unwrap();
    assert_ne!(first, second);
    assert_eq!(first.parent(), Some(target.join("fts-double-fixtures").as_path()));
    assert_eq!(second.parent(), first.parent());
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn invalid_explicit_fixture_roots_never_fall_back_or_follow_symlinks() {
    let root = fixture_path_test_root();
    let target = root.join("unused-target");
    assert!(fixture_case(Some(std::path::Path::new("relative")), &target, "case").is_err());
    assert!(fixture_case(Some(&root.join("missing")), &target, "case").is_err());
    let alias = root.join("alias");
    std::os::unix::fs::symlink(&root, &alias).unwrap();
    assert!(fixture_case(Some(&alias), &target, "case").is_err());
    assert!(fixture_case(Some(&alias.join("child")), &target, "case").is_err());
    assert!(!target.exists());
}
