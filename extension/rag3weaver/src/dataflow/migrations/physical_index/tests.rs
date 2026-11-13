use super::*;
use crate::{
    chunker::{Chunker, ChunkerConfig},
    config::{DerivedConfig, DerivedIdentity, EntityConfig, EntityDef, FieldType, SimpleFieldDef},
    dataflow::{
        derive_nodes::{derived_content_uuid, render_context, render_fields, render_inputs_hash},
        node_factories::register_builtins,
        node_registry::NodeRegistry,
    },
    search::SearchSignals,
};
use std::{
    collections::{BTreeSet, HashMap},
    process::Command,
};

const FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/arbre-principal/13-11-2026/raccord-physique/fixture.json"
));
fn text(value: &str) -> CypherValue {
    CypherValue::String(value.into())
}
fn field(kind: FieldType, content: bool) -> SimpleFieldDef {
    SimpleFieldDef {
        field_type: kind,
        is_content: content,
        ..Default::default()
    }
}
fn declarations() -> DeclarationSnapshot {
    use super::super::facts::{DeclaredSchemaInput, ProviderFacts, RecipeFactsInput};
    let source = EntityConfig {
        fields: HashMap::from([
            ("occurrence_id".into(), field(FieldType::String, false)),
            ("name".into(), field(FieldType::String, false)),
            ("docstring".into(), field(FieldType::Text, false)),
            ("body".into(), field(FieldType::Text, true)),
            ("file".into(), field(FieldType::String, false)),
            ("start_line".into(), field(FieldType::Int64, false)),
            ("end_line".into(), field(FieldType::Int64, false)),
        ]),
        hashsafe: Some(vec!["occurrence_id".into()]),
        chunked: Some(false),
        search_index: Some("SnippetContent".into()),
        signals: SearchSignals::BM25,
        ..Default::default()
    };
    let content = EntityConfig {
        fields: HashMap::from([("content".into(), field(FieldType::Text, true))]),
        signals: SearchSignals::BM25,
        derived: Some(DerivedConfig {
            from: "Snippet".into(),
            identity: DerivedIdentity::Content {
                format: "synthetic-snippet-content:v1".into(),
            },
            gather: vec![],
            render: BTreeMap::from([(
                "content".into(),
                "{{ root.name }}\n{{ root.docstring }}\n{{ root.body }}".into(),
            )]),
        }),
        ..Default::default()
    };
    DeclarationSnapshot::from_complete_input(DeclaredSchemaInput {
        entities: Some(BTreeMap::from([
            ("Snippet".into(), source),
            ("SnippetContent".into(), content),
        ])),
        relations: Some(BTreeMap::new()),
        provider: ProviderFacts::Absent,
        index_signals: BTreeMap::new(),
        recipe: RecipeFactsInput {
            id: "synthetic-initial-bm25:v1".into(),
            resolved_templates: Some(BTreeMap::new()),
            model_sources: Some(BTreeMap::new()),
            dense_models: Some(BTreeMap::new()),
        },
    })
    .unwrap()
}
fn records() -> Vec<BTreeMap<String, CypherValue>> {
    let fixture: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    fixture["occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| {
            BTreeMap::from([
                (
                    "occurrence_id".into(),
                    text(o["occurrence_uuid"].as_str().unwrap()),
                ),
                (
                    "name".into(),
                    text(o["render_input"]["name"].as_str().unwrap()),
                ),
                (
                    "docstring".into(),
                    text(o["render_input"]["docstring"].as_str().unwrap()),
                ),
                (
                    "body".into(),
                    text(o["render_input"]["body"].as_str().unwrap()),
                ),
                ("file".into(), text(o["position"]["file"].as_str().unwrap())),
                (
                    "start_line".into(),
                    CypherValue::Int(o["position"]["start_line"].as_i64().unwrap()),
                ),
                (
                    "end_line".into(),
                    CypherValue::Int(o["position"]["end_line"].as_i64().unwrap()),
                ),
                (
                    crate::scope::ORG_COLUMN.into(),
                    text(o["cell"]["org"].as_str().unwrap()),
                ),
                (
                    crate::scope::PROJECT_COLUMN.into(),
                    text(o["cell"]["project"].as_str().unwrap()),
                ),
            ])
        })
        .collect()
}
fn cells() -> Vec<crate::scope::Scope> {
    vec![
        crate::scope::Scope::new("synthetic", "Alpha"),
        crate::scope::Scope::new("synthetic", "Beta"),
    ]
}
fn occurrence_uuid(record: &BTreeMap<String, CypherValue>) -> String {
    Catalog::uuid_for_in(
        &record_scope(record).unwrap(),
        "Snippet",
        &EntityDef {
            fields: HashMap::new(),
            hashsafe: Some(vec!["occurrence_id".into()]),
            derived_from: None,
        },
        record,
    )
}
fn runner(dir: &Path) -> (MigrationRunner, BTreeMap<String, String>, Vec<LexicalProbe>) {
    std::fs::create_dir_all(dir).unwrap();
    let all = records();
    let mut variables = BTreeMap::new();
    let mut probes = Vec::new();
    for (index, cell) in cells().iter().enumerate() {
        let rows = all
            .iter()
            .filter(|row| record_scope(row).unwrap() == *cell)
            .cloned()
            .collect::<Vec<_>>();
        let key = format!("records_{index}");
        variables.insert(key.clone(), encode(&rows).unwrap());
        std::fs::write(dir.join(format!("{:03}_ingest_{}.mmd",index+1,cell.project)),
            format!("graph TD\ningest[\"IngestEntitiesNode(entity='Snippet', records_json='${key}')\"]\n")).unwrap();
        let expected = rows.iter().map(occurrence_uuid).collect();
        probes.push(LexicalProbe {
            scope: cell.clone(),
            target: "Snippet".into(),
            query: "alpha".into(),
            expected_occurrences: expected,
        });
        // Names and documentation must affect actual lexical retrieval too.
        for (query, ids) in [("renamed", vec!["Alpha-E"]), ("another", vec!["Alpha-D"])] {
            probes.push(LexicalProbe {
                scope: cell.clone(),
                target: "Snippet".into(),
                query: query.into(),
                expected_occurrences: rows
                    .iter()
                    .filter(|row| ids.contains(&row["occurrence_id"].as_str().unwrap()))
                    .map(occurrence_uuid)
                    .collect(),
            });
        }
    }
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let mut runner = MigrationRunner::new(Arc::new(registry));
    runner.add_dir(dir.into());
    (runner, variables, probes)
}
fn verify_exact_fixture(owned: &OwnedPhysicalIndex) {
    let catalog = owned.catalog().unwrap();
    let cat = catalog.lock().unwrap();
    let rows=cat.execute_raw("MATCH (s:Snippet) RETURN s._uuid,s.occurrence_id,s.name,s.docstring,s.body,s.file,s.start_line,s.end_line,s._org,s._project ORDER BY s.occurrence_id").unwrap().rows;
    let expected = records()
        .iter()
        .map(|record| {
            vec![
                text(&occurrence_uuid(record)),
                record["occurrence_id"].clone(),
                record["name"].clone(),
                record["docstring"].clone(),
                record["body"].clone(),
                record["file"].clone(),
                record["start_line"].clone(),
                record["end_line"].clone(),
                record[crate::scope::ORG_COLUMN].clone(),
                record[crate::scope::PROJECT_COLUMN].clone(),
            ]
        })
        .collect::<Vec<_>>();
    assert_eq!(rows, expected);
    let declaration = declarations();
    let content = &declaration.supplied_facts().entities()["SnippetContent"];
    let derived = content.derived.as_ref().unwrap();
    let mut contents = BTreeMap::new();
    let mut references = Vec::new();
    let mut chunks = Vec::new();
    for mut record in records() {
        let scope = record_scope(&record).unwrap();
        let uuid = occurrence_uuid(&record);
        record.remove(crate::scope::ORG_COLUMN);
        record.remove(crate::scope::PROJECT_COLUMN);
        record.insert("_uuid".into(), text(&uuid));
        let context = render_context(&record, &BTreeMap::new());
        let rendered = render_fields(derived, &context).unwrap();
        let version = derived_content_uuid(
            "SnippetContent",
            "synthetic-snippet-content:v1",
            content,
            &rendered,
            &scope,
        )
        .unwrap();
        let input_hash = render_inputs_hash(
            &serde_json::json!({"context":context,"derived":derived,"chunking":content.chunking,"chunked":content.chunked,"contentFields":content.content_fields(),"titleField":content.title_field()}),
        );
        references.push(vec![text(&version), text(&uuid), text(&input_hash)]);
        contents.insert(
            version.clone(),
            vec![
                text(&version),
                text(&rendered["content"]),
                text(&scope.org),
                text(&scope.project),
            ],
        );
        for chunk in
            Chunker::new(ChunkerConfig::from(&content.chunking)).chunk(&rendered["content"])
        {
            chunks.push(vec![
                text(&crate::uuid::chunk_uuid(&version, "content", chunk.index)),
                text(&version),
                text("content"),
                text(&chunk.text),
                CypherValue::Int(chunk.index as i64),
                CypherValue::Int(chunk.start_byte as i64),
                CypherValue::Int(chunk.end_byte as i64),
                CypherValue::Int(chunk.start_line as i64),
                CypherValue::Int(chunk.end_line as i64),
                text(&scope.org),
                text(&scope.project),
            ]);
        }
    }
    let sort = |mut rows: Vec<Vec<CypherValue>>| {
        rows.sort_by_key(|row| encode(row).unwrap());
        rows
    };
    assert_eq!(contents.len(), 6);
    assert_eq!(references.len(), 8);
    assert_eq!(
        sort(
            cat.execute_raw("MATCH (c:SnippetContent) RETURN c._uuid,c.content,c._org,c._project")
                .unwrap()
                .rows
        ),
        sort(contents.into_values().collect())
    );
    assert_eq!(sort(cat.execute_raw("MATCH (c:SnippetContent)-[r:SnippetContent_DERIVED_FROM]->(s:Snippet) RETURN c._uuid,s._uuid,r._render_hash").unwrap().rows),sort(references));
    // Repeated references do not create repeated chunks.
    chunks = sort(chunks);
    chunks.dedup();
    assert_eq!(sort(cat.execute_raw("MATCH (c:SnippetContent_Chunk) RETURN c._uuid,c._parent_uuid,c._parent_field,c._text,c._index,c._start_char,c._end_char,c._start_line,c._end_line,c._org,c._project").unwrap().rows),chunks);
    let cross=cat.execute_raw("MATCH (c:SnippetContent)-[:SnippetContent_DERIVED_FROM]->(s:Snippet) WHERE c._org <> s._org OR c._project <> s._project RETURN c._uuid").unwrap();
    assert!(cross.rows.is_empty());
    assert!(cat.registered_embedding_models().unwrap().is_empty());
    assert!(cat.current_embedding_slug().is_none());
    assert!(!read_metadata(cat.conn())
        .unwrap()
        .keys()
        .any(|key| key.starts_with(crate::embedding_storage::META_PREFIX)));
    let schema = PhysicalSnapshot::read(cat.conn()).unwrap();
    assert!(
        !schema
            .schema
            .values()
            .any(|definition| definition.contains("FLOAT[")),
        "no vector columns invented"
    );
}

