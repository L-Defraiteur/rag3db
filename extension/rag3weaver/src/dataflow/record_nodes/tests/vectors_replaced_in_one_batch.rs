//! The witness of the lot `vector-replace-batch` (22 November 2026): vectors are written in one statement per packet.
use super::*;
use std::sync::{Arc, Mutex};

/// **A packet of vectors is one statement, even when every row already carries one**: no line-by-line pass through
/// NULL (the workaround of 3 October 2026, retired when the core replayed every replacement case of the engine green).
/// The connection answers as if every row carried a vector; the old path asked which, then sent two statements per row.
#[test]
fn a_packet_of_vectors_is_one_statement_even_when_rows_carry_one() {
    let sent: Arc<Mutex<Vec<(String, usize)>>> = Arc::default();
    let seen = sent.clone();
    let conn = crate::connection::CallbackConnection::new(move |cypher, params| {
        let n = params.first().map(|p| match &p.value {
            CypherValue::List(l) => l.len(),
            _ => 0,
        });
        seen.lock().unwrap().push((cypher.to_string(), n.unwrap_or(0)));
        // Every row cited carries a vector already.
        let rows = match params.first().map(|p| &p.value) {
            Some(CypherValue::List(l)) => l
                .iter()
                .filter_map(|i| match i {
                    CypherValue::Map(m) => m.get("uuid").cloned(),
                    _ => None,
                })
                .map(|u| vec![u])
                .collect(),
            _ => Vec::new(),
        };
        Ok(crate::connection::QueryResult { rows, ..Default::default() })
    });
    let items: Vec<CypherValue> = (0..100)
        .map(|i| {
            CypherValue::Map(BTreeMap::from([
                ("uuid".to_string(), CypherValue::String(format!("u{i}"))),
                ("hash".to_string(), CypherValue::String(format!("h{i}"))),
                ("emb".to_string(), CypherValue::List(vec![CypherValue::Float(0.5); 4])),
            ]))
        })
        .collect();
    write_vectors(&conn, &crate::dialect::Rag3dbDialect, "ScopeContent_Chunk", "embedding", "_embed_hash", items).unwrap();
    let sent = sent.lock().unwrap();
    assert_eq!(sent.len(), 1, "one statement for the packet, not {}: {:?}", sent.len(), sent.iter().map(|(c, n)| (c.chars().take(60).collect::<String>(), *n)).take(4).collect::<Vec<_>>());
    assert_eq!(sent[0].1, 100, "the whole packet in it");
    assert!(sent[0].0.contains("SET") && !sent[0].0.contains("NULL"), "{}", sent[0].0);
}
