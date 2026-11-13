use super::super::layers::{FileKey, LayerRevision, Occurrence};
use super::super::{tests::snapshot, NewPublicationOrigin, Registry};
use super::*;
use crate::connection::{CypherValue, MockConnection, QueryResult};
use crate::dataflow::{NodeContext, ServiceRegistry};
use crate::search::SearchOptions;
use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Debug)]
struct Payload {
    cell: crate::scope::Scope,
    h: &'static str,
    line: u64,
}
fn id(s: &str) -> OccurrenceKey {
    OccurrenceKey::new("Snippet", s).unwrap()
}
fn row(s: &str, file: &str, line: u64) -> Occurrence<Payload> {
    Occurrence::new(
        id(s),
        FileKey::new(file).unwrap(),
        Payload {
            cell: crate::scope::Scope::new("org", "p"),
            h: "shared-H",
            line,
        },
    )
}
fn generations() -> (LayerGeneration, LayerGeneration, LayerGeneration) {
    let mut r = Registry::genesis(
        NewPublicationOrigin::fresh(),
        "fixture",
        snapshot("fixture"),
    )
    .unwrap();
    let p = r.published(&r.head).unwrap();
    let a = LayerGeneration::from_publication("anchor", &p).unwrap();
    let intent = r.prepare("child", snapshot("fixture")).unwrap();
    let p = r.apply(&intent).unwrap().current().clone();
    let b = LayerGeneration::from_publication("child", &p).unwrap();
    let intent = r.prepare("advance", snapshot("fixture")).unwrap();
    let p = r.apply(&intent).unwrap().current().clone();
    (
        a,
        b,
        LayerGeneration::from_publication("anchor", &p).unwrap(),
    )
}
fn layers_and_advance() -> (PinnedLayers<Payload>, LayerGeneration) {
    let (a, b, later) = generations();
    let layers = PinnedLayers::new(vec![
        LayerRevision::new(
            a.clone(),
            None,
            vec![
                row("hidden", "org/p/a.rs", 10),
                row("b", "org/p/b.rs", 90),
                row("c", "org/p/c.rs", 21),
            ],
            BTreeSet::new(),
            BTreeSet::new(),
        )
        .unwrap(),
        LayerRevision::new(
            b,
            Some(a),
            vec![row("local", "org/p/local.rs", 41)],
            BTreeSet::from([id("hidden")]),
            BTreeSet::new(),
        )
        .unwrap(),
    ])
    .unwrap();
    (layers, later)
}
fn layers() -> PinnedLayers<Payload> {
    layers_and_advance().0
}
fn admission_from(
    layers: &PinnedLayers<Payload>,
    name: &str,
    deadline: Instant,
) -> Result<ReadAdmission> {
    let selected = layers.select(name).map_err(AdmissionError::Layer)?;
    let resolved = layers.resolve(selected).map_err(AdmissionError::Layer)?;
    let bindings = resolved
        .occurrences()
        .iter()
        .enumerate()
        .map(|(i, (k, v))| {
            (
                k.clone(),
                StorageAddress {
                    cell: v.occurrence().value().cell.clone(),
                    generation: v.provenance().clone(),
                    model: "fixture-model".into(),
                    offset: i as u64,
                },
            )
        })
        .collect();
    ReadAdmission::from_fixture(
        layers,
        name,
        crate::scope::Scope::new("org", "p"),
        BTreeSet::from(["Snippet".into()]),
        bindings,
        deadline,
    )
}
fn admission() -> Arc<ReadAdmission> {
    Arc::new(
        admission_from(
            &layers(),
            "child",
            Instant::now() + Duration::from_secs(3600),
        )
        .unwrap(),
    )
}
fn context(a: Option<Arc<ReadAdmission>>) -> NodeContext {
    let mut services = ServiceRegistry::new();
    if let Some(a) = a {
        services.register("scope", a.cell.clone());
        services.register(SERVICE, a);
    }
    NodeContext::with_services(Arc::new(services))
}
fn target() -> crate::search::SearchTarget {
    crate::search::SearchTarget {
        occurrence_projection: None,
        name: "Snippet".into(),
        parent_table: "Snippet".into(),
        chunk_table: "Snippet_Chunk".into(),
        chunk_rel: "Snippet_CHUNKED_FROM".into(),
        chunk_rel_fwd: false,
        bm25_fields: vec![],
        enrich_fields: vec![],
        default_signals: crate::search::SearchSignals::HYBRID,
        default_fusion: None,
        field_weights: vec![],
        has_source_refs: false,
        filter_indirection: None,
        title_field: None,
        title_boost: 1.0,
    }
}
fn matches_filter(f: &crate::filter::FilterCondition, id: &str) -> bool {
    use crate::filter::{FilterCondition as F, FilterValue as V};
    match f {
        F::Field {
            key,
            value: V::List(ids),
        } if key == "_uuid" => ids.iter().any(|v| v.as_str() == Some(id)),
        F::Must(fs) => fs.iter().all(|f| matches_filter(f, id)),
        other => panic!("fixture cannot interpret {other:?}"),
    }
}

