//! **Le test 5 du contrat du dialecte** : un dialecte qui déclare
//! `transactions` tient un paquet entier — défait, il ne laisse rien ;
//! validé, il laisse tout. Les écritures passent par le dialecte (`Write`)
//! et la transaction par la connexion (`begin`, `commit`, `rollback`).
//!
//! Page : `docs/8-octobre-2026-16h29/embarquements/01-le-contrat-du-dialecte.md`, §5.
//! Exige une base PostgreSQL vivante (`RAG3WEAVER_PG`, ou le conteneur
//! `pgvector/pgvector:pg17` sur le port 5433) ; joué vivant pour la
//! première fois le 11 octobre 2026, sur le conteneur de Lucie.
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

/// Les colonnes d'une table d'entité : `_uuid` d'abord, la clé primaire que
/// `create_table` pose (sans elle, PostgreSQL refuse la table : 42703).
fn colonnes() -> Vec<ColumnDef> {
    vec![
        ColumnDef { name: "_uuid".into(), col_type: ColumnType::Text },
        ColumnDef { name: "titre".into(), col_type: ColumnType::Text },
    ]
}

#[test]
#[ignore]
fn un_paquet_tient_entier_par_le_dialecte_et_la_connexion() {
    let d = PostgresDialect;
    assert!(d.capabilities().transactions, "le dialecte déclare les transactions");
    let conn = PostgresConnection::new(&conn_str()).unwrap_or_else(|e| panic!("PostgreSQL injoignable ({e}) : RAG3WEAVER_PG, ou le conteneur sur 5433"));
    let table = "contrat_tx";
    conn.execute(&d.drop_table(table)).unwrap();
    conn.execute(&d.create_table(table, &colonnes())).unwrap();
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

/// **Défaire un lien le retire** (ticket
/// 2026-10-11-postgresql-defaire-un-lien-ne-trouve-rien) : `Write::Unlink`
/// lit `from` et `to` dans `$items`, et l'arête posée par `Write::Link`
/// (`from_uuid`, `to_uuid`) part ; l'autre reste.
#[test]
#[ignore]
fn defaire_un_lien_le_retire() {
    let d = PostgresDialect;
    let conn = PostgresConnection::new(&conn_str()).unwrap_or_else(|e| panic!("PostgreSQL injoignable ({e}) : RAG3WEAVER_PG, ou le conteneur sur 5433"));
    let (table, relation) = ("contrat_lien_noeud", "contrat_lien");
    conn.execute(&d.drop_table(relation)).unwrap();
    conn.execute(&d.drop_table(table)).unwrap();
    conn.execute(&d.create_table(table, &colonnes())).unwrap();
    conn.execute(&d.create_rel_table(relation, table, table, &[])).unwrap();
    conn.execute_with_params(&d.write(&Write::Upsert { table: table.into(), columns: vec!["_uuid".into(), "titre".into()] }).unwrap(), &[lignes(&["a", "b", "c"])]).unwrap();

    let paire = |de: &str, vers: &str, cles: (&str, &str)| {
        CypherValue::Map(std::collections::BTreeMap::from([
            (cles.0.to_string(), CypherValue::String(de.into())),
            (cles.1.to_string(), CypherValue::String(vers.into())),
        ]))
    };
    let lier = d.write(&Write::Link { relation: relation.into(), ends: Some((table.into(), table.into())), props: vec![] }).unwrap();
    let items = CypherValue::List(vec![paire("a", "b", ("from_uuid", "to_uuid")), paire("a", "c", ("from_uuid", "to_uuid"))]);
    conn.execute_with_params(&lier, &[QueryParam::new("items", items)]).unwrap();
    let compter = d.count_rows(relation);
    assert_eq!(conn.execute(&compter).unwrap().rows[0][0], CypherValue::Int(2), "deux arêtes posées");

    let defaire = d.write(&Write::Unlink { relation: relation.into() }).unwrap();
    conn.execute_with_params(&defaire, &[QueryParam::new("items", CypherValue::List(vec![paire("a", "b", ("from", "to"))]))]).unwrap();
    let restes = conn.execute(&format!("SELECT from_uuid, to_uuid FROM {}", d.quote(relation))).unwrap();
    assert_eq!(restes.rows.len(), 1, "une arête défaite, l'autre reste : {:?}", restes.rows);
    assert_eq!(restes.rows[0][1], CypherValue::String("c".into()), "c'est a → c qui reste");

    conn.execute(&d.drop_table(relation)).unwrap();
    conn.execute(&d.drop_table(table)).unwrap();
}
