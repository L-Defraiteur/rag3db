//! E2E : **réindexer un texte ne double pas les arêtes de ses morceaux vers lui** (14 novembre 2026,
//! ticket `docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-14-des-morceaux-de-scope-portent-plusieurs-aretes-vers-le-meme-scope.md`).
//!
//! Dans la base de dépôt de Lucie, 1 291 morceaux de Scope portaient 2 à 7 arêtes
//! `Scope_CHUNKED_FROM` vers le même Scope, une de plus par réindexation, en vagues identiques :
//! un défaut déterministe, pas une course. Le test le plus simple, sans concurrence ni point de
//! reprise forcé : un texte découpé en plus de 200 morceaux (les liens partent par COPY), réingéré
//! inchangé, puis modifié sur une ligne (il se redécoupe, ses morceaux gardent leur uuid par rang),
//! trois fois chacun ; après chaque passe, aucun morceau ne doit porter plus d'une arête. Puis la
//! même chose sous 200 morceaux (les liens partent par MERGE).
//!
//! Run with: ./run_e2e.sh --test e2e_liens_de_morceaux_reverses
#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use rag3weaver::config::{ChunkStrategy, ChunkingConfig, FieldType};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

fn texte() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("cle".to_string(), SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() });
    fields.insert("corps".to_string(), SimpleFieldDef { field_type: FieldType::String, is_content: true, ..Default::default() });
    // Une ligne par morceau : le nombre de morceaux est le nombre de lignes.
    let chunking = ChunkingConfig {
        strategy: ChunkStrategy::Lines,
        max_lines: 1,
        overlap_lines: 0,
        max_size: 10_000,
        title_max_chars: 0,
        ..Default::default()
    };
    EntityConfig { fields, signals: SearchSignals::BM25, chunking, hashsafe: Some(vec!["cle".into()]), ..Default::default() }
}

fn corps(lignes: usize, variante: usize) -> String {
    (0..lignes)
        .map(|i| if i == variante { format!("ligne {i} changée {variante}") } else { format!("ligne {i} du texte") })
        .collect::<Vec<_>>()
        .join("\n")
}

fn ligne(c: String) -> BTreeMap<String, CypherValue> {
    BTreeMap::from([("cle".to_string(), CypherValue::String("t".into())), ("corps".to_string(), CypherValue::String(c))])
}

fn entier(catalog: &Catalog, requete: &str) -> i64 {
    catalog.execute_raw(requete).unwrap().rows[0][0].as_i64().unwrap_or(-1)
}

/// (morceaux, arêtes, morceaux à plus d'une arête vers le même texte)
fn compte(catalog: &Catalog) -> (i64, i64, i64) {
    (
        entier(catalog, "MATCH (c:Texte_Chunk) RETURN count(c)"),
        entier(catalog, "MATCH ()-[r:Texte_CHUNKED_FROM]->() RETURN count(r)"),
        entier(
            catalog,
            "MATCH (c:Texte_Chunk)-[r:Texte_CHUNKED_FROM]->(p:Texte) WITH c, p, count(r) AS n WHERE n > 1 RETURN count(*)",
        ),
    )
}

fn jouer(nom: &str, lignes: usize) {
    let d = tempfile::tempdir().unwrap();
    let conn = Rag3dbConnection::new(d.path().join("base.rag3db")).expect("base sur disque");
    let config = CatalogConfig { name: Some(nom.into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    catalog.register_entity("Texte", texte()).unwrap();

    let mut releve: Vec<String> = Vec::new();
    let mut rouge: Option<String> = None;
    let mut passer = |catalog: &mut Catalog, etape: &str, c: String| {
        let res = catalog.ingest_entities("Texte", vec![ligne(c)]).unwrap();
        assert_eq!(res.failed, 0, "{etape} : {:?}", res.warnings);
        let (m, a, doubles) = compte(catalog);
        releve.push(format!("{etape} : {m} morceaux, {a} arêtes, {doubles} morceaux doublés"));
        if (doubles != 0 || a != m) && rouge.is_none() {
            rouge = Some(format!("{etape} : {m} morceaux, {a} arêtes, {doubles} morceaux à plus d'une arête"));
        }
        m
    };
    let m = passer(&mut catalog, "première ingestion", corps(lignes, usize::MAX));
    assert_eq!(m, lignes as i64, "[check: une-ligne-par-morceau] {m} morceaux pour {lignes} lignes");
    for k in 1..=3 {
        passer(&mut catalog, &format!("réingestion inchangée {k}"), corps(lignes, usize::MAX));
    }
    for k in 1..=3 {
        passer(&mut catalog, &format!("réingestion modifiée {k}"), corps(lignes, k * 7));
    }
    for r in &releve {
        println!("[témoin {nom}] {r}");
    }
    if let Some(r) = rouge {
        panic!("[check: une-arete-par-morceau] {r}");
    }
}

#[test]
#[ignore]
fn reindexer_un_texte_de_plus_de_200_morceaux_ne_double_pas_ses_aretes() {
    jouer("morceaux-par-copy", 300);
}

#[test]
#[ignore]
fn reindexer_un_texte_de_moins_de_200_morceaux_ne_double_pas_ses_aretes() {
    jouer("morceaux-par-merge", 50);
}
