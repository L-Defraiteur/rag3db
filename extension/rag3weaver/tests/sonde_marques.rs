//! **Sonde : comment les arêtes d'usage ont été résolues**, et, pour celles
//! posées par le seul nom, quelle forme a la référence — de quoi choisir le
//! palier qui rapporte le plus pour lire le type d'un receveur (ticket « une
//! arête ne dit pas comment elle a été résolue »).
//!
//! Corpus : `src/` et `tests/` de rag3weaver, indexés en mémoire (aucune
//! base de service touchée). La forme se lit sur la ligne du site : ce qui
//! précède le nom visé (`::`, `.`, rien) et ce qui précède le point.
//!
//! Run with: cargo test --features rag3db-native,code --test sonde_marques -- --ignored --nocapture

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::collections::{BTreeMap, HashMap};

use rag3weaver::code::{analyze, default_scope_chunking, read_sources, register_code_schema};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

fn setup() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("sonde-marques".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    catalog
}

fn texte(v: Option<&rag3weaver::connection::CypherValue>) -> String {
    v.and_then(|v| v.as_str()).unwrap_or("").to_string()
}

/// Des noms de méthodes de la bibliothèque standard : une arête « nom » d'un
/// receveur vers un homonyme du projet est presque sûrement fausse.
const STD: &[&str] = &[
    "clone", "collect", "lock", "find", "map", "get", "get_mut", "iter", "iter_mut", "into_iter", "len", "is_empty", "push", "pop", "insert",
    "remove", "contains", "contains_key", "unwrap", "expect", "as_str", "as_ref", "as_mut", "to_string", "to_owned", "into", "from", "new",
    "read", "write", "send", "recv", "try_recv", "as_bool", "as_i64", "as_u64", "as_f64", "as_array", "as_object", "join", "split", "trim",
    "starts_with", "ends_with", "extend", "drain", "clear", "sort", "dedup", "retain", "filter", "next", "take", "skip", "chain", "any", "all",
    "count", "sum", "min", "max", "entry", "or_default", "or_insert", "keys", "values", "first", "last", "replace", "parse", "format", "flush",
    "run", "execute", "close", "open", "start", "stop", "wait", "reset", "load", "store", "set", "update", "merge", "apply",
];

/// La forme d'une référence à `nom` sur la ligne `ligne`.
fn forme(ligne: &str, nom: &str) -> &'static str {
    let mot = |i: usize| {
        let avant = ligne[..i].chars().next_back();
        let apres = ligne[i + nom.len()..].chars().next();
        !avant.is_some_and(|c| c.is_alphanumeric() || c == '_') && !apres.is_some_and(|c| c.is_alphanumeric() || c == '_')
    };
    let Some(i) = ligne.match_indices(nom).map(|(i, _)| i).find(|i| mot(*i)) else {
        return "nom absent de la ligne";
    };
    let avant = ligne[..i].trim_end();
    let apres = &ligne[i + nom.len()..];
    if apres.starts_with('!') {
        return "nom de macro";
    }
    if ligne.starts_with("#[") || ligne.starts_with("#![") {
        return "attribut (#[cfg(…)])";
    }
    if let Some(tete) = avant.strip_suffix("::") {
        let seg = tete.rsplit(|c: char| !(c.is_alphanumeric() || c == '_')).next().unwrap_or("");
        let appel = apres.trim_start().starts_with('(');
        return match (seg, seg.starts_with(char::is_uppercase), appel) {
            ("Self", _, _) => "chemin par Self (Self::f)",
            (_, true, _) => "chemin par un type (T::f)",
            (_, false, true) => "appel par chemin de module (m::f())",
            (_, false, false) => "nom par chemin de module (m::T)",
        };
    }
    if let Some(recv) = avant.strip_suffix('.') {
        let recv = recv.trim_end();
        if recv.ends_with(')') || recv.ends_with('?') || recv.ends_with(']') {
            return "receveur : chaîne d'appels (x.a().f())";
        }
        let expr: String = recv.chars().rev().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.').collect::<Vec<_>>().into_iter().rev().collect();
        return if expr == "self" {
            "receveur : self (self.f())"
        } else if expr.starts_with("self.") {
            "receveur : champ (self.x.f())"
        } else if expr.contains('.') {
            "receveur : champ d'une variable (v.x.f())"
        } else {
            "receveur : variable (v.f())"
        };
    }
    if apres.trim_start().starts_with('(') {
        return "appel nu (f())";
    }
    "nom seul (type, valeur)"
}

