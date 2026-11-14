//! **Le coût d'une recherche sans filtre quand le drapeau `tout_visible` tombe** (2a′-2, la mesure du
//! plan accepté par c7 le 14 novembre 2026 ; page
//! `docs/everything-declarative/14-11-2026/03-le-pas-2a-prime-stocker-les-generations-sans-fuite.md`, § 7).
//!
//! Sur une **copie** d'une base de code à versions (`RAG3WEAVER_SONDE_BASE`) et son manifeste
//! (`RAG3WEAVER_SONDE_MANIFESTE`), jamais la base en service :
//! 1. la migration 2a′-1 (`code::migrer_vers_les_generations`), chronométrée : c'est le chemin réel
//!    d'une base qui monte ;
//! 2. trois états :
//!    - **A** : le drapeau tenu, comme toute base de 2a′-1 ;
//!    - **B** : le drapeau tombé (`marquer_des_lignes_invisibles`), toutes les lignes visibles ;
//!    - **C** : B, et 1 % des `Scope` retirés au rang lu ;
//! 3. dans chaque état, trois mesures séparées :
//!    - **(a)** les racines des occurrences : `select_all` en A (le repli d'`occurrence_roots_for`),
//!      la clause de visibilité puis `resolve_offsets` en B et C (`admitted_parent_uuids_for`) ;
//!    - **(b)** le masque des versions : `derived_references` par lots de 5 000 racines ;
//!    - **(c)** `search_code` de bout en bout, sur 20 requêtes fixes (10 noms, 10 phrases).
//!
//! 3 tours de chauffe, puis `RAG3WEAVER_SONDE_TOURS` tours (20 par défaut) : la médiane et le p95, en ms.
//! Une sonde mesure, elle n'échoue pas.

use rag3weaver::backend::PreparedBackend;
use rag3weaver::connection::{CypherValue, DbConnection, QueryParam};
use rag3weaver::search_backend::SearchBackend;
use rag3weaver::Rag3dbConnection;
use serde_json::json;
use std::time::Instant;

fn stats(mut t: Vec<f64>) -> (f64, f64) {
    t.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p95 = t[((t.len() as f64 * 0.95).ceil() as usize).saturating_sub(1).min(t.len() - 1)];
    (t[t.len() / 2], p95)
}

fn chrono<T>(tours: usize, mut f: impl FnMut() -> T) -> (f64, f64, T) {
    for _ in 0..3 {
        f();
    }
    let mut t = Vec::with_capacity(tours);
    let mut dernier = None;
    for _ in 0..tours {
        let d = Instant::now();
        dernier = Some(f());
        t.push(d.elapsed().as_secs_f64() * 1000.0);
    }
    let (m, p) = stats(t);
    (m, p, dernier.unwrap())
}