#[test]
fn native_initial_batch_publishes_exact_values_and_reopens_without_a_model() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("owned");
    let source = NewPublicationOrigin::fresh();
    let origin = source.identifier().to_string();
    let (runner, vars, probes) = runner(&dir.path().join("migrations"));
    let mut index = OwnedPhysicalIndex::create_new(&root, source, declarations(), cells()).unwrap();
    let before = index.observe_published().unwrap();
    let plan = index
        .prepare_initial_batch(&runner, "initial-eight", vars, probes)
        .unwrap();
    let frozen = encode(&plan.stored).unwrap();
    let receipt = index.execute_initial_batch(&runner, &plan).unwrap();
    verify_exact_fixture(&index);
    assert_ne!(before.publication(), receipt.state().publication());
    assert_ne!(
        before.logical_fingerprint().unwrap(),
        receipt.state().logical_fingerprint().unwrap()
    );
    assert_eq!(
        receipt.declaration().snapshot().fingerprint(),
        declarations().fingerprint()
    );
    assert_eq!(encode(&plan.stored).unwrap(), frozen);
    assert_eq!(
        encode(&index.execute_initial_batch(&runner, &plan).unwrap()).unwrap(),
        encode(&receipt).unwrap()
    );
    drop(index);
    let reopened = OwnedPhysicalIndex::open_owned(&root, &origin).unwrap();
    verify_exact_fixture(&reopened);
    assert_eq!(
        encode(&reopened.resolve_operation("initial-eight").unwrap()).unwrap(),
        encode(&receipt).unwrap()
    );
    println!(
        "PHYSICAL_WITNESS {}",
        serde_json::json!({
            "stage":"normal-reopen","origin":receipt.state().origin(),"publication":receipt.state().publication(),
            "logical":receipt.state().logical_fingerprint().unwrap(),"lexical":receipt.state().materialization_fingerprint().unwrap(),
            "plan":receipt.plan_identity(),"occurrences":8,"content_versions":6,"references":8,"cells":2,
            "engine_storage_version":rag3db::get_storage_version(),"dense_models":0,
        })
    );
}

