//! Explicit CPU fixtures at the real node's candidate/conversion boundary.
//! They cannot be constructed by a production graph or certify a physical index.
use crate::{catalog::CatalogError, filter::FilterCondition};
use std::sync::Arc;
pub(crate) const CANDIDATES: &str = "fixture_candidate_probe";
pub(crate) const OFFSETS: &str = "fixture_offset_resolution_probe";
#[derive(Debug, Clone)]
pub(crate) struct CandidateObservation {
    pub signal: &'static str,
    pub limit: usize,
    pub offsets: Option<Vec<u64>>,
}
pub(crate) struct CandidateProbe(
    pub Arc<dyn Fn(CandidateObservation) -> Result<(), String> + Send + Sync>,
);
pub(crate) struct OffsetProbe(
    pub Arc<dyn Fn(&FilterCondition, bool) -> Result<Option<Vec<u64>>, CatalogError> + Send + Sync>,
);

use super::ReadAdmission;
use crate::connection::{
    CallbackConnection, CypherValue, DbConnection, MockConnection, QueryParam, QueryResult,
};
use crate::dataflow::{NodeContext, ServiceRegistry};
use crate::search_backend::*;
use std::collections::{HashMap, HashSet};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};
pub(crate) const STOP: &str = "fixture-stop-before-candidates";

/// Dense uses the EXISTING SearchBackend seam, not a parallel local filter.
pub(crate) struct DenseSpy {
    pub capable: bool,
    pub calls: AtomicUsize,
    pub selected: Mutex<Vec<String>>,
    pub limits: Mutex<Vec<usize>>,
}
impl SearchBackend for DenseSpy {
    fn honore_le_filtre(&self) -> bool {
        self.capable
    }
    fn vector_search(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &[f32],
        _: usize,
    ) -> Result<Vec<VectorHit>, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err("fixture: unfiltered dense call is forbidden".into())
    }
    fn vector_search_filtered(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &[f32],
        limit: usize,
        join: Option<&str>,
        condition: Option<&str>,
        params: &[QueryParam],
    ) -> Result<Vec<VectorHit>, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.limits.lock().unwrap().push(limit);
        assert!(
            join.is_some(),
            "node lost the parent join before dense candidates"
        );
        assert!(
            condition.is_some_and(|w| w.contains("_uuid")),
            "node lost admission before dense candidates"
        );
        let admitted: HashSet<_> = params
            .iter()
            .flat_map(|p| match &p.value {
                CypherValue::List(v) => v
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect::<Vec<_>>(),
                _ => vec![],
            })
            .collect();
        assert!(!admitted.contains("hidden"));
        let ranked: Vec<_> = (0..20)
            .map(|i| format!("invisible-{i}"))
            .chain(["b".into(), "c".into()])
            .collect();
        *self.selected.lock().unwrap() = ranked
            .into_iter()
            .filter(|id| admitted.contains(id))
            .take(limit)
            .collect();
        // End this fixture at the actual candidate boundary. No physical DB,
        // embedding service, offset enrichment or index query follows it.
        Err(STOP.into())
    }
    fn resolve_offsets(&self, _: &str, _: &[u64], _: &[&str]) -> Result<Vec<OffsetResult>, String> {
        Err("unexpected offset backend call".into())
    }
    fn fetch_entities(&self, _: &str, _: &[&str], _: &[&str]) -> Result<Vec<EntityRow>, String> {
        Err("unexpected enrichment".into())
    }
    fn fetch_chunks(&self, _: &str, _: &[&str]) -> Result<Vec<ChunkMeta>, String> {
        Err("unexpected chunks".into())
    }
    fn fetch_with_chunks(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: bool,
        _: &[u64],
        _: &[&str],
    ) -> Result<Vec<ChunkWithParent>, String> {
        Err("unexpected chunk enrichment".into())
    }
}