fn query_for_node(ctx: &mut NodeContext, soft: bool, scan: bool) {
    use crate::dataflow::{PortValue, QueryPayload};
    let mut options = SearchOptions::default();
    options.limit = 2;
    options.strict_signals = !soft;
    options.bm25_mode = crate::search::BM25Mode::Parse;
    options.signals =
        Some(crate::search::SearchSignals::HYBRID | crate::search::SearchSignals::SPARSE);
    ctx.set_input(
        "query",
        PortValue::new(QueryPayload {
            target_name: "Snippet".into(),
            query: "query".into(),
            options,
            target: Some(target()),
            embedding: Some(vec![1.0; 4]),
            sparse: Some(crate::sparse_index::SparseVector::new(vec![1], vec![1.0])),
            scan,
        }),
    );
}

#[test]
fn all_three_signal_nodes_transmit_the_admission_at_the_actual_candidate_boundary() {
    use super::node_probe::{NodeFixture, STOP};
    use crate::dataflow::{
        generic_search_nodes::{BM25SearchNode, SparseSearchNode, VectorSearchNode},
        Node,
    };
    use std::sync::atomic::Ordering;
    for signal in ["dense", "sparse", "fts"] {
        let mut fixture = NodeFixture::new(admission(), true, false, None);
        query_for_node(&mut fixture.ctx, false, false);
        let result = match signal {
            "dense" => VectorSearchNode::new("dense", 2).execute(&mut fixture.ctx),
            "sparse" => SparseSearchNode::new("sparse", 2).execute(&mut fixture.ctx),
            _ => BM25SearchNode::new("fts", 2).execute(&mut fixture.ctx),
        };
        assert!(
            result.unwrap_err().contains(STOP),
            "the actual node must reach its candidate spy: {signal}"
        );
        if signal == "dense" {
            assert_eq!(fixture.dense.calls.load(Ordering::SeqCst), 1);
            assert_eq!(*fixture.dense.selected.lock().unwrap(), ["b", "c"]);
            assert_eq!(*fixture.dense.limits.lock().unwrap(), [2]);
            assert!(fixture.candidates.lock().unwrap().is_empty());
        } else {
            let calls = fixture.candidates.lock().unwrap();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].signal, signal);
            assert_eq!(calls[0].limit, 2);
            assert_eq!(calls[0].offsets.as_deref(), Some(&[41_u64, 77][..]));
            assert_eq!(*fixture.selected_offsets.lock().unwrap(), [41, 77]);
            assert_eq!(fixture.dense.calls.load(Ordering::SeqCst), 0);
        }
    }
}

#[test]
fn absent_empty_and_failed_admission_never_mean_the_same_mask() {
    let conn: Arc<dyn crate::connection::DbConnection> = Arc::new(MockConnection::new());
    let (o, roots) = crate::dataflow::generic_search_nodes::occurrence_mask_options(
        &context(None),
        &target(),
        SearchOptions::default(),
        &conn,
    )
    .unwrap();
    assert!(o.filter_condition.is_none() && roots.is_none());
    let (a, _, _) = generations();
    let empty = PinnedLayers::<Payload>::new(vec![LayerRevision::new(
        a,
        None,
        vec![],
        BTreeSet::new(),
        BTreeSet::new(),
    )
    .unwrap()])
    .unwrap();
    let e = Arc::new(
        admission_from(&empty, "anchor", Instant::now() + Duration::from_secs(10)).unwrap(),
    );
    let (o, roots) = crate::dataflow::generic_search_nodes::occurrence_mask_options(
        &context(Some(e)),
        &target(),
        SearchOptions::default(),
        &conn,
    )
    .unwrap();
    assert!(roots.unwrap().is_empty());
    assert!(!matches_filter(o.filter_condition.as_ref().unwrap(), "b"));
    assert!(admission().mask("UnboundEntity", None).is_err());
}

