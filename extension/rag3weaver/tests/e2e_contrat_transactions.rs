//! **Le test 5 du contrat du dialecte** : un dialecte qui déclare
//! `transactions` tient un paquet entier — défait, il ne laisse rien ;
//! validé, il laisse tout. Les écritures passent par le dialecte (`Write`)
//! et la transaction par la connexion (`begin`, `commit`, `rollback`).
//!
//! Page : `docs/8-octobre-2026-16h29/embarquements/01-le-contrat-du-dialecte.md`, §5.
//! Exige une base PostgreSQL vivante (`RAG3WEAVER_PG`, ou le conteneur
//! `pgvector/pgvector:pg17` sur le port 5433) : **non prouvé vivant** tant
//! qu'aucune ne tourne sur les postes.
//!
//! Run with: cargo test --features postgres --test e2e_contrat_transactions -- --ignored

#![cfg(feature = "postgres")]

use rag3weaver::connection::{CypherValue, DbConnection, QueryParam};
use rag3weaver::dialect::{ColumnDef, ColumnType, PostgresDialect, SchemaDialect};
use rag3weaver::postgres_connection::PostgresConnection;
use rag3weaver_ir::{Count, Write};

fn conn_str() -> String {
    std::env::var("RAG3WEAVER_PG").unwrap_or_else(|_| {
        "host=localhost port=5433 user=rag3weaver password=rag3weaver dbname=rag3weaver_test".to_string()
    })
}

fn lignes(uuids: &[&str]) -> QueryParam {
    QueryParam::new(
        "items",
        CypherValue::List(
            uuids
                .iter()
                .map(|u| {
                    CypherValue::Map(std::collections::BTreeMap::from([
                        ("_uuid".to_string(), CypherValue::String(u.to_string())),
                        ("titre".to_string(), CypherValue::String(format!("titre {u}"))),
                    ]))
                })
                .collect(),
        ),
    )
}

#[test]
#[ignore]
fn un_paquet_tient_entier_par_le_dialecte_et_la_connexion() {
    let d = PostgresDialect;
    assert!(d.capabilities().transactions, "le dialecte déclare les transactions");
    let conn = PostgresConnection::new(&conn_str()).unwrap_or_else(|e| panic!("PostgreSQL injoignable ({e}) : RAG3WEAVER_PG, ou le conteneur sur 5433"));
    let table = "contrat_tx";
    conn.execute(&d.drop_table(table)).unwrap();
    conn.execute(&d.create_table(table, &[ColumnDef { name: "titre".into(), col_type: ColumnType::Text }])).unwrap();
    let ecrire = d.write(&Write::Upsert { table: table.into(), columns: vec!["_uuid".into(), "titre".into()] }).unwrap();
    let compter = d.count(&Count::Rows { table: table.into() }).unwrap_or_else(|_| d.count_rows(table));
    let compte = |conn: &PostgresConnection| conn.execute(&compter).unwrap().rows[0][0].clone();

    // Un paquet défait ne laisse rien.
    conn.begin().unwrap();
    conn.execute_with_params(&ecrire, &[lignes(&["a", "b"])]).unwrap();
    conn.rollback().unwrap();
    assert_eq!(compte(&conn), CypherValue::Int(0), "un paquet défait ne laisse rien");

    // Un paquet validé laisse tout.
    conn.begin().unwrap();
    conn.execute_with_params(&ecrire, &[lignes(&["c", "d", "e"])]).unwrap();
    conn.commit().unwrap();
    assert_eq!(compte(&conn), CypherValue::Int(3), "un paquet validé laisse tout");

    conn.execute(&d.drop_table(table)).unwrap();
}
