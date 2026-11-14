//! **Le coût du prédicat de visibilité de 2a′** (14 novembre 2026, mesure demandée par c7 avant le
//! code ; page `docs/everything-declarative/14-11-2026/03-le-pas-2a-prime-stocker-les-generations-sans-fuite.md`).
//!
//! Sur une **copie** d'une base de code réelle (`RAG3WEAVER_SONDE_BASE`, jamais la base en service),
//! un jeu fixe de requêtes de lecteur se joue trois fois :
//! - **sans** les colonnes, comme aujourd'hui ;
//! - **avec** `_couche`, `_depuis` et `_jusqu_a` sur `Scope`, `File` et `CONSUMES` (ajoutées par
//!   `ALTER` sur la copie), et le prédicat de visibilité ;
//! - par une **table d'appartenance** (`Generation` → `Scope`), l'alternative de la page.
//!
//! Médiane de `RAG3WEAVER_SONDE_TOURS` jeux (20 par défaut), en millisecondes. Une sonde mesure, elle
//! n'échoue pas. `./run_e2e.sh --test sonde_cout_du_predicat` sous `RAG3WEAVER_SONDE_BASE=<copie>`.

use rag3weaver::connection::DbConnection;
use rag3weaver::Rag3dbConnection;
use std::time::Instant;

fn scalaire(c: &Rag3dbConnection, q: &str) -> String {
    let r = c.execute(q).unwrap_or_else(|e| panic!("{q} : {e}"));
    r.rows.first().and_then(|l| l.first()).map(|v| format!("{v:?}")).unwrap_or_default()
}

fn mediane(c: &Rag3dbConnection, q: &str, tours: usize) -> (f64, usize) {
    let mut t = Vec::with_capacity(tours);
    let mut lignes = 0;
    for _ in 0..tours {
        let d = Instant::now();
        let r = c.execute(q).unwrap_or_else(|e| panic!("{q} : {e}"));
        t.push(d.elapsed().as_secs_f64() * 1000.0);
        lignes = r.rows.len();
    }
    t.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (t[t.len() / 2], lignes)
}