#[test]
fn shared_h_does_not_resurrect_a_hidden_occurrence_or_move_its_visible_peer() {
    let l = layers();
    let r = l.resolve(l.select("child").unwrap()).unwrap();
    let a = admission_from(&l, "child", Instant::now() + Duration::from_secs(10)).unwrap();
    assert!(!a.admits("Snippet", "hidden").unwrap());
    assert!(a.admits("Snippet", "b").unwrap());
    assert_eq!(r.occurrences()[&id("b")].occurrence().value().h, "shared-H");
    assert_eq!(r.occurrences()[&id("b")].occurrence().value().line, 90);
    assert_eq!(
        a.provenance(&id("b")).unwrap(),
        r.occurrences()[&id("b")].provenance()
    );
    let a = Arc::new(a);
    let conn: Arc<dyn crate::connection::DbConnection> =
        Arc::new(crate::connection::CallbackConnection::new(|_, params| {
            if params.is_empty() {
                return Ok(QueryResult {
                    columns: vec![],
                    rows: vec![
                        vec![CypherValue::String("hidden".into())],
                        vec![CypherValue::String("b".into())],
                    ],
                });
            }
            assert_eq!(
                params[0].value,
                CypherValue::List(vec![CypherValue::String("b".into())]),
                "visible roots must be intersected before resolving the shared version"
            );
            Ok(QueryResult {
                columns: vec![],
                rows: vec![vec![
                    CypherValue::String("b".into()),
                    CypherValue::String("shared-version".into()),
                ]],
            })
        }));
    let mut services = ServiceRegistry::new();
    services.register(SERVICE, a.clone());
    let view = crate::read_view::ReadView {
        dialect: Arc::new(crate::dialect::Rag3dbDialect),
        conn: conn.clone(),
        entity_configs: HashMap::new(),
        entity_defs: HashMap::new(),
        relations: HashMap::new(),
        scope: crate::scope::Scope::new("org", "p"),
        fanout: None,
        two_step_hops: false,
        multi_cell: false,
        search_backend_externe: None,
        event_bus: crate::events::EventBus::new(8),
        search_services: Arc::new(services),
        pool: None,
        sans_instantane: false,
        texte_natif: false,
    };
    let mut services = ServiceRegistry::layered(view.search_services.clone());
    services.register(
        crate::read_view::READ_VIEW_SERVICE,
        Arc::new(crate::read_view::ReadViewCell::avec(view)),
    );
    services.register(
        "dialect",
        Arc::new(crate::dialect::Rag3dbDialect) as Arc<dyn crate::dialect::SchemaDialect>,
    );
    let projected_ctx = NodeContext::with_services(Arc::new(services));
    let mut projected = target();
    projected.parent_table = "SnippetContent".into();
    projected.occurrence_projection = Some(crate::search::OccurrenceProjection {
        entity: "Snippet".into(),
        relation: "SnippetContent_DERIVED_FROM".into(),
        fields: vec![],
        content_fields: vec![],
    });
    let (options, roots) = crate::dataflow::generic_search_nodes::occurrence_mask_options(
        &projected_ctx,
        &projected,
        SearchOptions::default(),
        &conn,
    )
    .unwrap();
    assert_eq!(roots.unwrap(), HashSet::from(["b".into()]));
    assert!(matches_filter(
        options.filter_condition.as_ref().unwrap(),
        "shared-version"
    ));
    let mut ctx = context(Some(a));
    let result = |uuid: &str| crate::search::SearchResult {
        uuid: uuid.into(),
        score: 1.0,
        entity: Some("Snippet".into()),
        data: Some(BTreeMap::from([
            ("line".into(), CypherValue::Int(90)),
            ("h".into(), CypherValue::String("shared-H".into())),
        ])),
        chunk: None,
        chunks: None,
    };
    let visible = crate::dataflow::generic_search_nodes::finish_signal(
        &mut ctx,
        "fixture",
        &target(),
        vec![result("b")],
        crate::search::ResultMode::Aggregated,
        None,
        "dense",
    )
    .unwrap();
    assert_eq!(
        visible[0].data.as_ref().unwrap()["line"],
        CypherValue::Int(90)
    );
    assert!(
        crate::dataflow::generic_search_nodes::finish_signal(
            &mut ctx,
            "fixture",
            &target(),
            vec![result("hidden")],
            crate::search::ResultMode::Aggregated,
            None,
            "dense"
        )
        .is_err(),
        "backend leaks fail instead of becoming a post-filter"
    );
}

