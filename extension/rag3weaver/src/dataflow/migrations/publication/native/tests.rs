use super::super::tests::snapshot;
use super::*;
use crate::connection::{CallbackConnection, DbError, QueryResult};
use std::sync::{Arc, Mutex};

#[test]
fn native_values_receipts_and_events_survive_reopens() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("owned");
    let origin = NewPublicationOrigin::fresh();
    let id = origin.identifier().to_owned();
    let (p1, intent, p3) = {
        let mut store =
            OwnedPublicationCatalog::create_new(&root, origin, "create", snapshot("A")).unwrap();
        let p1 = store.observe().unwrap();
        let intent = store.prepare("b", snapshot("B")).unwrap();
        store.publish(&intent).unwrap();
        let back = store.prepare("a", snapshot("A")).unwrap();
        let p3 = store.publish(&back).unwrap().current().clone();
        (p1, intent, p3)
    };
    let mut store = OwnedPublicationCatalog::open_owned(&root, &id).unwrap();
    let current = store.observe().unwrap();
    assert_eq!(current.publication(), p3.publication());
    assert_eq!(
        current.snapshot().fingerprint(),
        p1.snapshot().fingerprint()
    );
    assert_ne!(current.publication(), p1.publication());
    let replay = store.publish(&intent).unwrap();
    assert_eq!(
        replay.disposition(),
        PublicationDisposition::AlreadyCommittedHistorical
    );
    assert_eq!(
        replay.receipt().snapshot().fingerprint(),
        snapshot("B").fingerprint()
    );
    assert_eq!(replay.current().publication(), current.publication());
    assert_eq!(
        store.prepare("b", snapshot("B")).unwrap().encode().unwrap(),
        intent.encode().unwrap()
    );
    let inspection = crate::storage_inspection::inspect_snapshot_on(&store.conn).unwrap();
    assert!(inspection.descriptor.is_none());
    assert!(inspection.history.is_empty());
    assert_eq!(inspection.tables, ["_catalog_meta"]);
}

fn files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(root)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read(p).unwrap(),
            )
        })
        .collect()
}
#[test]
fn native_foreign_legacy_unknown_and_extra_state_refuse_before_writes() {
    for variant in 0..7 {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("owned");
        let origin = NewPublicationOrigin::fresh();
        let id = origin.identifier().to_owned();
        drop(OwnedPublicationCatalog::create_new(&root, origin, "create", snapshot("A")).unwrap());
        if variant != 0 {
            let conn = connection(&root, false).unwrap();
            match variant {
                1 => {
                    conn.execute("CREATE NODE TABLE Foreign(id STRING, PRIMARY KEY(id))")
                        .unwrap();
                }
                2 => metadata::write_value(&conn, &Rag3dbDialect, "foreign", "value").unwrap(),
                3 => metadata::write_value(&conn, &Rag3dbDialect, KEY, "{}").unwrap(),
                4 => {
                    let mut r =
                        transaction(&conn, &mut false, true, |c| read_registry(c, &id)).unwrap();
                    r.version = 2;
                    metadata::write_value(&conn, &Rag3dbDialect, KEY, &canonical(&r).unwrap())
                        .unwrap();
                }
                5 => {
                    conn.execute("MATCH (m:_catalog_meta) DELETE m").unwrap();
                }
                _ => {
                    let mut r =
                        transaction(&conn, &mut false, true, |c| read_registry(c, &id)).unwrap();
                    r.publications.get_mut(&r.head).unwrap().identity.push('0');
                    metadata::write_value(&conn, &Rag3dbDialect, KEY, &canonical(&r).unwrap())
                        .unwrap();
                }
            }
        }
        let before = files(&root);
        let expected = if variant == 0 {
            NewPublicationOrigin::fresh().identifier().to_owned()
        } else {
            id
        };
        assert!(
            OwnedPublicationCatalog::open_owned(&root, &expected).is_err(),
            "variant {variant}"
        );
        assert_eq!(
            files(&root),
            before,
            "refusal wrote files, variant {variant}"
        );
    }
}

