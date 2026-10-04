//! **La sonde des dix identifiants** (question de Lucie, 4 octobre 2026) :
//! en plein texte seul, l'attendu est dans le top 5 huit fois sur dix mais
//! premier une seule fois — défaut de classement, pas de rappel. Requête par
//! requête : le rang de l'attendu, ce qui passe devant (nom, genre, rôle de
//! test, taille), puis les MÊMES requêtes posées à lucivy directement dans
//! chacun de ses modes — exact (d=0), Levenshtein (d=1, l'actuel), Jaro-
//! Winkler (0,9), exact_match, phrase, parse, et le champ nom seul.
//!
//! Une sonde mesure, elle n'échoue pas. HashEmbedder : aucun embarquement
//! réel, aucun service. Run with: ./run_e2e.sh --test sonde_identifiants_lucivy

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::sync::{Arc, Mutex};

use rag3weaver::code::{default_scope_chunking, read_sources, register_code_schema, SCOPE};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::search::{Consistency, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

const IDENTIFIANTS: [&str; 10] = [
    "merge_port_values",
    "fuse_signals",
    "resolve_search_target",
    "register_search_services",
    "embarquer_la_requete",
    "base_de_fusion",
    "appliquer_la_consigne_pour",
    "search_bm25_chunked",
    "parse_mermaid_template",
    "rendre_le_retard",
];

/// Les champs du Scope dans l'index lucivy : `bm25_fields` =
/// `content_fields()` seulement (content, docstring) — catalog.rs:4261
/// n'ajoute le `title_field()` QUE pour une entité dérivée. Le nom du
/// scope n'est PAS indexé en plein texte : c'est la découverte de la
/// première passe de cette sonde (« unknown field: name » sur tous les
/// modes), et l'explication du classement — la définition ne porte son
/// nom que là où son texte le répète.
const CHAMPS: [&str; 2] = ["content", "docstring"];

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = env!("CARGO_MANIFEST_DIR");
        format!("{manifest}/../..")
    })
}

fn manifest() -> String {
    format!("{}/extension/rag3weaver", rag3db_root())
}

fn options_bm25() -> SearchOptions {
    SearchOptions {
        limit: 10,
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        ..Default::default()
    }
}

/// Pose une requête lucivy brute sur le handle FTS du Scope et rend la
/// PREMIÈRE LIGNE du content de chaque document, dans l'ordre des scores.
/// (`name` n'est pas dans l'index — c'est la découverte de cette sonde — mais
/// `content` est stocké et sa première ligne est la signature du scope.)
fn signatures_lucivy(
    handle: &lucivy_core::sharded_handle::ShardedHandle,
    query: &serde_json::Value,
    limit: usize,
) -> Result<Vec<String>, String> {
    use ld_lucivy::schema::document::Value;
    let config: lucivy_core::query::QueryConfig =
        serde_json::from_value(query.clone()).map_err(|e| format!("QueryConfig: {e}"))?;
    let results = handle.search(&config, limit, None)?;
    let content_field = handle.field("content").ok_or("champ content absent du schéma")?;
    let mut lignes = Vec::new();
    for r in &results {
        let Some(shard) = handle.shard(r.shard_id) else { continue };
        let searcher = shard.reader.searcher();
        let Ok(doc) = searcher.doc::<ld_lucivy::LucivyDocument>(r.doc_address) else {
            continue;
        };
        let ligne = doc
            .field_values()
            .find(|(f, _)| *f == content_field)
            .and_then(|(_, v)| v.as_value().as_str().map(|s| {
                s.lines().next().unwrap_or("").chars().take(90).collect::<String>()
            }));
        lignes.push(ligne.unwrap_or_default());
    }
    Ok(lignes)
}

/// Un OU de la même clause sur les quatre champs — la forme exacte de
/// `build_contains_clauses` du crate.
fn sur_les_champs(clause_par_champ: impl Fn(&str) -> serde_json::Value) -> serde_json::Value {
    let clauses: Vec<serde_json::Value> = CHAMPS.iter().map(|f| clause_par_champ(f)).collect();
    serde_json::json!({ "type": "boolean", "should": clauses })
}

/// Le rang du doc dont la première ligne (la signature) porte l'identifiant
/// suivi d'une parenthèse ou d'un chevron — la définition, pas une mention.
fn rang_de(signatures: &[String], attendu: &str) -> String {
    let def = |l: &String| -> bool {
        l.contains(&format!("{attendu}(")) || l.contains(&format!("{attendu}<"))
    };
    match signatures.iter().position(def) {
        Some(i) => format!("{}", i + 1),
        None => "—".into(),
    }
}