#[test]
fn same_revision_file_replacement_keeps_the_other_cells_file() {
    let (a, b, _) = generations();
    let l = PinnedLayers::new(vec![
        LayerRevision::new(
            a.clone(),
            None,
            vec![
                row("old", "org/p/a.rs", 10),
                Occurrence::new(
                    id("other-cell"),
                    FileKey::new("org/q/a.rs").unwrap(),
                    Payload {
                        cell: crate::scope::Scope::new("org", "q"),
                        h: "shared-H",
                        line: 90,
                    },
                ),
            ],
            BTreeSet::new(),
            BTreeSet::new(),
        )
        .unwrap(),
        LayerRevision::new(
            b,
            Some(a),
            vec![row("new", "org/p/a.rs", 41)],
            BTreeSet::new(),
            BTreeSet::from([FileKey::new("org/p/a.rs").unwrap()]),
        )
        .unwrap(),
    ])
    .unwrap();
    let a = admission_from(&l, "child", Instant::now() + Duration::from_secs(10)).unwrap();
    assert_eq!(
        a.mask("Snippet", None).unwrap(),
        HashSet::from(["new".into()])
    );
    let r = l.resolve(l.select("child").unwrap()).unwrap();
    assert_eq!(r.occurrences()[&id("new")].occurrence().value().line, 41);
    assert_eq!(
        r.occurrences()[&id("other-cell")].occurrence().value().line,
        90,
        "the file tombstone must not remove another cell's occurrence from the forest"
    );
    assert!(
        !a.admits("Snippet", "other-cell").unwrap(),
        "the active call still intersects the cell boundary"
    );
}

#[test]
fn a_user_filter_cannot_resurrect_an_occurrence_hidden_by_the_view() {
    let conn: Arc<dyn crate::connection::DbConnection> = Arc::new(MockConnection::new());
    let mut options = SearchOptions::default();
    options.filter_condition = Some(crate::filter::FilterCondition::Field {
        key: "_uuid".into(),
        value: crate::filter::FilterValue::List(vec![CypherValue::String("hidden".into())]),
    });
    let (o, _) = crate::dataflow::generic_search_nodes::occurrence_mask_options(
        &context(Some(admission())),
        &target(),
        options,
        &conn,
    )
    .unwrap();
    assert!(!matches_filter(
        o.filter_condition.as_ref().unwrap(),
        "hidden"
    ));
    assert!(!matches_filter(o.filter_condition.as_ref().unwrap(), "b"));
}