#[test]
#[ignore]
fn le_cout_d_une_recherche_sans_filtre() {
    let (Some(base), Some(manifeste)) = (std::env::var_os("RAG3WEAVER_SONDE_BASE"), std::env::var_os("RAG3WEAVER_SONDE_MANIFESTE")) else {
        eprintln!("SAUTÉ le_cout_d_une_recherche_sans_filtre : RAG3WEAVER_SONDE_BASE ou RAG3WEAVER_SONDE_MANIFESTE manque (une copie, jamais la base en service)");
        return;
    };
    let tours: usize = std::env::var("RAG3WEAVER_SONDE_TOURS").ok().and_then(|v| v.parse().ok()).unwrap_or(20);
    let prepared = PreparedBackend::load(std::path::Path::new(&manifeste)).expect("le manifeste");
    let t0 = Instant::now();
    let backend = prepared.open(Box::new(Rag3dbConnection::new(&base).expect("ouvrir la copie")), None).expect("ouvrir le backend");
    eprintln!("[sonde] ouverture : {:.0} ms", t0.elapsed().as_secs_f64() * 1000.0);

    // 1. La migration.
    let (conn, dialecte, cible) = {
        let mut cat = backend.catalog.lock().unwrap();
        let forme = rag3weaver::code::scope_form(&cat).unwrap();
        eprintln!("[sonde] forme à l'ouverture : {forme:?}");
        if forme == rag3weaver::code::ScopeForm::Versions {
            let t = Instant::now();
            rag3weaver::code::migrer_vers_les_generations(&mut cat).expect("la migration");
            eprintln!("[sonde] migration 2a′-1 : {:.0} ms", t.elapsed().as_secs_f64() * 1000.0);
        }
        assert_eq!(rag3weaver::code::scope_form(&cat).unwrap(), rag3weaver::code::ScopeForm::Generations, "la copie doit être à générations");
        (cat.conn_arc(), cat.dialect_arc(), cat.resolve_search_target("Scope").expect("la cible Scope"))
    };
    let projection = cible.occurrence_projection.clone().expect("une base à versions projette ses occurrences");
    let compter = |q: &str| match conn.execute(q).unwrap_or_else(|e| panic!("{q} : {e}")).rows.first().and_then(|l| l.first()) {
        Some(CypherValue::Int(n)) => *n,
        v => panic!("{q} : {v:?}"),
    };
    eprintln!(
        "[sonde] {} : {} lignes ; {} : {} ; parent {} ; relation {} ; {tours} tours",
        projection.entity,
        compter(&format!("MATCH (s:{}) RETURN count(*)", projection.entity)),
        cible.parent_table,
        compter(&format!("MATCH (s:{}) RETURN count(*)", cible.parent_table)),
        cible.parent_table,
        projection.relation
    );

    // Les 20 requêtes : les 10 noms les plus appelés, et 10 phrases.
    let r = conn
        .execute("MATCH (a:Scope)-[:CONSUMES]->(b:Scope) RETURN b.name AS n, count(*) AS k ORDER BY k DESC LIMIT 10")
        .unwrap();
    let mut requetes: Vec<String> = r.rows.iter().filter_map(|l| l.first().and_then(|v| v.as_str()).map(str::to_string)).collect();
    requetes.extend(
        [
            "ouvrir une base et enregistrer le schéma",
            "parse a json manifest",
            "rebuild the vector index",
            "lock order of the catalog",
            "read a file by its path",
            "tokenize text for full text search",
            "embedding service client",
            "transaction rollback",
            "write ahead log checkpoint",
            "translate a hop into a query",
        ]
        .map(String::from),
    );
    eprintln!("[sonde] requêtes : {requetes:?}");

    let moteur = rag3weaver::rag3db_search_backend::Rag3dbSearchBackend::new(conn.clone());
    let tout = dialecte.select_all(&projection.entity, &["_uuid"], None);
    let masque = dialecte.derived_references(&cible.parent_table, &projection.entity, &projection.relation);
    let clause = dialecte.condition_visible_sur("n", rag3weaver::code::COUCHE_DU_BACKEND, 0).expect("rag3db sait les générations");
    let visibles = dialecte.filter_resolve_offsets(&projection.entity, "n", &[], &clause, None);

    let etat = |nom: &str, drapeau: bool| {
        // (a) les racines.
        let (ma, pa, racines): (f64, f64, Vec<String>) = if drapeau {
            chrono(tours, || {
                conn.execute(&tout).unwrap().rows.into_iter().filter_map(|l| l.first().and_then(|v| v.as_str()).map(str::to_string)).collect()
            })
        } else {
            chrono(tours, || {
                let mut ids: Vec<u64> = conn
                    .execute(&visibles)
                    .unwrap()
                    .rows
                    .iter()
                    .filter_map(|l| l.first().and_then(|v| v.as_i64()).map(|i| i as u64))
                    .collect();
                ids.sort_unstable();
                ids.dedup();
                moteur.resolve_offsets(&projection.entity, &ids, &[]).unwrap().into_iter().map(|r| r.uuid).collect()
            })
        };
        eprintln!("[sonde] {nom} (a) racines   médiane {ma:>8.1} ms   p95 {pa:>8.1} ms   {} racines", racines.len());
        // (b) le masque des versions.
        let mut tri = racines.clone();
        tri.sort();
        let (mb, pb, versions) = chrono(tours, || {
            let mut n = 0usize;
            for lot in tri.chunks(5_000) {
                let p = QueryParam::new("uuids", CypherValue::List(lot.iter().cloned().map(CypherValue::String).collect()));
                n += conn.execute_with_params(&masque, &[p]).unwrap().rows.len();
            }
            n
        });
        eprintln!("[sonde] {nom} (b) masque    médiane {mb:>8.1} ms   p95 {pb:>8.1} ms   {versions} références");
        // (c) search_code de bout en bout.
        let mut medianes = Vec::new();
        let mut tous = Vec::new();
        for q in &requetes {
            let args = json!({"query": q, "options": {}});
            for _ in 0..3 {
                backend.call("search_code", args.clone()).unwrap_or_else(|e| panic!("search_code {q} : {e}"));
            }
            let mut t = Vec::with_capacity(tours);
            for _ in 0..tours {
                let d = Instant::now();
                backend.call("search_code", args.clone()).unwrap_or_else(|e| panic!("search_code {q} : {e}"));
                t.push(d.elapsed().as_secs_f64() * 1000.0);
            }
            tous.extend(t.iter().copied());
            medianes.push(stats(t).0);
        }
        let (mc, _) = stats(medianes);
        let (_, pc) = stats(tous);
        eprintln!("[sonde] {nom} (c) search_code   médiane des médianes {mc:>8.1} ms   p95 de tous {pc:>8.1} ms");
        (ma + mb, mc)
    };

    let a = etat("A", true);
    backend.catalog.lock().unwrap().marquer_des_lignes_invisibles();
    let b = etat("B", false);
    let retirees = compter(&format!(
        "MATCH (s:{}) WHERE offset(id(s)) % 100 = 0 SET s._jusqu_a = 0 RETURN count(*)",
        projection.entity
    ));
    eprintln!("[sonde] C : {retirees} lignes retirées au rang 0");
    let c = etat("C", false);
    eprintln!(
        "[sonde] RÉSUMÉ search_code médiane : A {:.1} ms, B {:.1} ms ({:+.1} ms, {:+.0} %), C {:.1} ms ({:+.1} ms, {:+.0} %) ; seuil : +20 % ou +50 ms",
        a.1,
        b.1,
        b.1 - a.1,
        (b.1 / a.1 - 1.0) * 100.0,
        c.1,
        c.1 - a.1,
        (c.1 / a.1 - 1.0) * 100.0
    );
}