#[test]
fn sonde_identifiants() {
    // La base du banc, à l'identique, sans modèle : le BM25 ne dépend pas de
    // l'embarqueur, et le HashEmbedder évite service et GPU.
    let embedder: Arc<dyn rag3weaver::embedder::Embedder> = Arc::new(HashEmbedder::new(64));
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!(
        "{}/extension/vector/build/libvector.rag3db_extension",
        rag3db_root()
    );
    boxed
        .execute(&format!("LOAD EXTENSION '{ext}'"))
        .expect("charger l'extension vecteur");
    let config = CatalogConfig {
        name: Some("sonde-identifiants".into()),
        embedding_dim: embedder.dim(),
        ..Default::default()
    };
    let mut catalog = Catalog::new(boxed, Box::new(embedder), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    let reel = Arc::new(Mutex::new(catalog));

    let racine = manifest();
    let sources: Vec<(String, String)> = read_sources(&format!("{racine}/src"))
        .expect("lire src/")
        .into_iter()
        .map(|(rel, c)| (format!("src/{rel}"), c))
        .collect();
    let analysis = rag3weaver::code::analyze(&racine, sources);
    let rapport = reel.lock().unwrap().ingest_code(&analysis).expect("ingérer src/");
    eprintln!("[sonde] {} scopes, {} en échec", rapport.scopes, rapport.failed);

    // ── 1. Le chemin réel : Catalog::rechercher, BM25 seul ─────────────────
    // Ce qui passe devant l'attendu, requête par requête.
    eprintln!("\n### 1. Le chemin réel (BM25 seul, mode Auto→Contains d=1)\n");
    for ident in IDENTIFIANTS {
        let r = Catalog::rechercher(&reel, SCOPE, ident, options_bm25()).expect("bm25");
        let mut lignes = Vec::new();
        let mut rang = String::from("—");
        for (i, res) in r.results.iter().enumerate() {
            let data = res.data.as_ref();
            let champ = |k: &str| -> String {
                data.and_then(|d| d.get(k))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            };
            let name = champ("name");
            if name == ident && rang == "—" {
                rang = format!("{}", i + 1);
            }
            if i < 5 {
                let lignes_scope = match (
                    data.and_then(|d| d.get("start_line")).map(|v| format!("{v:?}")),
                    data.and_then(|d| d.get("end_line")).map(|v| format!("{v:?}")),
                ) {
                    (Some(a), Some(b)) => format!("{a}..{b}"),
                    _ => String::new(),
                };
                lignes.push(format!(
                    "    {}. {} [{} | test_role={} | {} | score {:.3}]",
                    i + 1,
                    name,
                    champ("scope_type"),
                    champ("test_role"),
                    lignes_scope,
                    res.score,
                ));
            }
        }
        eprintln!("  {ident} → rang {rang}");
        for l in lignes {
            eprintln!("{l}");
        }
    }

    // ── 2. Lucivy en direct, mode par mode ─────────────────────────────────
    let cat = reel.lock().unwrap();
    let handle = cat.fts_handle(SCOPE).expect("handle FTS du Scope");
    eprintln!("\n### 2. Lucivy en direct — rang de l'attendu par mode\n");
    eprintln!(
        "| identifiant | lev d=1 (actuel) | exact d=0 | jaro-winkler 0,9 | exact_match | strict sép. | phrase | parse | content exact_match |"
    );
    eprintln!("|---|---|---|---|---|---|---|---|---|");
    for ident in IDENTIFIANTS {
        let actuel = sur_les_champs(|f| {
            serde_json::json!({"type":"contains","field":f,"value":ident,"distance":1})
        });
        let exact = sur_les_champs(|f| {
            serde_json::json!({"type":"contains","field":f,"value":ident,"distance":0})
        });
        let jaro = sur_les_champs(|f| {
            serde_json::json!({"type":"contains","field":f,"value":ident,"distance":2,
                "fuzzy_metric":"jaro_winkler","min_similarity":0.9})
        });
        let entier = sur_les_champs(|f| {
            serde_json::json!({"type":"contains","field":f,"value":ident,"distance":0,
                "exact_match":true})
        });
        // Le « strict » de lucivy (note de Lucie) : séparateurs compris, au
        // bit près — le mode Symbol de rag3weaver l'emploie déjà sur demande.
        let strict = sur_les_champs(|f| {
            serde_json::json!({"type":"contains","field":f,"value":ident,"distance":0,
                "strict_separators":true})
        });
        let phrase = sur_les_champs(|f| {
            serde_json::json!({"type":"phrase","field":f,"value":ident})
        });
        let parse = serde_json::json!({"type":"parse","fields":CHAMPS,"value":ident});
        // Le champ name n'existe pas dans l'index (voir CHAMPS) : la colonne
        // mesure ce que vaudrait un contains exact sur content SEUL.
        let nom_seul =
            serde_json::json!({"type":"contains","field":"content","value":ident,"distance":0,
                "exact_match":true});

        let mut cellules = Vec::new();
        for q in [&actuel, &exact, &jaro, &entier, &strict, &phrase, &parse, &nom_seul] {
            match signatures_lucivy(&handle, q, 10) {
                Ok(noms) => cellules.push(rang_de(&noms, ident)),
                Err(e) => cellules.push(format!("ERR {}", e.chars().take(40).collect::<String>())),
            }
        }
        eprintln!("| {ident} | {} |", cellules.join(" | "));
    }

    // Ce que l'actuel met devant, vu de lucivy même (top 3 brut, sans la
    // résolution aux chunks) — dit si le désordre naît dans lucivy ou après.
    eprintln!("\n### 3. Le top 3 lucivy brut de l'actuel (lev d=1)\n");
    for ident in IDENTIFIANTS.iter().take(4) {
        let actuel = sur_les_champs(|f| {
            serde_json::json!({"type":"contains","field":f,"value":ident,"distance":1})
        });
        match signatures_lucivy(&handle, &actuel, 3) {
            Ok(noms) => eprintln!("  {ident} → {}", noms.join(" · ")),
            Err(e) => eprintln!("  {ident} → ERR {e}"),
        }
    }
}