#[test]
fn related_reads_admit_both_endpoints_before_limit_without_deduplicating_edges() {
    use crate::connection::CallbackConnection;
    let conn = CallbackConnection::new(|query, params| {
        assert!(
            !query.contains("LIMIT"),
            "the relation mask must precede per-parent limit"
        );
        assert_eq!(
            params[0].value,
            CypherValue::List(vec![CypherValue::String("b".into())])
        );
        Ok(QueryResult {
            columns: vec![],
            rows: vec![
                vec![
                    CypherValue::String("b".into()),
                    CypherValue::String("hidden".into()),
                    CypherValue::String("Snippet".into()),
                    CypherValue::Map(BTreeMap::new()),
                ],
                vec![
                    CypherValue::String("b".into()),
                    CypherValue::String("c".into()),
                    CypherValue::String("Snippet".into()),
                    CypherValue::Map(BTreeMap::new()),
                ],
                vec![
                    CypherValue::String("b".into()),
                    CypherValue::String("c".into()),
                    CypherValue::String("Snippet".into()),
                    CypherValue::Map(BTreeMap::new()),
                ],
                vec![
                    CypherValue::String("hidden".into()),
                    CypherValue::String("b".into()),
                    CypherValue::String("Snippet".into()),
                    CypherValue::Map(BTreeMap::new()),
                ],
            ],
        })
    });
    for direction in [
        crate::search_strategy::ExpansionDirection::Incoming,
        crate::search_strategy::ExpansionDirection::Outgoing,
    ] {
        let a = admission();
        let result = crate::dataflow::search_nodes::fetch_related_admitted_in(
            &crate::dialect::Rag3dbDialect,
            &conn,
            &["b".into(), "hidden".into()],
            "REL",
            direction,
            2,
            Some(&a),
            Some(&HashMap::from([
                ("b".into(), "Snippet".into()),
                ("hidden".into(), "Snippet".into()),
            ])),
        )
        .unwrap();
        assert_eq!(result["b"].len(), 2);
        assert!(result["b"].iter().all(|c| c.uuid == "c"));
        assert!(!result.contains_key("hidden"));
    }
}

#[test]
fn an_advanced_head_cannot_rebind_a_pinned_parent_or_storage_generation() {
    let (l, later) = layers_and_advance();
    let a = admission_from(&l, "child", Instant::now() + Duration::from_secs(10)).unwrap();
    let before = a.provenance(&id("b")).unwrap().clone();
    assert_eq!(a.selected(), l.select("child").unwrap());
    assert_ne!(before, *l.select("child").unwrap());
    assert_eq!(later.origin(), before.origin());
    assert_ne!(later.publication(), before.publication());
    assert!(a
        .validate_storage(&id("b"), &later, "fixture-model", 0)
        .is_err());
    assert_eq!(a.winners[&id("b")], before);
}

#[test]
fn losing_the_snapshot_invalidates_all_copies_and_cannot_be_repaired_by_a_new_connection() {
    let a = admission();
    let mut services = ServiceRegistry::new();
    services.register(SERVICE, a.clone());
    let v = crate::read_view::ReadView {
        dialect: Arc::new(crate::dialect::Rag3dbDialect),
        conn: Arc::new(MockConnection::new()),
        entity_configs: HashMap::new(),
        entity_defs: HashMap::new(),
        relations: HashMap::new(),
        scope: crate::scope::Scope::new("org", "p"),
        fanout: None,
        two_step_hops: false,
        multi_cell: false,
        search_backend_externe: None,
        event_bus: crate::events::EventBus::new(8),
        search_services: Arc::new(services),
        pool: None,
        sans_instantane: false,
        texte_natif: false,
    };
    let invalid = v.sans_instantane();
    let copied = invalid.sur_connexion(Arc::new(MockConnection::new()));
    assert!(copied.admission().unwrap().check().is_err());
    assert!(a.check().is_err());
}

