//! Mesure : marquer chaque ligne vue d'une synchronisation, par lots, contre
//! tenir les identifiants vus à part et faire la différence à la fin.
//! Usage : mesure_marque_snapshot <dossier-temporaire> [lignes]
use rag3weaver::connection::{CypherValue, QueryParam};
use rag3weaver::{DbConnection, Rag3dbConnection};
use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("dossier");
    let n: usize = args.next().map(|s| s.parse().unwrap()).unwrap_or(27_000);
    let path = format!("{dir}/mesure.rag3db");
    let _ = std::fs::remove_file(&path);
    let conn = Rag3dbConnection::new(&path).expect("ouverture");
    conn.execute("CREATE NODE TABLE Ligne(_uuid STRING PRIMARY KEY, nom STRING, texte STRING, a INT64, b DOUBLE, c STRING[], _content_hash STRING, _snapshot STRING)").unwrap();
    let uuids: Vec<String> = (0..n).map(|i| format!("{i:032x}")).collect();
    let t = Instant::now();
    for lot in uuids.chunks(512) {
        let rows = CypherValue::List(lot.iter().map(|u| CypherValue::Map([
            ("u".to_string(), CypherValue::String(u.clone())),
            ("t".to_string(), CypherValue::String(format!("texte de la ligne {u} ").repeat(12))),
        ].into())).collect());
        conn.execute_with_params(
            "UNWIND $rows AS r CREATE (:Ligne {_uuid: r.u, nom: r.u, texte: r.t, a: 1, b: 2.0, c: ['x','y'], _content_hash: r.u, _snapshot: 's0'})",
            &[QueryParam::new("rows", rows)]).unwrap();
    }
    println!("création de {n} lignes : {:?}", t.elapsed());
    conn.execute("CHECKPOINT").ok();

    // A. Marquer chaque ligne vue, un SET par lot de 512.
    let t = Instant::now();
    for lot in uuids.chunks(512) {
        let l = CypherValue::List(lot.iter().map(|u| CypherValue::String(u.clone())).collect());
        conn.execute_with_params("UNWIND $uuids AS u MATCH (n:Ligne {_uuid: u}) SET n._snapshot = 's1'",
            &[QueryParam::new("uuids", l)]).unwrap();
    }
    let marquer = t.elapsed();
    let t = Instant::now();
    let r = conn.execute("MATCH (n:Ligne) WHERE n._snapshot <> 's1' RETURN count(*)").unwrap();
    println!("A — marquer {} lots : {marquer:?} ; fin (non marquées) : {:?} → {:?}", uuids.len().div_ceil(512), t.elapsed(), r.rows);
    conn.execute("CHECKPOINT").ok();
    let taille = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    println!("   taille de la base après marquage + checkpoint : {} Mo", taille / 1_000_000);

    // B. Identifiants vus tenus à part (en mémoire ici), différence à la fin.
    let t = Instant::now();
    let tous = CypherValue::List(uuids.iter().map(|u| CypherValue::String(u.clone())).collect());
    let r = conn.execute_with_params("MATCH (n:Ligne) WHERE NOT n._uuid IN $vus RETURN count(*)",
        &[QueryParam::new("vus", tous)]).unwrap();
    println!("B — différence à la fin sur {n} identifiants : {:?} → {:?}", t.elapsed(), r.rows);
}
