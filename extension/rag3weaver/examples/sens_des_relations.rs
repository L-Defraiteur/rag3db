//! **Une base a-t-elle perdu des arêtes ?** Compte chaque table de relation
//! dans les deux sens (`rag3weaver::relation_directions`) et dit les couples
//! dont les comptes diffèrent. À lancer sur une **copie** d'une base, jamais
//! sur une base qu'un backend tient : un seul processus par base.
//!
//! Usage : sens_des_relations <base.rag3db> [<extension vector>]
//! Sort en 1 si un couple est dissymétrique.
use rag3weaver::relation_directions::count_both_directions;
use rag3weaver::{DbConnection, Rag3dbConnection};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("chemin de la base");
    let conn = Rag3dbConnection::new(&path).expect("ouverture");
    if let Some(ext) = args.next() {
        conn.execute(&format!("LOAD EXTENSION '{ext}'")).expect("chargement de l'extension");
    }
    let sens = count_both_directions(&conn).unwrap_or_else(|e| panic!("{e}"));
    let mut fautes = 0;
    for d in &sens {
        let marque = if d.is_symmetric() { "  " } else { "✗ " };
        if !d.is_symmetric() {
            fautes += 1;
        }
        println!("{marque}{:<20} {:>12} → {:<12} direct {:>9}  inverse {:>9}", d.table, d.from, d.to, d.forward, d.backward);
    }
    println!("{} couples, {fautes} dissymétriques", sens.len());
    if fautes > 0 {
        std::process::exit(1);
    }
}