#[test]
fn real_nodes_refuse_incapable_dense_native_text_and_missing_or_failed_offsets_even_when_soft() {
    use super::node_probe::{NodeFixture, OffsetProbe};
    use crate::dataflow::{
        generic_search_nodes::{BM25SearchNode, SparseSearchNode, VectorSearchNode},
        Node,
    };
    use std::sync::atomic::Ordering;
    let a = admission();
    let mut fixture = NodeFixture::new(a.clone(), false, false, None);
    query_for_node(&mut fixture.ctx, true, false);
    assert!(VectorSearchNode::new("dense", 2)
        .execute(&mut fixture.ctx)
        .unwrap_err()
        .contains("IncapableFilter"));
    assert_eq!(fixture.dense.calls.load(Ordering::SeqCst), 0);
    assert!(fixture.candidates.lock().unwrap().is_empty());
    assert_eq!(a.check(), Err(AdmissionError::IncapableFilter));

    let a = admission();
    let mut fixture = NodeFixture::new(a.clone(), true, true, None);
    query_for_node(&mut fixture.ctx, true, false);
    assert!(BM25SearchNode::new("fts", 2)
        .execute(&mut fixture.ctx)
        .unwrap_err()
        .contains("BM25 natif"));
    assert_eq!(a.check(), Err(AdmissionError::IncapableFilter));
    assert!(fixture.candidates.lock().unwrap().is_empty());

    for sparse in [false, true] {
        for error in [false, true] {
            let a = admission();
            let resolution_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let count = resolution_calls.clone();
            let fault = OffsetProbe(Arc::new(move |condition, chunks| {
                count.fetch_add(1, Ordering::SeqCst);
                assert_eq!(chunks, sparse);
                assert!(matches_filter(condition, "b") && !matches_filter(condition, "hidden"));
                if error {
                    Err(crate::catalog::CatalogError::DbError(
                        "fixture offset failure".into(),
                    ))
                } else {
                    Ok(None)
                }
            }));
            let mut fixture = NodeFixture::new(a.clone(), true, false, Some(fault));
            query_for_node(&mut fixture.ctx, true, false);
            let result = if sparse {
                SparseSearchNode::new("sparse", 2).execute(&mut fixture.ctx)
            } else {
                BM25SearchNode::new("fts", 2).execute(&mut fixture.ctx)
            };
            assert!(result.unwrap_err().contains("admission"));
            assert_eq!(resolution_calls.load(Ordering::SeqCst), 1);
            assert!(
                fixture.candidates.lock().unwrap().is_empty(),
                "failed conversion must not reach unmasked candidates"
            );
            assert_eq!(fixture.dense.calls.load(Ordering::SeqCst), 0);
            assert_eq!(a.check(), Err(AdmissionError::StorageChanged));
        }
    }
    let mut ctx = context(Some(admission()));
    query_for_node(&mut ctx, true, true);
    assert!(VectorSearchNode::new("dense", 2)
        .execute(&mut ctx)
        .unwrap_err()
        .contains("ScanWithoutAdmission"));
}

#[test]
fn unknown_ambiguous_or_fixture_only_views_never_fall_back_to_a_public_tool() {
    let l = layers();
    assert!(admission_from(&l, "unknown", Instant::now() + Duration::from_secs(10)).is_err());
    let (a, _, advanced) = generations();
    let l = PinnedLayers::<Payload>::new(vec![
        LayerRevision::new(a, None, vec![], BTreeSet::new(), BTreeSet::new()).unwrap(),
        LayerRevision::new(advanced, None, vec![], BTreeSet::new(), BTreeSet::new()).unwrap(),
    ])
    .unwrap();
    assert!(admission_from(&l, "anchor", Instant::now() + Duration::from_secs(10)).is_err());
    assert_eq!(
        admission().require_physical_tool_access(),
        Err(AdmissionError::FixtureOnly)
    );
}

#[test]
fn provenance_is_opt_in_and_belongs_to_each_winning_occurrence() {
    let a = admission();
    assert!(a
        .provenances([id("b"), id("local")], false)
        .unwrap()
        .is_none());
    let p = a
        .provenances([id("b"), id("local")], true)
        .unwrap()
        .unwrap();
    assert_ne!(p[&id("b")], p[&id("local")]);
    assert_eq!(p[&id("b")].layer(), "anchor");
    assert_eq!(p[&id("local")].layer(), "child");
}

#[test]
fn storage_offsets_never_alias_across_publications_or_models() {
    let (l, other) = layers_and_advance();
    let a = admission_from(&l, "child", Instant::now() + Duration::from_secs(10)).unwrap();
    let key = id("b");
    let address = &a.bindings[&key];
    a.validate_storage(&key, &address.generation, &address.model, address.offset)
        .unwrap();
    assert!(a
        .validate_storage(&key, &address.generation, "another-model", address.offset)
        .is_err());
    let a = admission_from(&l, "child", Instant::now() + Duration::from_secs(10)).unwrap();
    let address = &a.bindings[&key];
    assert_eq!(other.origin(), address.generation.origin());
    assert_ne!(other.publication(), address.generation.publication());
    assert!(a
        .validate_storage(&key, &other, &address.model, address.offset)
        .is_err());
}

