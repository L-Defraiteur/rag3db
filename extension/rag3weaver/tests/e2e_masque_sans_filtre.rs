//! **Sans filtre ni vue, la recherche à versions ne masque rien** (défaut du lot 1a, 14 novembre 2026) :
//! sur la copie migrée de `code-rag3db-depot`, une recherche sans filtre prenait 7,5 s en bm25 et 25 s
//! avec le dense, contre 1 s avant les versions. Le masque admettait toutes les occurrences, puis passait
//! `_uuid IN [toutes les versions]` à chaque signal. Le travail d'une recherche se borne ici par les
//! lignes que lit chaque requête, pas par le temps.
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use rag3weaver::code::{analyze_source, default_scope_chunking, register_code_schema};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::connection::{DbConnection, DbError, QueryParam, QueryResult};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::search::{SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

/// La connexion de la base, qui retient le plus grand nombre de lignes rendu par une requête.
struct Comptee {
    inner: Rag3dbConnection,
    plus_de_lignes: Arc<AtomicUsize>,
}

impl DbConnection for Comptee {
    fn execute(&self, cypher: &str) -> Result<QueryResult, DbError> {
        let r = self.inner.execute(cypher)?;
        self.plus_de_lignes.fetch_max(r.rows.len(), Ordering::Relaxed);
        Ok(r)
    }
    fn execute_with_params(&self, cypher: &str, params: &[QueryParam]) -> Result<QueryResult, DbError> {
        let r = self.inner.execute_with_params(cypher, params)?;
        self.plus_de_lignes.fetch_max(r.rows.len(), Ordering::Relaxed);
        Ok(r)
    }
}

const N: usize = 200;

/// Une base de production à versions : `N` fonctions distinctes, dans `N` fichiers.
fn base() -> (Arc<Mutex<Catalog>>, Arc<AtomicUsize>) {
    let inner = Rag3dbConnection::in_memory().expect("base privée en mémoire");
    inner.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).unwrap();
    let plus_de_lignes = Arc::new(AtomicUsize::new(0));
    let mut cat = Catalog::new(Box::new(Comptee { inner, plus_de_lignes: plus_de_lignes.clone() }), Box::new(HashEmbedder::new(8)), CatalogConfig {
        allow_mock_embedder: true, embedding_dim: 8, ..Default::default()
    });
    cat.initialize().unwrap();
    register_code_schema(&mut cat, default_scope_chunking()).unwrap();
    let fichiers: Vec<(String, String)> = (0..N)
        .map(|i| (format!("f{i}.rs"), format!("pub fn fonction_{i}() {{\n    let mot_commun_{i} = {i};\n    let _ = mot_commun_{i} + 1;\n}}\n")))
        .collect();
    assert_eq!(cat.ingest_code(&analyze_source(&Snapshot::new("ancre", fichiers)).unwrap()).unwrap().failed, 0);
    (Arc::new(Mutex::new(cat)), plus_de_lignes)
}

/// **Une recherche sans filtre ne lit pas toutes les occurrences** : à k = 3, aucune requête de la
/// recherche ne rend autant de lignes qu'il y a d'occurrences, en plein texte comme en dense. Les
/// résultats restent des occurrences, jusqu'à k.
#[test]
#[ignore]
fn une_recherche_sans_filtre_ne_lit_pas_toutes_les_occurrences() {
    let (cat, plus_de_lignes) = base();
    let occurrences = cat.lock().unwrap().execute_raw("MATCH (s:Scope) RETURN count(s)").unwrap().rows[0][0].as_i64().unwrap() as usize;
    assert!(occurrences >= N, "le décor : {occurrences} occurrences");
    for signaux in [SearchSignals::BM25, SearchSignals::VECTOR] {
        plus_de_lignes.store(0, Ordering::Relaxed);
        let options = SearchOptions { signals: Some(signaux), strict_signals: true, limit: 3, ..Default::default() };
        let reponse = Catalog::rechercher(&cat, "Scope", "fonction_7 mot_commun_7", options).unwrap();
        assert!(!reponse.results.is_empty(), "{signaux:?} : aucun résultat");
        assert!(reponse.results.len() <= 3, "{signaux:?} : {} résultats pour k = 3", reponse.results.len());
        assert!(reponse.results.iter().all(|r| r.entity.as_deref() == Some("Scope")), "{signaux:?} : un résultat n'est pas une occurrence : {:?}", reponse.results);
        let lu = plus_de_lignes.load(Ordering::Relaxed);
        assert!(lu < occurrences / 2, "{signaux:?} : une requête de la recherche a lu {lu} lignes pour {occurrences} occurrences et k = 3");
    }
}

/// **Une version orpheline ne prend pas la place d'une occurrence** : un scope édité laisse sa version
/// d'avant sans occurrence (la rétention des orphelines est un pas à part). Une recherche qui la vise la
/// première rend quand même k occurrences, aucune version orpheline.
#[test]
#[ignore]
fn une_version_orpheline_ne_prend_pas_la_place_d_une_occurrence() {
    let (cat, _) = base();
    {
        let mut c = cat.lock().unwrap();
        let edite = Snapshot::new("ancre", [("f7.rs".to_string(), "pub fn fonction_7() {\n    let autre_chose = 0;\n}\n".to_string())]);
        assert_eq!(c.ingest_code(&analyze_source(&edite).unwrap()).unwrap().failed, 0);
        let orphelines = c.execute_raw("MATCH (v:ScopeContent) WHERE NOT EXISTS { MATCH (v)-[:ScopeContent_DERIVED_FROM]->(:Scope) } RETURN count(v)").unwrap().rows[0][0].as_i64().unwrap();
        assert!(orphelines >= 1, "le décor : l'édition devait laisser une version orpheline");
    }
    for signaux in [SearchSignals::BM25, SearchSignals::VECTOR] {
        let options = SearchOptions { signals: Some(signaux), strict_signals: true, limit: 3, ..Default::default() };
        let reponse = Catalog::rechercher(&cat, "Scope", "mot_commun_7", options).unwrap();
        assert_eq!(reponse.results.len(), 3, "{signaux:?} : la version orpheline a pris une place : {:?}", reponse.results);
    }
}
