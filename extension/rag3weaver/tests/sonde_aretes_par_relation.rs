//! Sonde : **les arêtes par relation d'une copie de base** (18 octobre 2026).
//! Ticket : `docs/tickets/2026-10-18-un-copy-de-relations-vers-un-noeud-deja-relie-perd-ses-relations-persistantes.md`.
//!
//! Ouvre `SONDE_BASE` (une copie reflink, jamais une base vive) et rend, par
//! relation et par couple de tables, le compte dans les deux sens
//! (`count_both_directions`) : une perte au stockage se voit en dissymétrie,
//! et contre une réindexation à neuf du même commit en écart de compte.
//! Une ligne TSV par couple : `table  from  to  aller  retour` ; puis les nœuds par
//! table et deux invariants (`sans_lien`). `SONDE_LISTER=R1,R2` liste en plus chaque
//! arête de ces relations par les clés de ses bouts, pour un `comm` entre deux bases.
//! `SONDE_REQUETE=<cypher>` rend en plus les lignes d'une requête libre.
//!
//! Mesuré le 18 octobre 2026 (code-lourd, même arbre, même binaire) : la
//! réindexation au seuil de COPY de liens par défaut a 18 `HAS_PARENT` et 11
//! `INHERITS_FROM` de moins et 17 `CONSUMES` de plus que celle au seuil haut
//! (`RAG3WEAVER_LINK_COPY_THRESHOLD=1000000000`) ; la base vive était identique à la
//! première. Ce n'est pas une arête perdue par le moteur : la même paire reçoit
//! `CONSUMES` quand les liens passent par COPY (un ordre de résolution du produit,
//! lu par l'arbre principal). Un correctif se juge par l'égalité des deux réindexations.
//!
//! Run with: cargo test --features rag3db-native,code --test sonde_aretes_par_relation -- --ignored --nocapture
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use rag3weaver::connection::DbConnection;
use rag3weaver::relation_directions::count_both_directions;
use rag3weaver::Rag3dbConnection;

#[test]
#[ignore]
fn compter_les_aretes() {
    let chemin = std::env::var("SONDE_BASE").expect("SONDE_BASE");
    let conn = Rag3dbConnection::new(&chemin).unwrap();
    conn.execute(&format!(
        "LOAD EXTENSION '{}'",
        rag3weaver::backend::vector_extension_of_this_process().expect("l'extension vector de ce processus").display()
    ))
    .unwrap();
    println!("base\t{chemin}");
    for c in count_both_directions(&conn).unwrap() {
        println!("{}\t{}\t{}\t{}\t{}", c.table, c.from, c.to, c.forward, c.backward);
    }
    // Les nœuds par table, puis les invariants qu'une perte d'arête casse : un
    // morceau sans son `*_CHUNKED_FROM`, un scope sans son `DEFINED_IN`.
    let compte = |q: &str| -> String {
        match conn.execute(q) {
            Ok(r) => r.rows.first().and_then(|l| l.first()).map(|v| format!("{v:?}")).unwrap_or_default(),
            Err(e) => format!("erreur : {e}"),
        }
    };
    let tables = conn.execute("CALL show_tables() RETURN *").unwrap();
    let mut noeuds: Vec<String> = tables
        .rows
        .iter()
        .filter(|r| r.iter().any(|v| v.as_str() == Some("NODE")))
        .filter_map(|r| r.get(1).and_then(|v| v.as_str()).map(str::to_string))
        .collect();
    noeuds.sort();
    for n in &noeuds {
        println!("noeuds\t{n}\t{}", compte(&format!("MATCH (x:{n}) RETURN count(x)")));
        if let Some(parent) = n.strip_suffix("_Chunk") {
            if noeuds.iter().any(|m| m == parent) {
                println!(
                    "sans_lien\t{n}\t{parent}_CHUNKED_FROM\t{}",
                    compte(&format!("MATCH (c:{n}) WHERE NOT EXISTS {{ MATCH (c)-[:{parent}_CHUNKED_FROM]->() }} RETURN count(c)"))
                );
            }
        }
    }
    // `SONDE_LISTER=R1,R2` : chaque arête de ces relations, par les clés de ses deux bouts.
    for rel in std::env::var("SONDE_LISTER").unwrap_or_default().split(',').filter(|r| !r.is_empty()) {
        if let Ok(r) = conn.execute(&format!("MATCH (a)-[:{rel}]->(b) RETURN a.key, b.key")) {
            for l in &r.rows {
                println!("arete\t{rel}\t{:?}\t{:?}", l.first(), l.get(1));
            }
        }
    }
    // `SONDE_REQUETE` : une requête libre, ses lignes telles quelles (la marque d'une arête…).
    if let Ok(q) = std::env::var("SONDE_REQUETE") {
        match conn.execute(&q) {
            Ok(r) => r.rows.iter().for_each(|l| println!("ligne\t{l:?}")),
            Err(e) => println!("requete\terreur\t{e}"),
        }
    }
    if noeuds.iter().any(|m| m == "Scope") && noeuds.iter().any(|m| m == "File") {
        println!(
            "sans_lien\tScope\tDEFINED_IN\t{}",
            compte("MATCH (s:Scope) WHERE NOT EXISTS { MATCH (s)-[:DEFINED_IN]->() } RETURN count(s)")
        );
        // Leur genre et leur fichier : une perte touche des genres qui ont un fichier.
        if let Ok(r) = conn.execute(
            "MATCH (s:Scope) WHERE NOT EXISTS { MATCH (s)-[:DEFINED_IN]->() } \
             RETURN s.scope_type, count(s) AS n ORDER BY n DESC LIMIT 12",
        ) {
            for l in &r.rows {
                println!("sans_defined_in_par_genre\t{:?}\t{:?}", l.first(), l.get(1));
            }
        }
        if let Ok(r) = conn.execute(
            "MATCH (s:Scope) WHERE NOT EXISTS { MATCH (s)-[:DEFINED_IN]->() } RETURN s.key, s.file_path, s.start_line LIMIT 8",
        ) {
            for l in &r.rows {
                println!("sans_defined_in_exemple\t{:?}", l);
            }
        }
    }
}