#[test]
#[ignore]
fn arêtes_par_marque_et_formes_des_noms() {
    let mut cat = setup();
    let racine = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut sources = Vec::new();
    for dir in ["src", "tests"] {
        let d = format!("{racine}/{dir}");
        sources.extend(read_sources(&d).unwrap().into_iter().filter(|(p, _)| p.ends_with(".rs")).map(|(p, c)| {
            let chemin = if p.starts_with('/') { p } else { format!("{dir}/{p}") };
            (chemin, c)
        }));
    }
    let contenus: HashMap<String, String> = sources.iter().map(|(p, c)| (format!("{racine}/{p}"), c.clone())).collect();
    cat.ingest_code(&analyze(&racine, sources)).unwrap();

    let r = cat.execute_raw("MATCH (:Scope)-[r:CONSUMES]->(:Scope) RETURN r.resolution, count(*)").unwrap();
    let mut total = 0;
    eprintln!("\n| marque | arêtes CONSUMES |\n|---|---|");
    for x in &r.rows {
        let n = x.get(1).and_then(|v| v.as_i64()).unwrap_or(0);
        total += n;
        eprintln!("| {} | {n} |", texte(x.first()));
    }
    eprintln!("| total | {total} |");

    let r = cat
        .execute_raw("MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) WHERE r.resolution = 'nom' RETURN a.file_path, r.line, b.name, a.name, b.file_path")
        .unwrap();
    let mut formes: BTreeMap<&'static str, (usize, Vec<String>)> = BTreeMap::new();
    let mut dans_macro = 0;
    let (mut receveurs, mut receveurs_std) = (0, 0);
    for x in &r.rows {
        let (fichier, nom, de) = (texte(x.first()), texte(x.get(2)), texte(x.get(3)));
        let ligne = x.get(1).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let texte_ligne = contenus.get(&fichier).and_then(|c| c.lines().nth(ligne.saturating_sub(1))).unwrap_or("").trim().to_string();
        let f = forme(&texte_ligne, &nom);
        if f.starts_with("receveur") && STD.contains(&nom.as_str()) {
            receveurs_std += 1;
        }
        if f.starts_with("receveur") {
            receveurs += 1;
        }
        if texte_ligne.contains("!(") {
            dans_macro += 1;
        }
        let e = formes.entry(f).or_default();
        e.0 += 1;
        if e.1.len() < 3 {
            let court = fichier.rsplit('/').next().unwrap_or("");
            let cible = texte(x.get(4));
            let cible = cible.rsplit('/').next().unwrap_or("");
            e.1.push(format!("`{de}` → `{nom}` ({cible}) — {court}:{ligne} `{}`", texte_ligne.chars().take(90).collect::<String>()));
        }
    }
    let n = r.rows.len();
    eprintln!("\n| forme de la référence (arêtes « nom ») | arêtes | part |\n|---|---|---|");
    let mut tri: Vec<_> = formes.iter().collect();
    tri.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
    for (f, (c, _)) in &tri {
        eprintln!("| {f} | {c} | {:.0} % |", 100.0 * *c as f64 / n.max(1) as f64);
    }
    eprintln!("\n{dans_macro} des {n} sur une ligne qui contient une macro.");
    eprintln!("{receveurs_std} des {receveurs} « receveur » visent un nom de méthode de la bibliothèque standard (clone, collect, lock…).\n\nExemples :");
    for (f, (_, ex)) in &tri {
        eprintln!("\n**{f}**");
        for e in ex {
            eprintln!("- {e}");
        }
    }
}
