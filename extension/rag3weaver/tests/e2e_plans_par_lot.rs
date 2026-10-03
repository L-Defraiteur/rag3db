//! E2E : **aucune requête par lot ne balaie une table** (3 octobre 2026).
//!
//! `UNWIND $items AS item MATCH (n:T {_uuid: item.champ})` faisait un produit
//! cartésien de la table et du lot : le moteur ne joint par hachage que sur
//! une variable simple. Toutes les formes du dialecte passent maintenant par
//! `dialect::unwind_par_cle`, qui extrait la clé dans un `WITH`. Ce test lit
//! le plan (`EXPLAIN`) de chacune et refuse tout `CROSS_PRODUCT` : c'est lui
//! qui empêche le retour du balayage. Il exécute aussi chaque forme, pour
//! qu'une forme acceptée par le planificateur mais refusée à l'exécution
//! (« Cannot evaluate expression with type VARIABLE ») ne passe pas.
//!
//! Run with: ./run_e2e.sh --test e2e_plans_par_lot
#![cfg(feature = "rag3db-native")]

use std::collections::BTreeMap;

use rag3weaver::connection::{CypherValue, DbConnection, QueryParam};
use rag3weaver::dialect::{Rag3dbDialect, SchemaDialect};
use rag3weaver::Rag3dbConnection;

fn s(x: &str) -> CypherValue {
    CypherValue::String(x.into())
}

fn ligne(pairs: &[(&str, CypherValue)]) -> CypherValue {
    CypherValue::Map(pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect::<BTreeMap<_, _>>())
}

fn lot(n: usize, f: impl Fn(usize) -> CypherValue) -> CypherValue {
    CypherValue::List((0..n).map(f).collect())
}

/// Une base où chaque forme a ses tables, assez grosses pour que le
/// planificateur ait un choix à faire.
fn base() -> Rag3dbConnection {
    let c = Rag3dbConnection::in_memory().expect("base en mémoire");
    let d = Rag3dbDialect;
    for ddl in [
        "CREATE NODE TABLE A(_uuid STRING, x STRING, mk STRING, _sparse_hash STRING, _absent_since INT64, emb FLOAT[4], PRIMARY KEY(_uuid))",
        "CREATE NODE TABLE B(_uuid STRING, x STRING, PRIMARY KEY(_uuid))",
        "CREATE NODE TABLE I(_uuid STRING, x STRING, PRIMARY KEY(_uuid))",
        "CREATE NODE TABLE _index_blobs(_key STRING, _data BLOB, _deleted_gen INT64, PRIMARY KEY(_key))",
        "CREATE REL TABLE R(FROM A TO B, p STRING)",
    ] {
        c.execute(ddl).unwrap();
    }
    c.execute(&d.create_aside_table()).unwrap();
    for k in 0..2000 {
        c.execute(&format!("CREATE (:A {{_uuid: 'a{k}', x: 'x', mk: '', _sparse_hash: ''}})")).unwrap();
        c.execute(&format!("CREATE (:B {{_uuid: 'b{k}', x: 'x'}})")).unwrap();
    }
    c.execute("MATCH (a:A), (b:B) WHERE a._uuid = 'a1' AND b._uuid = 'b1' CREATE (a)-[:R {p: 'p'}]->(b)").unwrap();
    c
}

