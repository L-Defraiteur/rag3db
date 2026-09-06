//! **Combien coûte de poser 200 000 arêtes, et par quel chemin ?**
//!
//! Sur le cœur C++ de rag3db (6 septembre 2026), 225 000 rendez-vous
//! MENTIONS prenaient 98 s par `UNWIND … MATCH … MERGE`, CREATE ou MERGE —
//! 2 300 arêtes par seconde. Ce banc compare, sur des tables nues :
//! l'UNWIND par tranches, et `COPY … FROM` un CSV, le chemin de chargement en
//! masse du moteur.
#![cfg(feature = "rag3db-native")]
mod common;

use std::time::Instant;

use rag3weaver::connection::DbConnection;
use rag3weaver::Rag3dbConnection;

#[test]
#[ignore]
fn poser_deux_cent_mille_aretes() {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    conn.execute("CREATE NODE TABLE Scope(_uuid STRING, PRIMARY KEY(_uuid))").unwrap();
    conn.execute("CREATE NODE TABLE Symbol(_uuid STRING, PRIMARY KEY(_uuid))").unwrap();
    conn.execute("CREATE REL TABLE MENTIONS(FROM Scope TO Symbol, kind STRING)").unwrap();
    conn.execute("CREATE REL TABLE MENTIONS2(FROM Scope TO Symbol, kind STRING)").unwrap();

    let (n_scopes, n_symbols, n_edges) = (18_000usize, 16_000usize, 200_000usize);
    // Les nœuds, par UNWIND — c'est rapide.
    for (table, n) in [("Scope", n_scopes), ("Symbol", n_symbols)] {
        let t = Instant::now();
        for lot in (0..n).collect::<Vec<_>>().chunks(5_000) {
            let liste: Vec<String> = lot.iter().map(|i| format!("\"{table}-{i}\"")).collect();
            conn.execute(&format!("UNWIND [{}] AS u CREATE (:{table} {{_uuid: u}})", liste.join(","))).unwrap();
        }
        eprintln!("[liens] {n} nœuds {table} en {:?}", t.elapsed());
    }
    // Des arêtes avec des moyeux : un symbole sur vingt reçoit la moitié des arêtes.
    let arete = |i: usize| -> (String, String) {
        let s = i % n_scopes;
        let sym = if i % 2 == 0 { (i / 2) % (n_symbols / 20) } else { i % n_symbols };
        (format!("Scope-{s}"), format!("Symbol-{sym}"))
    };

    // 1. UNWIND … MERGE par tranches de 5 000.
    let t = Instant::now();
    for lot in (0..n_edges).collect::<Vec<_>>().chunks(5_000) {
        let items: Vec<String> = lot
            .iter()
            .map(|&i| {
                let (a, b) = arete(i);
                format!("{{from_uuid: \"{a}\", to_uuid: \"{b}\", kind: \"CONSUMES\"}}")
            })
            .collect();
        conn.execute(&format!(
            "UNWIND [{}] AS item MATCH (a:Scope {{_uuid: item.from_uuid}}), (b:Symbol {{_uuid: item.to_uuid}}) MERGE (a)-[r:MENTIONS]->(b) SET r.kind = item.kind",
            items.join(",")
        ))
        .unwrap();
    }
    let unwind = t.elapsed();
    eprintln!("[liens] {n_edges} arêtes par UNWIND/MERGE en {unwind:?} → {:.0} arêtes/s", n_edges as f64 / unwind.as_secs_f64());

    // 2. COPY … FROM un CSV.
    let csv = std::env::temp_dir().join(format!("rag3weaver-liens-{}.csv", std::process::id()));
    {
        use std::io::Write;
        let mut f = std::io::BufWriter::new(std::fs::File::create(&csv).unwrap());
        for i in 0..n_edges {
            let (a, b) = arete(i);
            writeln!(f, "{a},{b},CONSUMES").unwrap();
        }
    }
    let t = Instant::now();
    let r = conn.execute(&format!("COPY MENTIONS2 FROM '{}' (from='Scope', to='Symbol')", csv.display()));
    let copie = t.elapsed();
    match r {
        Ok(_) => eprintln!("[liens] {n_edges} arêtes par COPY en {copie:?} → {:.0} arêtes/s", n_edges as f64 / copie.as_secs_f64()),
        Err(e) => eprintln!("[liens] COPY refusé : {e}"),
    }
    let _ = std::fs::remove_file(&csv);
    let n1 = conn.execute("MATCH ()-[r:MENTIONS]->() RETURN count(r)").unwrap();
    let n2 = conn.execute("MATCH ()-[r:MENTIONS2]->() RETURN count(r)").unwrap();
    eprintln!("[liens] posées : MERGE {:?}, COPY {:?}", n1.rows[0][0], n2.rows[0][0]);
}