pub(crate) struct NodeFixture {
    pub ctx: NodeContext,
    pub dense: Arc<DenseSpy>,
    pub candidates: Arc<Mutex<Vec<CandidateObservation>>>,
    pub selected_offsets: Arc<Mutex<Vec<u64>>>,
    // Concrete sparse handle required by the node; empty private CPU fixture,
    // never searched because the probe stops at its actual argument boundary.
    _temporary: tempfile::TempDir,
}
impl NodeFixture {
    pub(crate) fn new(
        admitted: Arc<ReadAdmission>,
        capable: bool,
        native_text: bool,
        fault: Option<OffsetProbe>,
    ) -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let dense = Arc::new(DenseSpy {
            capable,
            calls: AtomicUsize::new(0),
            selected: Mutex::new(vec![]),
            limits: Mutex::new(vec![]),
        });
        let conn: Arc<dyn DbConnection> = Arc::new(CallbackConnection::new(|query, params| {
            if query.contains("SHOW_INDEXES") {
                return Ok(QueryResult {
                    columns: vec![
                        "table_name".into(),
                        "index_name".into(),
                        "extension_loaded".into(),
                        "state".into(),
                    ],
                    rows: vec![vec![
                        "Snippet_Chunk".into(),
                        "Snippet_Chunk_vec".into(),
                        CypherValue::Bool(true),
                        "READY".into(),
                    ]],
                });
            }
            assert!(
                query.contains("WHERE") && query.contains("_uuid"),
                "real node conversion lost the occurrence predicate"
            );
            let ids: HashSet<_> = params
                .iter()
                .flat_map(|p| match &p.value {
                    CypherValue::List(v) => v
                        .iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect::<Vec<_>>(),
                    _ => vec![],
                })
                .collect();
            assert_eq!(ids, HashSet::from(["b".into(), "c".into(), "local".into()]));
            Ok(QueryResult {
                columns: vec!["offset".into()],
                rows: vec![vec![CypherValue::Int(41)], vec![CypherValue::Int(77)]],
            })
        }));
        let mut cat = crate::Catalog::new(
            Box::new(MockConnection::new()),
            Box::new(crate::embedder::MockEmbedder::new(4)),
            crate::CatalogConfig {
                embedding_dim: 4,
                allow_mock_embedder: true,
                ..Default::default()
            },
        );
        cat.set_cache_base(temporary.path().join("unused-fts"));
        // Prevent the real node's live-catalog fallback from opening a lucivy
        // writer. No native-text SERVICE is provided except the refusal case.
        cat.set_moteur_texte(MoteurTexte::Natif);
        let mut base = ServiceRegistry::new();
        base.register(super::SERVICE, admitted.clone());
        let backend: Arc<dyn SearchBackend> = dense.clone();
        let view = crate::read_view::ReadView {
            dialect: Arc::new(crate::dialect::Rag3dbDialect),
            conn: conn.clone(),
            entity_configs: HashMap::new(),
            entity_defs: HashMap::new(),
            relations: HashMap::new(),
            scope: admitted.cell.clone(),
            fanout: None,
            two_step_hops: false,
            multi_cell: false,
            search_backend_externe: Some(backend.clone()),
            event_bus: crate::events::EventBus::new(8),
            search_services: Arc::new(base),
            pool: None,
            sans_instantane: false,
            texte_natif: false,
        };
        let mut services = ServiceRegistry::layered(view.search_services.clone());
        services.register("scope", admitted.cell.clone());
        services.register(
            crate::read_view::READ_VIEW_SERVICE,
            Arc::new(crate::read_view::ReadViewCell::avec(view)),
        );
        services.register("catalog", Arc::new(Mutex::new(cat)));
        services.register("conn", crate::dataflow::services::ConnService(conn));
        services.register(
            "dialect",
            Arc::new(crate::dialect::Rag3dbDialect) as Arc<dyn crate::dialect::SchemaDialect>,
        );
        services.register(
            "embedder",
            Arc::new(crate::embedder::MockEmbedder::new(4)) as Arc<dyn crate::embedder::Embedder>,
        );
        if native_text {
            services.register("texte_natif", backend);
        }
        let handle = sparse_vector::handle::SparseHandle::create(
            temporary
                .path()
                .join("empty-sparse-fixture")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        services.register(
            "sparse_handles",
            HashMap::from([("Snippet_Chunk".to_string(), Arc::new(handle))]),
        );
        let candidates = Arc::new(Mutex::new(vec![]));
        let observed = candidates.clone();
        let selected_offsets = Arc::new(Mutex::new(vec![]));
        let selected = selected_offsets.clone();
        services.register(
            CANDIDATES,
            CandidateProbe(Arc::new(move |call| {
                let ids = call
                    .offsets
                    .as_ref()
                    .expect("node omitted its mandatory offsets at candidates");
                assert_eq!(ids, &[41, 77]);
                let ranked: Vec<u64> = (0..20).chain([41, 77]).collect();
                *selected.lock().unwrap() = ranked
                    .into_iter()
                    .filter(|id| ids.contains(id))
                    .take(call.limit)
                    .collect();
                observed.lock().unwrap().push(call);
                Err(STOP.into())
            })),
        );
        if let Some(fault) = fault {
            services.register(OFFSETS, fault);
        }
        Self {
            ctx: NodeContext::with_services(Arc::new(services)),
            dense,
            candidates,
            selected_offsets,
            _temporary: temporary,
        }
    }
}