#[test]
fn native_legacy_foreign_schema_and_values_refuse_owned_open_without_writes() {
    for change in 0..4 {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("owned");
        let source = NewPublicationOrigin::fresh();
        let origin = source.identifier().to_string();
        drop(OwnedPhysicalIndex::create_new(&root, source, declarations(), cells()).unwrap());
        if change > 0 {
            let conn = connection(&root, false).unwrap();
            match change {
                1 => {
                    conn.execute("ALTER TABLE Snippet ADD foreign_value STRING DEFAULT NULL")
                        .unwrap();
                }
                2 => {
                    metadata::write_value(
                        &conn,
                        &Rag3dbDialect,
                        "foreign_metadata",
                        "foreign bytes",
                    )
                    .unwrap();
                }
                _ => {
                    conn.execute(
                        "MATCH (n:_catalog_meta) WHERE n._key = 'owned_physical_index:v1' DELETE n",
                    )
                    .unwrap();
                }
            }
        }
        let before = std::fs::read(root.join(DB_FILE)).unwrap();
        let result = OwnedPhysicalIndex::open_owned(
            &root,
            if change == 0 {
                "foreign-origin"
            } else {
                &origin
            },
        );
        assert!(result.is_err());
        assert_eq!(std::fs::read(root.join(DB_FILE)).unwrap(), before);
    }
}

