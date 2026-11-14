//! **Le coût de la clause de visibilité sans filtre** (2a′-2, mesure (a) du plan accepté par c7 le
//! 14 novembre 2026 ; page `docs/everything-declarative/14-11-2026/03-le-pas-2a-prime-stocker-les-generations-sans-fuite.md`, § 7).
//!
//! Sans filtre, une recherche admet toutes les lignes tant que `Visibilite::tout_visible` tient. Une
//! fois le drapeau défait, `resoudre_le_filtre` résout **tous les offsets visibles** de `Scope` à
//! chaque recherche. C'est cette requête qui se mesure ici, telle que le dialecte la rend
//! (`condition_visible_sur`, puis `filter_resolve_offsets`), sur une **copie** d'une base de code réelle
//! (`RAG3WEAVER_SONDE_BASE`, jamais la base en service). Les trois colonnes sont posées par `ALTER`
//! sur la copie si elles manquent.
//!
//! Les états :
//! - **plancher** : tous les offsets, sans clause (le coût d'une résolution de toute la table) ;
//! - **B** : la clause, toutes les lignes visibles ;
//! - **C** : la clause, 1 % des lignes retirées au rang lu (`_jusqu_a = 0`, sur la copie).
//! L'état A, le drapeau tenu, ne lance aucune requête : son coût est nul.
//!
//! 3 tours de chauffe, puis la médiane et le p95 de `RAG3WEAVER_SONDE_TOURS` tours (20 par défaut),
//! en ms. Une sonde mesure, elle n'échoue pas.

use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::dialect::{Rag3dbDialect, SchemaDialect};
use rag3weaver::Rag3dbConnection;
use std::time::Instant;

fn compter(c: &Rag3dbConnection, q: &str) -> i64 {
    let r = c.execute(q).unwrap_or_else(|e| panic!("{q} : {e}"));
    match r.rows.first().and_then(|l| l.first()) {
        Some(CypherValue::Int(n)) => *n,
        v => panic!("{q} : {v:?}"),
    }
}

fn mesurer(c: &Rag3dbConnection, nom: &str, q: &str, tours: usize) {
    for _ in 0..3 {
        c.execute(q).unwrap_or_else(|e| panic!("{q} : {e}"));
    }
    let mut t = Vec::with_capacity(tours);
    let mut lignes = 0;
    for _ in 0..tours {
        let d = Instant::now();
        let r = c.execute(q).unwrap_or_else(|e| panic!("{q} : {e}"));
        t.push(d.elapsed().as_secs_f64() * 1000.0);
        lignes = r.rows.len();
    }
    t.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p95 = t[((t.len() as f64 * 0.95).ceil() as usize).saturating_sub(1).min(t.len() - 1)];
    eprintln!("[sonde] {nom:<10} médiane {:>8.1} ms   p95 {:>8.1} ms   {lignes} offsets", t[t.len() / 2], p95);
    eprintln!("[sonde]            {q}");
}

#[test]
#[ignore]
fn le_cout_de_la_clause_sans_filtre() {
    let Some(base) = std::env::var_os("RAG3WEAVER_SONDE_BASE") else {
        eprintln!("SAUTÉ le_cout_de_la_clause_sans_filtre : RAG3WEAVER_SONDE_BASE manque (une copie, jamais la base en service)");
        return;
    };
    let tours: usize = std::env::var("RAG3WEAVER_SONDE_TOURS").ok().and_then(|v| v.parse().ok()).unwrap_or(20);
    let c = Rag3dbConnection::new(&base).expect("ouvrir la copie");
    let d = Rag3dbDialect;
    eprintln!("[sonde] Scope {} lignes, {tours} tours", compter(&c, "MATCH (s:Scope) RETURN count(*)"));

    // Les colonnes, sur la copie seulement, si la base ne les a pas.
    if c.execute("MATCH (s:Scope) RETURN s._couche LIMIT 1").is_err() {
        let t0 = Instant::now();
        for (col, def) in [("_couche", "STRING DEFAULT 'backend'"), ("_depuis", "INT64 DEFAULT 0"), ("_jusqu_a", "INT64")] {
            c.execute(&format!("ALTER TABLE Scope ADD {col} {def}")).unwrap_or_else(|e| panic!("ALTER Scope {col} : {e}"));
        }
        eprintln!("[sonde] ALTER Scope : {:.0} ms", t0.elapsed().as_secs_f64() * 1000.0);
    }

    let plancher = d.filter_resolve_offsets("Scope", "n", &[], "n._uuid IS NOT NULL", None);
    let clause = d.condition_visible_sur("n", "backend", 0).expect("rag3db sait les générations");
    let visible = d.filter_resolve_offsets("Scope", "n", &[], &clause, None);

    mesurer(&c, "plancher", &plancher, tours);
    mesurer(&c, "B", &visible, tours);

    // C : 1 % des lignes retirées au rang lu.
    let retirees = compter(&c, "MATCH (s:Scope) WHERE offset(id(s)) % 100 = 0 SET s._jusqu_a = 0 RETURN count(*)");
    eprintln!("[sonde] C : {retirees} lignes retirées");
    mesurer(&c, "C", &visible, tours);
}
