//! E2E : le journal d'une conversation se relit par le saut du dialecte, et
//! rend les mêmes lignes que la requête Cypher qu'il remplace.
//!
//! La parité se prouve sur une base vivante : les deux requêtes tournent sur
//! les mêmes messages, et leurs lignes se comparent champ à champ (l'uuid de
//! la conversation en tête du saut mis à part).
//!
//! Run with: ./run_e2e.sh --test e2e_journal_de_conversation

#![cfg(feature = "rag3db-native")]

use rag3weaver::connection::{CypherValue, QueryParam};
use rag3weaver::dataflow::trace_nodes::{record_runs_and_messages, register_trace_schema, CONVERSATION_ENTITY, IN_CONVERSATION, MESSAGE_ENTITY};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

fn setup() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    assert!(std::path::Path::new(&ext).exists(), "vector extension not found at {ext} — ./run_e2e.sh --build-only");
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("journal-e2e".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_trace_schema(&mut catalog).unwrap();
    catalog
}

#[test]
#[ignore]
fn le_journal_par_le_dialecte_rend_les_lignes_d_avant() {
    let mut cat = setup();
    // Deux fils : celui qu'on relit, et un autre qui ne doit pas s'y mêler.
    let messages = [
        (1_000, "alice", "bob", "fil-a", "premier"),
        (2_000, "bob", "alice", "fil-a", "réponse"),
        (2_100, "alice", "bob", "fil-a", "juste après"),
        (2_500, "carol", "dan", "fil-b", "ailleurs"),
        (3_000, "alice", "bob", "fil-a", "dernier"),
    ];
    for (at_ms, from, to, conv, content) in messages {
        let e = serde_json::json!({"kind": "Message", "from": from, "to": to, "conversation": conv, "content": content, "at_ms": at_ms});
        record_runs_and_messages(&mut cat, std::slice::from_ref(&e), at_ms).unwrap();
    }

    for since in [0_i64, 2_000, 9_999] {
        let avant = cat
            .execute_raw_with_params(
                "MATCH (m:Message)-[:IN_CONVERSATION]->(c:Conversation) \
                 WHERE c.conversation_id = $conversation AND m.at_ms >= $since \
                 RETURN m.at_ms, m.at, m.from, m.to, m.content ORDER BY m.at_ms, m.seq",
                &[QueryParam::new("conversation", "fil-a"), QueryParam::new("since", since)],
            )
            .unwrap()
            .rows;

        let cle = std::collections::BTreeMap::from([("conversation_id".to_string(), CypherValue::String("fil-a".into()))]);
        let uuid = cat.entity_uuid(CONVERSATION_ENTITY, &cle).unwrap();
        let mut saut = rag3weaver_ir::Hop::new(CONVERSATION_ENTITY, IN_CONVERSATION, MESSAGE_ENTITY, rag3weaver_ir::Direction::Incoming);
        saut.returns = ["at_ms", "at", "from", "to", "content"].map(|f| rag3weaver_ir::Column::Node(f.into())).to_vec();
        saut.filter = Some(rag3weaver_ir::Predicate::AtLeast { field: "at_ms".into(), param: "since".into() });
        saut.order_by = vec!["at_ms".into(), "seq".into()];
        let q = cat.dialect_arc().hop(&saut).unwrap();
        let apres: Vec<Vec<CypherValue>> = cat
            .execute_raw_with_params(
                &q,
                &[
                    QueryParam::new("uuids", CypherValue::List(vec![CypherValue::String(uuid.clone())])),
                    QueryParam::new("since", since),
                ],
            )
            .unwrap()
            .rows
            .into_iter()
            .map(|r| {
                assert_eq!(r.first().and_then(|v| v.as_str()), Some(uuid.as_str()), "l'uuid de la conversation en tête");
                r.into_iter().skip(1).collect()
            })
            .collect();

        assert_eq!(apres, avant, "depuis {since} : les mêmes lignes, dans le même ordre");
        let attendus = [4usize, 3, 0][[0_i64, 2_000, 9_999].iter().position(|s| *s == since).unwrap()];
        assert_eq!(avant.len(), attendus, "depuis {since} : le fil-a seul");
    }
}
