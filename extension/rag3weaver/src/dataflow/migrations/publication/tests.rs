use super::super::facts::{ProviderFacts, RecipeFactsInput};
use super::*;
use crate::{config::EntityConfig, search::SearchSignals};

pub(super) fn input(label: &str) -> DeclaredSchemaInput {
    DeclaredSchemaInput {
        entities: Some(BTreeMap::from([(
            "Document".into(),
            EntityConfig {
                signals: SearchSignals::BM25,
                ..Default::default()
            },
        )])),
        relations: Some(BTreeMap::new()),
        provider: ProviderFacts::Absent,
        index_signals: BTreeMap::new(),
        recipe: RecipeFactsInput {
            id: label.into(),
            resolved_templates: Some(BTreeMap::new()),
            model_sources: Some(BTreeMap::new()),
            dense_models: Some(BTreeMap::new()),
        },
    }
}
pub(super) fn snapshot(label: &str) -> DeclarationSnapshot {
    DeclarationSnapshot::from_complete_input(input(label)).unwrap()
}
fn registry() -> Registry {
    Registry::genesis(NewPublicationOrigin::fresh(), "creation", snapshot("A")).unwrap()
}

#[test]
fn a_b_a_distinguishes_events_from_content_and_replays_historical_receipts() {
    let mut r = registry();
    let a = r.published(&r.head).unwrap();
    let b = r.prepare("to-b", snapshot("B")).unwrap();
    let first = r.apply(&b).unwrap();
    assert_eq!(first.disposition(), PublicationDisposition::Committed);
    assert_ne!(
        first.current().snapshot().fingerprint(),
        a.snapshot().fingerprint()
    );
    assert_eq!(
        r.apply(&b).unwrap().disposition(),
        PublicationDisposition::AlreadyCommittedAtHead
    );
    let back = r.prepare("back-a", snapshot("A")).unwrap();
    let third = r.apply(&back).unwrap();
    assert_eq!(
        third.current().snapshot().fingerprint(),
        a.snapshot().fingerprint()
    );
    assert_ne!(third.current().publication(), a.publication());
    assert_ne!(third.current().publication(), first.current().publication());
    let encoded = canonical(&r).unwrap();
    let replay = r.apply(&b).unwrap();
    assert_eq!(
        replay.disposition(),
        PublicationDisposition::AlreadyCommittedHistorical
    );
    assert_eq!(
        replay.receipt().publication(),
        first.receipt().publication()
    );
    assert_eq!(
        replay.current().publication(),
        third.current().publication()
    );
    assert_eq!(canonical(&r).unwrap(), encoded);
    assert_eq!(
        r.prepare("to-b", snapshot("B")).unwrap().encode().unwrap(),
        b.encode().unwrap()
    );
    assert_eq!(
        r.published(a.publication())
            .unwrap()
            .snapshot()
            .fingerprint(),
        a.snapshot().fingerprint()
    );
    Registry::decode(&encoded).unwrap();
}

#[test]
fn stale_intent_refuses_same_content_new_event_and_requalification_freezes_new_intent() {
    let mut r = registry();
    let old = r.prepare("pending", snapshot("C")).unwrap();
    let bytes = old.encode().unwrap();
    let b = r.prepare("b", snapshot("B")).unwrap();
    r.apply(&b).unwrap();
    let a = r.prepare("a", snapshot("A")).unwrap();
    r.apply(&a).unwrap();
    assert!(matches!(
        r.apply(&old),
        Err(PublicationError::PublicationMoved)
    ));
    let new = r.prepare("requalified", old.target().clone()).unwrap();
    assert_ne!(new.identity(), old.identity());
    assert_ne!(new.target_publication(), old.target_publication());
    assert_eq!(new.target().fingerprint(), old.target().fingerprint());
    r.apply(&new).unwrap();
    assert_eq!(old.encode().unwrap(), bytes);
    assert_eq!(
        PublicationIntent::decode(&bytes).unwrap().encode().unwrap(),
        bytes
    );
}

#[test]
fn operation_collisions_and_foreign_origin_never_change_registry() {
    let mut r = registry();
    let intent = r.prepare("op", snapshot("B")).unwrap();
    r.apply(&intent).unwrap();
    let original = canonical(&r).unwrap();
    assert!(matches!(
        r.prepare("op", snapshot("C")),
        Err(PublicationError::OperationConflict)
    ));
    let mut changed = intent.stored.body.clone();
    changed.source = SourceRevision::Genesis;
    assert!(matches!(
        r.apply(&PublicationIntent::build(changed).unwrap()),
        Err(PublicationError::OperationConflict)
    ));
    let foreign = registry().prepare("foreign", snapshot("B")).unwrap();
    assert!(matches!(
        r.apply(&foreign),
        Err(PublicationError::OriginMismatch)
    ));
    assert_eq!(canonical(&r).unwrap(), original);
}

#[test]
fn complete_input_and_strict_wire_are_required() {
    let mut incomplete = input("A");
    incomplete.relations = None;
    assert!(matches!(
        DeclarationSnapshot::from_complete_input(incomplete),
        Err(PublicationError::Facts(FactsError::Incomplete(_)))
    ));
    let r = registry();
    let i = r.prepare("b", snapshot("B")).unwrap();
    let wire = i.encode().unwrap();
    for changed in [
        format!(" {wire}"),
        wire.replacen("{", "{\"unknown\":1,", 1),
        wire.replacen("\"version\":1", "\"version\":1,\"version\":1", 1),
        wire.replace("\"identity\":\"", "\"identity\":\"changed-"),
    ] {
        assert!(PublicationIntent::decode(&changed).is_err());
    }
    let mut value: serde_json::Value = serde_json::from_str(&wire).unwrap();
    value["body"]["target"]["entities"]["Document"]["unknown"] = true.into();
    assert!(PublicationIntent::decode(&facts::canonical_json(&value).unwrap()).is_err());
    value = serde_json::from_str(&wire).unwrap();
    value["body"]["version"] = 2.into();
    assert!(matches!(
        PublicationIntent::decode(&facts::canonical_json(&value).unwrap()),
        Err(PublicationError::UnknownContract)
    ));
}