#[test]
fn native_normal_catalog_open_refuses_owned_profile_before_initialization() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("owned");
    drop(
        OwnedPhysicalIndex::create_new(
            &root,
            NewPublicationOrigin::fresh(),
            declarations(),
            cells(),
        )
        .unwrap(),
    );
    let before = std::fs::read(root.join(DB_FILE)).unwrap();
    let mut normal = Catalog::without_embedding_model(
        Box::new(connection(&root, false).unwrap()),
        CatalogConfig::default(),
    );
    assert!(normal.initialize().is_err());
    drop(normal);
    assert_eq!(std::fs::read(root.join(DB_FILE)).unwrap(), before);
}

#[test]
fn physical_crash_child() {
    let Ok(root) = std::env::var("RAG3WEAVER_PHYSICAL_CHILD_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let support = root.parent().unwrap();
    let seed = NewPublicationOrigin::fresh();
    std::fs::write(support.join("origin"), seed.identifier()).unwrap();
    let (runner, vars, probes) = runner(&support.join("migrations"));
    let mut index = OwnedPhysicalIndex::create_new(&root, seed, declarations(), cells()).unwrap();
    let plan = index
        .prepare_initial_batch(&runner, "crash-eight", vars, probes)
        .unwrap();
    std::fs::write(support.join("plan"), encode(&plan.stored).unwrap()).unwrap();
    index.execute_initial_batch(&runner, &plan).unwrap();
    panic!("crash boundary not reached");
}

#[test]
fn native_process_death_recovers_same_program_and_lost_ack_once() {
    for stage in [
        "before-logical-commit",
        "after-logical-commit",
        "after-lexical-durability",
        "after-publication",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("owned");
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "dataflow::migrations::physical_index::tests::physical_crash_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("RAG3WEAVER_PHYSICAL_CHILD_ROOT", &root)
            .env("RAG3WEAVER_PHYSICAL_CRASH", stage)
            .output()
            .unwrap();
        assert_eq!(
            child.status.code(),
            Some(91),
            "{stage}: {} {}",
            String::from_utf8_lossy(&child.stdout),
            String::from_utf8_lossy(&child.stderr)
        );
        let origin = std::fs::read_to_string(dir.path().join("origin")).unwrap();
        let frozen = std::fs::read_to_string(dir.path().join("plan")).unwrap();
        if stage != "after-publication" {
            assert!(OwnedPhysicalIndex::open_owned(&root, &origin).is_err());
        }
        // Delete source files owned by this fixture. Recovery must use the exact
        // graph and records in the existing journal, not the old environment.
        for entry in std::fs::read_dir(dir.path().join("migrations")).unwrap() {
            std::fs::remove_file(entry.unwrap().path()).unwrap();
        }
        let mut registry = NodeRegistry::new();
        register_builtins(&mut registry);
        let runner = MigrationRunner::new(Arc::new(registry));
        let (index, receipt) =
            OwnedPhysicalIndex::recover_initial_batch(&root, &origin, &runner, "crash-eight")
                .unwrap();
        verify_exact_fixture(&index);
        let saved = {
            let cat = index.catalog().unwrap();
            let guard = cat.lock().unwrap();
            journal(guard.conn()).unwrap().unwrap().plan
        };
        assert_eq!(
            encode(&saved.stored).unwrap(),
            frozen,
            "same plan after {stage}"
        );
        let resolved = index.resolve_operation("crash-eight").unwrap();
        assert_eq!(encode(&resolved).unwrap(), encode(&receipt).unwrap());
        assert!(index.resolve_operation("other-operation").is_err());
        println!(
            "PHYSICAL_WITNESS {}",
            serde_json::json!({
                "stage":stage,"origin":receipt.state().origin(),"publication":receipt.state().publication(),
                "logical":receipt.state().logical_fingerprint().unwrap(),"lexical":receipt.state().materialization_fingerprint().unwrap(),
                "plan":receipt.plan_identity(),"occurrences":8,"content_versions":6,"references":8,"cells":2,
                "engine_storage_version":rag3db::get_storage_version(),"dense_models":0,
            })
        );
        let no_dup = BTreeSet::from_iter(records().iter().map(occurrence_uuid));
        assert_eq!(no_dup.len(), 8);
    }
}

#[test]
fn native_failed_lexical_closure_has_no_receipt_and_is_never_served() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("owned");
    let seed = NewPublicationOrigin::fresh();
    let origin = seed.identifier().to_string();
    let (runner, vars, mut probes) = runner(&dir.path().join("migrations"));
    probes[0].expected_occurrences = vec!["impossible-occurrence".into()];
    let mut index = OwnedPhysicalIndex::create_new(&root, seed, declarations(), cells()).unwrap();
    let plan = index
        .prepare_initial_batch(&runner, "wrong-closure", vars, probes)
        .unwrap();
    let error = index.execute_initial_batch(&runner, &plan).unwrap_err();
    assert!(
        error.to_string().contains("lexical closure failed"),
        "{error}"
    );
    assert!(index.observe_published().is_err());
    drop(index);
    assert!(OwnedPhysicalIndex::open_owned(&root, &origin).is_err());
    let conn = connection(&root, true).unwrap();
    let entry = journal(&conn).unwrap().unwrap();
    assert_eq!(entry.status, "pending");
    assert_eq!(entry.committed.unwrap().next_step, 2);
    assert_ne!(
        read_registry(&conn).unwrap().head(),
        plan.target_publication()
    );
}