#[test]
fn native_creation_refuses_existing_directory_and_never_bootstraps_missing_catalog() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("foreign"), "untouched").unwrap();
    let before = files(dir.path());
    assert!(matches!(
        OwnedPublicationCatalog::create_new(
            dir.path(),
            NewPublicationOrigin::fresh(),
            "create",
            snapshot("A")
        ),
        Err(PublicationError::ExistingPath)
    ));
    assert!(OwnedPublicationCatalog::open_owned(
        dir.path(),
        NewPublicationOrigin::fresh().identifier()
    )
    .is_err());
    assert_eq!(files(dir.path()), before);
    let absent = dir.path().join("absent");
    assert!(OwnedPublicationCatalog::open_owned(
        &absent,
        NewPublicationOrigin::fresh().identifier()
    )
    .is_err());
    assert!(!absent.exists());
}

#[test]
fn transaction_errors_preserve_rollback_and_block_reuse_after_uncertain_commit() {
    for commit_failure in [false, true] {
        for rollback_failure in [false, true] {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let trace = calls.clone();
            let conn = CallbackConnection::new(move |sql, _| {
                trace.lock().unwrap().push(sql.to_owned());
                if (sql == "COMMIT" && commit_failure) || (sql == "ROLLBACK" && rollback_failure) {
                    Err(DbError::QueryError(format!("injected {sql}")))
                } else {
                    Ok(QueryResult::default())
                }
            });
            let mut poisoned = false;
            let result = transaction(&conn, &mut poisoned, false, |_| {
                if commit_failure {
                    Ok(17)
                } else {
                    Err(PublicationError::Invalid("action refused".into()))
                }
            });
            assert!(result.is_err());
            let text = result.unwrap_err().to_string();
            assert_eq!(poisoned, commit_failure || rollback_failure);
            assert_eq!(
                *calls.lock().unwrap(),
                if commit_failure {
                    vec!["BEGIN TRANSACTION", "COMMIT", "ROLLBACK"]
                } else {
                    vec!["BEGIN TRANSACTION", "ROLLBACK"]
                }
            );
            if rollback_failure {
                assert!(text.contains("rollback failed"), "{text}");
            }
            if poisoned {
                let previous = calls.lock().unwrap().len();
                assert!(matches!(
                    transaction(&conn, &mut poisoned, false, |_| Ok(())),
                    Err(PublicationError::NeedsReopen)
                ));
                assert_eq!(calls.lock().unwrap().len(), previous);
            }
        }
    }
}

// Invoked only by the parent test with an owned TempDir, under the same common
// supervisor. process::exit intentionally bypasses Rust/C++ Drop and checkpoint.
#[test]
fn native_interruption_child() {
    let Some(root) = std::env::var_os("CX_PUBLICATION_CRASH_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let origin = std::env::var("CX_PUBLICATION_CRASH_ORIGIN").unwrap();
    let intent = PublicationIntent::decode(
        &std::fs::read_to_string(root.parent().unwrap().join("intent.json")).unwrap(),
    )
    .unwrap();
    let mut store = OwnedPublicationCatalog::open_owned(&root, &origin).unwrap();
    if std::env::var("CX_PUBLICATION_CRASH_PHASE").unwrap() == "before" {
        store.conn.begin().unwrap();
        let mut r = read_registry(&store.conn, &origin).unwrap();
        r.apply(&intent).unwrap();
        write_registry(&store.conn, &r).unwrap();
        std::process::exit(73);
    }
    store.publish(&intent).unwrap();
    std::process::exit(74);
}

#[test]
fn native_interruption_before_commit_and_lost_ack_after_commit_resolve_from_disk() {
    for (phase, code) in [("before", 73), ("after", 74)] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("owned");
        let origin = NewPublicationOrigin::fresh();
        let id = origin.identifier().to_owned();
        let (initial, intent) = {
            let mut store =
                OwnedPublicationCatalog::create_new(&root, origin, "create", snapshot("A"))
                    .unwrap();
            (
                store.observe().unwrap(),
                store.prepare("b", snapshot("B")).unwrap(),
            )
        };
        std::fs::write(dir.path().join("intent.json"), intent.encode().unwrap()).unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "dataflow::migrations::publication::native::tests::native_interruption_child",
                "--nocapture",
            ])
            .env("CX_PUBLICATION_CRASH_ROOT", &root)
            .env("CX_PUBLICATION_CRASH_ORIGIN", &id)
            .env("CX_PUBLICATION_CRASH_PHASE", phase)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(code),
            "{}",
            format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        );
        let mut reopened = OwnedPublicationCatalog::open_owned(&root, &id).unwrap();
        let observed = reopened.observe().unwrap();
        if phase == "before" {
            assert_eq!(observed.publication(), initial.publication());
            assert_eq!(
                observed.snapshot().fingerprint(),
                snapshot("A").fingerprint()
            );
        } else {
            assert_eq!(observed.publication(), intent.target_publication());
            let recovered = reopened.prepare("b", snapshot("B")).unwrap();
            assert_eq!(recovered.encode().unwrap(), intent.encode().unwrap());
            let replay = reopened.publish(&intent).unwrap();
            assert_eq!(
                replay.disposition(),
                PublicationDisposition::AlreadyCommittedAtHead
            );
            assert_eq!(
                replay.receipt().snapshot().fingerprint(),
                snapshot("B").fingerprint()
            );
        }
    }
}

