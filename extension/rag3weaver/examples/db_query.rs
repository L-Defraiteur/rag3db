//! Exécuter des requêtes Cypher sur une base rag3db et imprimer les lignes en
//! JSON — pour inspecter une **copie** d'une base (jamais une base qu'un
//! backend tient : un seul processus par base).
//! Usage : db_query <base.rag3db> "<cypher>" ["<cypher>" …]
use rag3weaver::{DbConnection, Rag3dbConnection};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("chemin de la base");
    let conn = Rag3dbConnection::new(&path).expect("ouverture");
    for cypher in args {
        println!("## {cypher}");
        match conn.execute(&cypher) {
            Ok(result) => {
                for row in result.rows {
                    println!("{}", serde_json::to_string(&row).unwrap_or_default());
                }
            }
            Err(e) => println!("ERREUR : {e}"),
        }
    }
}
