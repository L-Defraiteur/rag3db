//! **Ce qu'un arrêt brutal peut laisser, et que la relance doit réparer**
//! (5 octobre 2026).
//!
//! Un groupe revenu à moitié d'une mort pendant sa validation (défaut du
//! moteur, corrigé par `37608cf4b`), ou tout autre arrêt, pouvait laisser
//! des morceaux sans leur arête `{E}_CHUNKED_FROM` : la complétude de
//! l'inchangé ne comptait que les morceaux, le parent était sauté à chaque
//! relance, et ces morceaux restaient introuvables par la recherche.
//!
//! ```bash
//! ./run_e2e.sh --test e2e_reprise_durcie
//! ```
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::path::Path;
use std::sync::{Arc, Mutex};

use rag3weaver::code::{default_scope_chunking, register_code_schema, SCOPE};
use rag3weaver::code_sync::{sync_source, SourceSyncOptions};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::connection::DbConnection;
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::search::{SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn racine_moteur() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap().display().to_string()
    })
}

fn corpus(n: usize) -> Vec<(String, String)> {
    (0..n)
        .map(|i| {
            let appel = if i > 0 { format!("f{}() + ", i - 1) } else { String::new() };
            (format!("src/f{i:03}.rs"), format!("pub fn f{i}() -> u32 {{ {appel}{i} }}\n"))
        })
        .collect()
}

fn catalogue(conn: Rag3dbConnection) -> Catalog {
    conn.execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", racine_moteur()))
        .expect("extension vecteur");
    let config = CatalogConfig { name: Some("reprise-durcie".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().expect("initialize");
    register_code_schema(&mut catalog, default_scope_chunking()).expect("schéma du code");
    catalog
}

fn compte(catalog: &Catalog, q: &str) -> i64 {
    catalog.execute_raw(q).unwrap().rows[0][0].as_i64().unwrap()
}

const SANS_ARETE: &str =
    "MATCH (c:Scope_Chunk) WHERE NOT EXISTS { MATCH (c)-[:Scope_CHUNKED_FROM]->(:Scope) } RETURN count(c)";

/// La fonction `f10` est-elle rendue par une recherche par vecteur sur son
/// propre texte ?
fn retrouvee(catalog: Catalog) -> (bool, Catalog) {
    let partage = Arc::new(Mutex::new(catalog));
    let r = Catalog::rechercher(&partage, SCOPE, "pub fn f10() -> u32 { f9() + 10 }", SearchOptions {
        signals: Some(SearchSignals::VECTOR),
        ..Default::default()
    })
    .expect("recherche");
    let trouve = r.results.iter().any(|x| format!("{x:?}").contains("f010.rs"));
    (trouve, Arc::try_unwrap(partage).ok().unwrap().into_inner().unwrap())
}

/// **Des morceaux sans leur arête rendent leur parent incomplet** : la
/// relance le réingère, l'arête revient, et la recherche le retrouve.
#[test]
#[ignore]
fn des_morceaux_sans_leur_arete_sont_refaits_a_la_relance() {
    let mut catalog = catalogue(Rag3dbConnection::in_memory().expect("base en mémoire"));
    let options = SourceSyncOptions { batch_files: 16, exige: Disponibilites::TOUT, ..Default::default() };
    let snapshot = Snapshot::new("depot", corpus(40));
    let r = sync_source(&mut catalog, &snapshot, &options, &mut |_| {}).expect("première passe");
    assert_eq!(r.failed, 0, "{r:?}");
    assert_eq!(compte(&catalog, SANS_ARETE), 0, "une passe saine relie chaque morceau");

    // L'état fabriqué : les morceaux de deux scopes, sans leur arête.
    catalog
        .execute_raw("MATCH (c:Scope_Chunk)-[r:Scope_CHUNKED_FROM]->(s:Scope) WHERE s.name IN ['f10', 'f11'] DELETE r")
        .unwrap();
    let orphelins = compte(&catalog, SANS_ARETE);
    assert!(orphelins >= 2, "les morceaux de f10 et f11 sont sans arête ({orphelins})");
    let (avant, c) = retrouvee(catalog);
    catalog = c;
    println!("▸ {orphelins} morceaux sans arête ; f10 retrouvée avant la relance : {avant}");

    let r = sync_source(&mut catalog, &snapshot, &options, &mut |_| {}).expect("relance");
    assert_eq!(r.failed, 0, "{r:?}");
    assert_eq!(compte(&catalog, SANS_ARETE), 0, "la relance a refait les morceaux sans arête");
    let (apres, _) = retrouvee(catalog);
    assert!(apres, "après la relance, la recherche retrouve f10 par son vecteur");
}
