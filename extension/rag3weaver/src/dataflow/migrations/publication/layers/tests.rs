use super::*;
use crate::dataflow::migrations::publication::{tests::snapshot, NewPublicationOrigin, Registry};

#[derive(Debug, PartialEq, Eq)]
struct Value {
    content_h: &'static str,
    line: u64,
}
fn publications() -> Vec<PublishedDeclaration> {
    let mut registry =
        Registry::genesis(NewPublicationOrigin::fresh(), "anchor", snapshot("same-H")).unwrap();
    let mut values = vec![registry.published(&registry.head).unwrap()];
    for op in ["parent", "child", "anchor-advance"] {
        let intent = registry.prepare(op, snapshot("same-H")).unwrap();
        values.push(registry.apply(&intent).unwrap().current().clone());
    }
    values
}
fn key(layer: &str, publication: &PublishedDeclaration) -> LayerGeneration {
    LayerGeneration::from_publication(layer, publication).unwrap()
}
fn id(uuid: &str) -> OccurrenceKey {
    OccurrenceKey::new("Snippet", uuid).unwrap()
}
fn file(name: &str) -> FileKey {
    FileKey::new(name).unwrap()
}
fn row(uuid: &str, path: &str, h: &'static str, line: u64) -> Occurrence<Value> {
    Occurrence::new(id(uuid), file(path), Value { content_h: h, line })
}
fn revision(
    key: LayerGeneration,
    parent: Option<LayerGeneration>,
    rows: Vec<Occurrence<Value>>,
) -> LayerRevision<Value> {
    LayerRevision::new(key, parent, rows, BTreeSet::new(), BTreeSet::new()).unwrap()
}

#[test]
fn inherited_values_keep_occurrence_coordinates_and_exact_publication_provenance() {
    let p = publications();
    let a = key("anchor", &p[0]);
    let b = key("parent", &p[1]);
    let c = key("child", &p[2]);
    let layers = PinnedLayers::new(vec![
        revision(a.clone(), None, vec![row("h", "h.rs", "shared-H", 7)]),
        revision(
            b.clone(),
            Some(a.clone()),
            vec![row("f", "f.rs", "shared-H", 19)],
        ),
        revision(
            c.clone(),
            Some(b.clone()),
            vec![row("g", "g.rs", "other-H", 31)],
        ),
    ])
    .unwrap();
    let result = layers.resolve(&c).unwrap();
    assert_eq!(result.generation(), &c);
    assert_eq!(result.occurrences().len(), 3);
    let f = &result.occurrences()[&id("f")];
    let h = &result.occurrences()[&id("h")];
    assert_eq!(
        f.occurrence().value().content_h,
        h.occurrence().value().content_h
    );
    assert_eq!(f.occurrence().value().line, 19);
    assert_eq!(h.occurrence().value().line, 7);
    assert_eq!(f.provenance(), &b);
    assert_eq!(h.provenance(), &a);
    assert_eq!(result.occurrences()[&id("g")].provenance(), &c);
}

#[test]
fn tombstones_hide_all_ancestors_and_a_descendant_can_reintroduce_a_file() {
    let p = publications();
    let a = key("anchor", &p[0]);
    let b = key("parent", &p[1]);
    let c = key("child", &p[2]);
    let parent = LayerRevision::new(
        b.clone(),
        Some(a.clone()),
        vec![],
        BTreeSet::from([id("scope")]),
        BTreeSet::from([file("deleted.rs")]),
    )
    .unwrap();
    let layers = PinnedLayers::new(vec![
        revision(
            a.clone(),
            None,
            vec![
                row("scope", "keep.rs", "H", 1),
                row("old1", "deleted.rs", "H", 2),
                row("old2", "deleted.rs", "H", 3),
            ],
        ),
        parent,
        revision(
            c.clone(),
            Some(b.clone()),
            vec![row("new", "deleted.rs", "H", 17)],
        ),
    ])
    .unwrap();
    assert!(layers.resolve(&b).unwrap().occurrences().is_empty());
    let visible = layers.resolve(&c).unwrap();
    assert_eq!(visible.occurrences().len(), 1);
    assert_eq!(
        visible.occurrences()[&id("new")].occurrence().value().line,
        17
    );
    assert_eq!(visible.occurrences()[&id("new")].provenance(), &c);
}