#[test]
#[ignore]
fn aucune_forme_par_lot_ne_balaie_une_table() {
    let c = base();
    let d = Rag3dbDialect;
    let par_uuid = |champ: &'static str| lot(300, move |k| ligne(&[(champ, s(&format!("a{k}"))), ("x", s("z")), ("state", s("z"))]));
    let liens = |de: &'static str, a: &'static str| {
        lot(300, move |k| ligne(&[(de, s(&format!("a{k}"))), (a, s(&format!("b{k}"))), ("p", s("q"))]))
    };
    let p = |nom: &str, v: CypherValue| vec![QueryParam::new(nom, v)];

    let formes: Vec<(&str, String, Vec<QueryParam>)> = vec![
        ("batch_upsert", d.batch_upsert("A", &["_uuid", "x"]), p("items", par_uuid("_uuid"))),
        ("batch_update_fields", d.batch_update_fields("A", &["x"]), p("items", par_uuid("_uuid"))),
        ("batch_update_returning", d.batch_update_returning("A", &["x"], &[("item._uuid", "uuid")]), p("items", par_uuid("_uuid"))),
        ("batch_select", d.batch_select("A", "uuid", "_uuid", &["x"]), p("items", par_uuid("uuid"))),
        ("revert_lifecycle_state", d.revert_lifecycle_state("A", "x"), p("items", par_uuid("uuid"))),
        ("batch_link", d.batch_link("R", &["p"]), p("items", liens("from_uuid", "to_uuid"))),
        ("batch_link_labeled", d.batch_link_labeled("R", Some(("A", "B")), &["p"]), p("items", liens("from_uuid", "to_uuid"))),
        ("batch_delete_relation", d.batch_delete_relation("R"), p("items", liens("from", "to"))),
        ("embed_check_hashes", d.embed_check_hashes("A", "mk"), p("items", par_uuid("uuid"))),
        (
            "embed_set",
            d.embed_set("A", "emb", "mk"),
            p(
                "items",
                lot(300, |k| {
                    ligne(&[
                        ("uuid", s(&format!("a{k}"))),
                        ("emb", CypherValue::List(vec![CypherValue::Float(0.5); 4])),
                        ("hash", s("h")),
                    ])
                }),
            ),
        ),
        ("embed_set_hash_returning_offset", d.embed_set_hash_returning_offset("A", "mk"), p("items", lot(300, |k| ligne(&[("uuid", s(&format!("a{k}"))), ("hash", s("h"))])))),
        ("embed_get_offset", d.embed_get_offset("A"), p("items", par_uuid("uuid"))),
        ("kb_gather_fields", d.kb_gather_fields("A", &["x"]), p("items", par_uuid("uuid"))),
        ("kb_gather_content (avant)", d.kb_gather_content("A", "R", "B", true, &["x"]), p("items", par_uuid("uuid"))),
        ("kb_gather_content (arrière)", d.kb_gather_content("B", "R", "A", false, &["x"]), p("items", lot(300, |k| ligne(&[("uuid", s(&format!("b{k}")))])))),
        ("kb_upsert_index", d.kb_upsert_index("I", &["x"], &["x"]), p("items", par_uuid("uuid"))),
        (
            "upsert_aside",
            d.upsert_aside(),
            p(
                "items",
                lot(300, |k| {
                    ligne(&[
                        ("key", s(&format!("k{k}"))),
                        ("entity", s("A")),
                        ("uuid", s(&format!("a{k}"))),
                        ("hash", s("h")),
                        ("row", s("{}")),
                        ("chunks", s("[]")),
                        ("session", s("s")),
                        ("at", CypherValue::Int(1)),
                    ])
                }),
            ),
        ),
        (
            "upsert_blobs (cypher_blob_store)",
            rag3weaver::cypher_blob_store::upsert_blobs_query(),
            p("items", lot(300, |k| ligne(&[("key", s(&format!("k{k}"))), ("data", CypherValue::Blob(vec![1, 2]))]))),
        ),
    ];

    let mut fautes = Vec::new();
    for (nom, requete, params) in &formes {
        let plan = match c.execute_with_params(&format!("EXPLAIN {requete}"), params) {
            Ok(r) => r.rows.iter().map(|row| format!("{row:?}")).collect::<Vec<_>>().join("\n"),
            Err(e) => {
                fautes.push(format!("{nom} : EXPLAIN refusé — {e}\n  {requete}"));
                continue;
            }
        };
        if plan.contains("CROSS_PRODUCT") {
            fautes.push(format!("{nom} : produit cartésien\n  {requete}"));
        }
        if let Err(e) = c.execute_with_params(requete, params) {
            fautes.push(format!("{nom} : exécution refusée — {e}\n  {requete}"));
        }
    }
    assert!(fautes.is_empty(), "{} forme(s) en faute :\n{}", fautes.len(), fautes.join("\n"));
}