#[test]
fn expiry_after_the_last_read_checks_the_effective_deadline_and_latches_before_rendering() {
    let now = Instant::now();
    let deadline = now + Duration::from_secs(1);
    let a = admission_from(&layers(), "child", deadline).unwrap();
    a.check_at(now).unwrap(); // Last read succeeds; no query sets an invalid flag afterwards.
    assert!(a.invalid.lock().unwrap().is_none());
    assert_eq!(a.finish_at(deadline), Err(AdmissionError::Expired));
    assert_eq!(
        a.check_at(now),
        Err(AdmissionError::Expired),
        "finishing cannot revalidate an invalid call"
    );
    assert!(
        a.provenances([id("b")], false).is_err(),
        "even the ordinary renderer must refuse the entire response"
    );
}

fn read_fixture_view(
    a: Option<Arc<ReadAdmission>>,
    cell: crate::scope::Scope,
    conn: Arc<dyn crate::connection::DbConnection>,
) -> crate::read_view::ReadView {
    let mut services = ServiceRegistry::new();
    if let Some(a) = a {
        services.register(SERVICE, a);
    }
    crate::read_view::ReadView {
        dialect: Arc::new(crate::dialect::Rag3dbDialect),
        conn,
        entity_configs: HashMap::new(),
        entity_defs: HashMap::from([(
            "Snippet".into(),
            crate::config::EntityDef {
                fields: HashMap::new(),
                hashsafe: None,
                derived_from: None,
            },
        )]),
        relations: HashMap::new(),
        scope: cell,
        fanout: None,
        two_step_hops: false,
        multi_cell: false,
        search_backend_externe: None,
        event_bus: crate::events::EventBus::new(8),
        search_services: Arc::new(services),
        pool: None,
        sans_instantane: false,
        texte_natif: false,
    }
}

#[test]
fn get_rechecks_after_the_last_callback_for_both_empty_and_nonempty_rows_and_unlatched_expiry() {
    use crate::read_view::ReadCatalog;
    use std::sync::atomic::{AtomicUsize, Ordering};
    for empty in [false, true] {
        for expiry_without_flag in [false, true] {
            let a = admission();
            let invalidated = a.clone();
            let calls = Arc::new(AtomicUsize::new(0));
            let count = calls.clone();
            let conn = Arc::new(crate::connection::CallbackConnection::new(move |_, _| {
                count.fetch_add(1, Ordering::SeqCst);
                if expiry_without_flag {
                    invalidated.tighten_deadline(Instant::now());
                    assert!(
                        invalidated.invalid.lock().unwrap().is_none(),
                        "the DB callback does not latch a flag for this case"
                    );
                } else {
                    invalidated.invalidate(AdmissionError::MissingSnapshot);
                }
                Ok(QueryResult {
                    columns: vec!["_uuid".into()],
                    rows: if empty {
                        vec![]
                    } else {
                        vec![vec![CypherValue::String("b".into())]]
                    },
                })
            }));
            let view = read_fixture_view(Some(a.clone()), a.cell.clone(), conn);
            let error = view.get("Snippet", "b").unwrap_err();
            assert_eq!(
                calls.load(Ordering::SeqCst),
                1,
                "must reach the real get callback, not reject before the DB read"
            );
            assert!(error.to_string().contains(if expiry_without_flag {
                "Expired"
            } else {
                "MissingSnapshot"
            }));
            assert!(a.check().is_err());
        }
    }
}

#[test]
fn direct_admission_cannot_hide_a_foreign_unadmitted_read_view_even_with_a_matching_scope_service()
{
    let a = admission();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = calls.clone();
    let conn = Arc::new(crate::connection::CallbackConnection::new(move |_, _| {
        count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(QueryResult::default())
    }));
    let view = read_fixture_view(
        None,
        crate::scope::Scope::new("org", "foreign"),
        conn.clone(),
    );
    let mut services = ServiceRegistry::new();
    services.register(SERVICE, a.clone());
    services.register("scope", a.cell.clone());
    services.register(
        crate::read_view::READ_VIEW_SERVICE,
        Arc::new(crate::read_view::ReadViewCell::avec(view)),
    );
    let ctx = NodeContext::with_services(Arc::new(services));
    let error = crate::dataflow::generic_search_nodes::occurrence_mask_options(
        &ctx,
        &target(),
        SearchOptions::default(),
        &(conn as Arc<dyn crate::connection::DbConnection>),
    )
    .unwrap_err();
    assert!(error.contains("ForeignCell"));
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(a.check(), Err(AdmissionError::ForeignCell));
}