#[test]
fn persisted_chain_rejects_missing_ancestors_forks_cycles_and_corrupt_fingerprints() {
    let mut r = registry();
    let b = r.prepare("b", snapshot("B")).unwrap();
    r.apply(&b).unwrap();
    let a = r.prepare("a", snapshot("A")).unwrap();
    r.apply(&a).unwrap();
    let mut missing = r.clone();
    missing.publications.remove(b.target_publication());
    assert!(Registry::decode(&canonical(&missing).unwrap()).is_err());
    let mut fork = r.clone();
    let forked = r.prepare("fork", snapshot("F")).unwrap();
    fork.publications
        .insert(forked.target_publication().into(), forked.stored);
    assert!(Registry::decode(&canonical(&fork).unwrap()).is_err());
    let mut corrupt = r.clone();
    let mut body = corrupt.publications[&corrupt.head].body.clone();
    body.source = SourceRevision::Publication {
        publication: b.target_publication().into(),
        fingerprint: snapshot("A").fingerprint().into(),
    };
    corrupt.publications.insert(
        corrupt.head.clone(),
        PublicationIntent::build(body).unwrap().stored,
    );
    assert!(Registry::decode(&canonical(&corrupt).unwrap()).is_err());
    let mut cycle = r.clone();
    let genesis = cycle
        .publications
        .iter()
        .find(|(_, i)| matches!(i.body.source, SourceRevision::Genesis))
        .unwrap()
        .0
        .clone();
    let mut body = cycle.publications[&genesis].body.clone();
    body.source = SourceRevision::Publication {
        publication: cycle.head.clone(),
        fingerprint: snapshot("A").fingerprint().into(),
    };
    cycle
        .publications
        .insert(genesis, PublicationIntent::build(body).unwrap().stored);
    assert!(Registry::decode(&canonical(&cycle).unwrap()).is_err());
}

pub(super) fn rich_input() -> DeclaredSchemaInput {
    use crate::{
        backend::BackendRelation,
        config::{DerivedConfig, DerivedIdentity, FieldType, RelationDef, SimpleFieldDef},
        embedding_storage::{EmbeddingModelEntry, StorageKind},
        model_source::{Capability, ModelSource},
    };
    let mut i = input("rich-synthetic");
    let config = i.entities.as_mut().unwrap().get_mut("Document").unwrap();
    config.fields.insert(
        "body".into(),
        SimpleFieldDef {
            field_type: FieldType::Text,
            is_content: true,
            ..Default::default()
        },
    );
    let mut derived = config.clone();
    derived.derived = Some(DerivedConfig {
        from: "Document".into(),
        identity: DerivedIdentity::Content {
            format: "document-content-v1".into(),
        },
        gather: vec![],
        render: BTreeMap::from([("body".into(), "body-template".into())]),
    });
    i.entities
        .as_mut()
        .unwrap()
        .insert("DocumentContent".into(), derived);
    i.relations.as_mut().unwrap().insert(
        "DERIVED_FROM".into(),
        BackendRelation {
            def: RelationDef {
                from: "DocumentContent".into(),
                to: "Document".into(),
                properties: None,
            },
            from_states: Some(vec!["open".into()]),
            to_states: None,
            description: Some("synthetic policy".into()),
        },
    );
    i.recipe
        .resolved_templates
        .as_mut()
        .unwrap()
        .insert("body-template".into(), "{{ root.body }}".into());
    let mut model = ModelSource::local("synthetic-4");
    model.dimensions = Some(4);
    i.recipe
        .model_sources
        .as_mut()
        .unwrap()
        .insert(Capability::Embed, model);
    i.recipe.dense_models.as_mut().unwrap().insert(
        "synthetic-4".into(),
        EmbeddingModelEntry {
            name: "synthetic-4".into(),
            dim: 4,
            storage: StorageKind::Legacy,
        },
    );
    i
}

#[test]
fn snapshot_codec_refuses_non_lossless_declarations_before_any_publication() {
    use crate::model_source::{Addresses, Capability};
    let mut input = rich_input();
    input
        .recipe
        .model_sources
        .as_mut()
        .unwrap()
        .get_mut(&Capability::Embed)
        .unwrap()
        .address = Addresses::parse(" http://127.0.0.1:9, http://127.0.0.1:10 ");
    let normal = DeclarationSnapshot::from_complete_input(input.clone()).unwrap();
    let r = Registry::genesis(NewPublicationOrigin::fresh(), "create", normal).unwrap();
    let encoded = canonical(&r).unwrap();
    assert_eq!(
        canonical(&Registry::decode(&encoded).unwrap()).unwrap(),
        encoded
    );
    input
        .recipe
        .model_sources
        .as_mut()
        .unwrap()
        .get_mut(&Capability::Embed)
        .unwrap()
        .address = Addresses(vec![" http://127.0.0.1:9, http://127.0.0.1:10 ".into()]);
    assert!(
        DeclarationSnapshot::from_complete_input(input).is_err(),
        "non-lossless declaration admitted by publication snapshot"
    );
}