#[test]
fn native_foreign_column_is_rejected_without_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("owned");
    let origin = NewPublicationOrigin::fresh();
    let id = origin.identifier().to_owned();
    drop(OwnedPublicationCatalog::create_new(&root, origin, "create", snapshot("A")).unwrap());
    {
        let conn = connection(&root, false).unwrap();
        conn.execute("ALTER TABLE _catalog_meta ADD foreign STRING")
            .unwrap();
        conn.execute("MATCH (m:_catalog_meta) SET m.foreign = 'outside-closed-contract'")
            .unwrap();
    }
    let before = files(&root);
    assert!(
        OwnedPublicationCatalog::open_owned(&root, &id).is_err(),
        "foreign column admitted by closed declaration catalog"
    );
    assert_eq!(files(&root), before, "refusal changed foreign catalog");
}

#[test]
fn native_complete_recipe_models_relations_and_configs_are_real_persisted_values() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("owned");
    for (index, address) in ["a,b", " a,b ", ""].into_iter().enumerate() {
        let mut malformed = super::super::tests::rich_input();
        malformed
            .recipe
            .model_sources
            .as_mut()
            .unwrap()
            .get_mut(&crate::model_source::Capability::Embed)
            .unwrap()
            .address = crate::model_source::Addresses(vec![address.into()]);
        let refused_root = dir.path().join(format!("refused-{index}"));
        let result = DeclarationSnapshot::from_complete_input(malformed).and_then(|snapshot| {
            OwnedPublicationCatalog::create_new(
                &refused_root,
                NewPublicationOrigin::fresh(),
                "refused",
                snapshot,
            )
        });
        assert!(result.is_err());
        assert!(
            !refused_root.exists(),
            "codec refusal created a native catalog"
        );
    }
    assert!(
        files(dir.path()).is_empty(),
        "codec refusal wrote fixture bytes"
    );
    let origin = NewPublicationOrigin::fresh();
    let id = origin.identifier().to_owned();
    let input = super::super::tests::rich_input();
    let initial = DeclarationSnapshot::from_complete_input(input.clone()).unwrap();
    let expected = canonical(&initial).unwrap();
    let intent = {
        let mut store =
            OwnedPublicationCatalog::create_new(&root, origin, "create", initial).unwrap();
        assert_eq!(
            canonical(store.observe().unwrap().snapshot()).unwrap(),
            expected
        );
        let mut changed = input;
        changed
            .entities
            .as_mut()
            .unwrap()
            .get_mut("Document")
            .unwrap()
            .chunking
            .max_size += 1;
        let intent = store
            .prepare(
                "change-config",
                DeclarationSnapshot::from_complete_input(changed).unwrap(),
            )
            .unwrap();
        store.publish(&intent).unwrap();
        intent
    };
    let mut reopened = OwnedPublicationCatalog::open_owned(&root, &id).unwrap();
    assert_eq!(
        canonical(reopened.observe().unwrap().snapshot()).unwrap(),
        canonical(intent.target()).unwrap()
    );
    let first = transaction(&reopened.conn, &mut false, true, |conn| {
        read_registry(conn, &id)
    })
    .unwrap()
    .known_operation("create")
    .unwrap()
    .unwrap();
    assert_eq!(canonical(first.target()).unwrap(), expected);
    assert_eq!(
        reopened.publish(&intent).unwrap().disposition(),
        PublicationDisposition::AlreadyCommittedAtHead
    );
}