#[test]
fn overrides_do_not_resurrect_an_old_file_and_anchor_advancement_is_not_followed() {
    let p = publications();
    let a1 = key("anchor", &p[0]);
    let b = key("child", &p[1]);
    let a2 = key("anchor", &p[3]);
    assert_eq!(p[0].snapshot().fingerprint(), p[3].snapshot().fingerprint());
    assert_ne!(a1, a2); // P is not H, including A -> A.
    let layers = PinnedLayers::new(vec![
        revision(
            a1.clone(),
            None,
            vec![
                row("same", "old.rs", "H", 1),
                row("inherited", "f.rs", "H", 2),
            ],
        ),
        revision(
            b.clone(),
            Some(a1.clone()),
            vec![row("same", "new.rs", "H", 90)],
        ),
        revision(a2.clone(), None, vec![row("new-anchor", "x.rs", "H", 4)]),
    ])
    .unwrap();
    let result = layers.resolve(&b).unwrap();
    assert_eq!(result.occurrences().len(), 2);
    assert_eq!(
        result.occurrences()[&id("same")].occurrence().file(),
        &file("new.rs")
    );
    assert_eq!(
        result.occurrences()[&id("same")].occurrence().value().line,
        90
    );
    assert!(result.occurrences().contains_key(&id("inherited")));
    assert!(!result.occurrences().contains_key(&id("new-anchor")));
    assert!(matches!(
        layers.select("anchor"),
        Err(LayerError::AmbiguousLayer(_))
    ));
    assert!(matches!(
        layers.select("missing"),
        Err(LayerError::UnknownLayer(_))
    ));
    assert!(matches!(
        layers.resolve(&key("absent", &p[0])),
        Err(LayerError::UnknownGeneration(_))
    ));
}

#[test]
fn malformed_piles_and_ambiguous_local_changes_are_refused_without_partial_resolution() {
    let p = publications();
    let a = key("anchor", &p[0]);
    let b = key("child", &p[1]);
    assert!(matches!(
        PinnedLayers::new(vec![revision(b.clone(), Some(a.clone()), vec![])]),
        Err(LayerError::MissingParent(_))
    ));
    assert!(matches!(
        PinnedLayers::new(vec![
            revision(a.clone(), Some(b.clone()), vec![]),
            revision(b.clone(), Some(a.clone()), vec![])
        ]),
        Err(LayerError::Cycle(_))
    ));
    assert!(matches!(
        PinnedLayers::new(vec![
            revision(a.clone(), None, vec![]),
            revision(a.clone(), None, vec![])
        ]),
        Err(LayerError::DuplicateGeneration(_))
    ));
    assert!(matches!(
        LayerRevision::new(
            a.clone(),
            None,
            vec![row("x", "a.rs", "H", 1), row("x", "b.rs", "H", 2)],
            BTreeSet::new(),
            BTreeSet::new()
        ),
        Err(LayerError::DuplicateOccurrence(_))
    ));
    assert!(matches!(
        LayerRevision::new(
            a.clone(),
            None,
            vec![row("x", "a.rs", "H", 1)],
            BTreeSet::from([id("x")]),
            BTreeSet::new()
        ),
        Err(LayerError::ConflictingTombstone(_))
    ));
    let foreign = publications();
    let f = key("foreign", &foreign[0]);
    assert!(matches!(
        PinnedLayers::new(vec![
            revision(a.clone(), None, vec![]),
            revision(f.clone(), Some(a.clone()), vec![])
        ]),
        Err(LayerError::ForeignOrigin)
    ));
    assert!(LayerGeneration::from_publication(" ", &p[0]).is_err());
    assert!(FileKey::new("").is_err());
    assert!(OccurrenceKey::new("", "x").is_err());
}