#[test]
fn native_unowned_cell_or_changed_source_refuses_before_new_effects() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("owned");
    let seed = NewPublicationOrigin::fresh();
    let (runner, vars, probes) = runner(&dir.path().join("migrations"));
    let index =
        OwnedPhysicalIndex::create_new(&root, seed, declarations(), vec![cells()[0].clone()])
            .unwrap();
    assert!(index
        .prepare_initial_batch(&runner, "foreign-cell", vars.clone(), probes.clone())
        .is_err());
    drop(index);
    let root = dir.path().join("both");
    let mut index = OwnedPhysicalIndex::create_new(
        &root,
        NewPublicationOrigin::fresh(),
        declarations(),
        cells(),
    )
    .unwrap();
    let plan = index
        .prepare_initial_batch(&runner, "source-change", vars, probes)
        .unwrap();
    let catalog = index.catalog().unwrap();
    catalog
        .lock()
        .unwrap()
        .execute_raw("ALTER TABLE Snippet ADD foreign_value STRING DEFAULT NULL")
        .unwrap();
    let before = PhysicalSnapshot::read(catalog.lock().unwrap().conn()).unwrap();
    drop(catalog);
    assert!(index.execute_initial_batch(&runner, &plan).is_err());
    drop(index);
    let conn = connection(&root, true).unwrap();
    assert_eq!(PhysicalSnapshot::read(&conn).unwrap(), before);
    assert!(journal(&conn).unwrap().is_none());
}

#[test]
fn frozen_program_codec_binds_sources_records_and_probe_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("owned");
    let (runner, vars, probes) = runner(&dir.path().join("migrations"));
    let index = OwnedPhysicalIndex::create_new(
        &root,
        NewPublicationOrigin::fresh(),
        declarations(),
        cells(),
    )
    .unwrap();
    let plan = index
        .prepare_initial_batch(&runner, "frozen-codec", vars, probes)
        .unwrap();
    let bytes = encode(&plan.stored).unwrap();
    assert_eq!(
        StoredInitialPlan::restore(&bytes).unwrap().identity(),
        plan.identity()
    );
    let mut value: serde_json::Value = serde_json::from_str(&bytes).unwrap();
    value["probes"][0]["query"] = serde_json::json!("different");
    assert!(StoredInitialPlan::restore(&value.to_string()).is_err());
    let mut value: serde_json::Value = serde_json::from_str(&bytes).unwrap();
    value["program"]["steps"][0]["content"] =
        serde_json::json!("graph TD\nx[\"CypherNode(query='RETURN 1')\"]\n");
    assert!(StoredInitialPlan::restore(&value.to_string()).is_err());
}