#[test]
#[ignore]
fn le_cout_du_predicat_de_visibilite() {
    let Some(base) = std::env::var_os("RAG3WEAVER_SONDE_BASE") else {
        eprintln!("SAUTÉ le_cout_du_predicat_de_visibilite : RAG3WEAVER_SONDE_BASE manque (une copie, jamais la base en service)");
        return;
    };
    let tours: usize = std::env::var("RAG3WEAVER_SONDE_TOURS").ok().and_then(|v| v.parse().ok()).unwrap_or(20);
    let c = Rag3dbConnection::new(&base).expect("ouvrir la copie");
    eprintln!("[sonde] Scope {} lignes, File {}, CONSUMES {}", scalaire(&c, "MATCH (s:Scope) RETURN count(*)"),
        scalaire(&c, "MATCH (f:File) RETURN count(*)"), scalaire(&c, "MATCH ()-[r:CONSUMES]->() RETURN count(*)"));
    // Les entrées du jeu : le fichier le plus peuplé, le nom le plus appelé, la source.
    let r = c.execute("MATCH (s:Scope) RETURN s.file_path AS f, count(*) AS n ORDER BY n DESC LIMIT 1").unwrap();
    let fichier = match r.rows.first().and_then(|l| l.first()) { Some(rag3weaver::connection::CypherValue::String(s)) => s.clone(), v => panic!("fichier : {v:?}") };
    let r = c.execute("MATCH (a:Scope)-[:CONSUMES]->(b:Scope) RETURN b.name AS n, count(*) AS k ORDER BY k DESC LIMIT 1").unwrap();
    let nom = match r.rows.first().and_then(|l| l.first()) { Some(rag3weaver::connection::CypherValue::String(s)) => s.clone(), v => panic!("nom : {v:?}") };
    let r = c.execute(&format!("MATCH (s:Scope) WHERE s.file_path = '{fichier}' RETURN s.source LIMIT 1")).unwrap();
    let source = match r.rows.first().and_then(|l| l.first()) { Some(rag3weaver::connection::CypherValue::String(s)) => s.clone(), v => panic!("source : {v:?} ({fichier})") };
    eprintln!("[sonde] fichier « {fichier} », nom « {nom} », source « {source} », {tours} tours");

    let p = |v: &str| format!("{v}._couche = 'ancre' AND {v}._depuis <= 0 AND ({v}._jusqu_a IS NULL OR {v}._jusqu_a > 0)");
    let jeu = |pred: bool| -> Vec<(&'static str, String)> {
        let et = |v: &str| if pred { format!(" AND {}", p(v)) } else { String::new() };
        let ou = |v: &str| if pred { format!(" WHERE {}", p(v)) } else { String::new() };
        vec![
            ("outline (un fichier)", format!("MATCH (s:Scope) WHERE s.file_path = '{fichier}'{} RETURN s.name, s.start_line ORDER BY s.start_line", et("s"))),
            ("usages (un saut)", format!("MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) WHERE b.name = '{nom}'{}{}{} RETURN a._uuid", et("a"), et("b"), et("r"))),
            ("impact (1 à 3 sauts)", if pred {
                format!("MATCH (a:Scope)-[:CONSUMES*1..3 (r, n | WHERE {} AND {})]->(b:Scope) WHERE b.name = '{nom}'{}{} RETURN count(DISTINCT a)", p("r"), p("n"), et("a"), et("b"))
            } else {
                format!("MATCH (a:Scope)-[:CONSUMES*1..3]->(b:Scope) WHERE b.name = '{nom}' RETURN count(DISTINCT a)")
            }),
            ("racines d'une source (masque)", format!("MATCH (s:Scope) WHERE s.source = '{source}'{} RETURN s._uuid", et("s"))),
            ("compte des scopes", format!("MATCH (s:Scope){} RETURN count(*)", ou("s"))),
            ("fichiers d'une source", format!("MATCH (f:File) WHERE f.source = '{source}'{} RETURN f._uuid", et("f"))),
        ]
    };
    let jouer = |etiquette: &str, requetes: Vec<(&'static str, String)>| -> Vec<f64> {
        requetes.into_iter().map(|(quoi, q)| {
            let (ms, n) = mediane(&c, &q, tours);
            eprintln!("[sonde] {etiquette:>12} | {quoi:<30} | {ms:>9.2} ms | {n} ligne(s)");
            ms
        }).collect()
    };
    let sans = jouer("sans", jeu(false));

    // Les colonnes, sur la copie seulement.
    for t in ["Scope", "File", "CONSUMES"] {
        let d = Instant::now();
        for (col, def) in [("_couche", "STRING DEFAULT 'ancre'"), ("_depuis", "INT64 DEFAULT 0"), ("_jusqu_a", "INT64")] {
            c.execute(&format!("ALTER TABLE {t} ADD {col} {def}")).unwrap_or_else(|e| panic!("ALTER {t} {col} : {e}"));
        }
        eprintln!("[sonde] ALTER {t} : {:.0} ms", d.elapsed().as_secs_f64() * 1000.0);
    }
    let avec = jouer("prédicat", jeu(true));
    // Le prédicat sur les nœuds seuls : une arête visible si ses deux bouts le sont.
    let pn = |v: &str| p(v);
    let _ = jouer("nœuds seuls", vec![
        ("usages (un saut)", format!("MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) WHERE b.name = '{nom}' AND {} AND {} RETURN a._uuid", pn("a"), pn("b"))),
        ("impact (1 à 3 sauts)", format!("MATCH (a:Scope)-[:CONSUMES*1..3 (r, n | WHERE {})]->(b:Scope) WHERE b.name = '{nom}' AND {} AND {} RETURN count(DISTINCT a)", pn("n"), pn("a"), pn("b"))),
        ("usages, arête seule", format!("MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) WHERE b.name = '{nom}' AND {} RETURN a._uuid", pn("r"))),
    ]);
    // Les formes de la clause complète (nœuds et arête), pour la traduction IR de 2a′-1.
    let formes = vec![
        ("A : une seule clause", format!("MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) WHERE b.name = '{nom}' AND {} AND {} AND {} RETURN a._uuid", pn("b"), pn("r"), pn("a"))),
        ("B : WITH entre le bout et le saut", format!("MATCH (b:Scope) WHERE b.name = '{nom}' AND {} WITH b MATCH (a:Scope)-[r:CONSUMES]->(b) WHERE {} AND {} RETURN a._uuid", pn("b"), pn("r"), pn("a"))),
        ("C : WITH après le saut", format!("MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) WHERE b.name = '{nom}' AND {} WITH a, r WHERE {} AND {} RETURN a._uuid", pn("b"), pn("r"), pn("a"))),
        ("D : l'arête dans le motif", format!("MATCH (a:Scope)-[r:CONSUMES {{_couche: 'ancre'}}]->(b:Scope) WHERE b.name = '{nom}' AND {} AND {} AND r._depuis <= 0 AND (r._jusqu_a IS NULL OR r._jusqu_a > 0) RETURN a._uuid", pn("b"), pn("a"))),
    ];
    let mut meilleure = (f64::MAX, String::new());
    for (quoi, q) in &formes {
        let (ms, n) = mediane(&c, q, tours);
        eprintln!("[sonde] {:>12} | {quoi:<34} | {ms:>9.2} ms | {n} ligne(s)", "forme");
        if ms < meilleure.0 { meilleure = (ms, q.clone()); }
    }
    for (etiquette, q) in [("A", &formes[0].1), ("meilleure", &meilleure.1)] {
        match c.execute(&format!("EXPLAIN {q}")) {
            Ok(r) => for l in r.rows { eprintln!("[sonde] EXPLAIN {etiquette} | {}", l.iter().map(|v| format!("{v:?}")).collect::<Vec<_>>().join(" ")); },
            Err(e) => eprintln!("[sonde] EXPLAIN {etiquette} refusé : {e}"),
        }
    }

    // L'alternative : une table d'appartenance de la génération 0 vers chaque Scope.
    c.execute("CREATE NODE TABLE Generation_sonde(id STRING, PRIMARY KEY(id))").unwrap();
    c.execute("CREATE REL TABLE Generation_sonde_CONTIENT(FROM Generation_sonde TO Scope)").unwrap();
    c.execute("CREATE (:Generation_sonde {id: 'g0'})").unwrap();
    let d = Instant::now();
    c.execute("MATCH (g:Generation_sonde {id: 'g0'}), (s:Scope) CREATE (g)-[:Generation_sonde_CONTIENT]->(s)").unwrap();
    eprintln!("[sonde] appartenance de g0 : {:.0} ms", d.elapsed().as_secs_f64() * 1000.0);
    let m = "(g:Generation_sonde {id: 'g0'})-[:Generation_sonde_CONTIENT]->";
    let membre = jouer("appartenance", vec![
        ("outline (un fichier)", format!("MATCH {m}(s:Scope) WHERE s.file_path = '{fichier}' RETURN s.name, s.start_line ORDER BY s.start_line")),
        ("usages (un saut)", format!("MATCH {m}(a:Scope)-[r:CONSUMES]->(b:Scope)<-[:Generation_sonde_CONTIENT]-(g) WHERE b.name = '{nom}' RETURN a._uuid")),
        ("racines d'une source (masque)", format!("MATCH {m}(s:Scope) WHERE s.source = '{source}' RETURN s._uuid")),
        ("compte des scopes", format!("MATCH {m}(s:Scope) RETURN count(*)")),
    ]);
    eprintln!("[sonde] BILAN prédicat/sans : {}", sans.iter().zip(&avec).map(|(a, b)| format!("{:.2}", b / a.max(0.001))).collect::<Vec<_>>().join(" "));
    eprintln!("[sonde] BILAN appartenance/sans (outline, usages, racines, compte) : {}", [0, 1, 3, 4].iter().zip(&membre).map(|(i, b)| format!("{:.2}", b / sans[*i].max(0.001))).collect::<Vec<_>>().join(" "));
}
