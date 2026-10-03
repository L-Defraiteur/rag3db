//! E2E — **la recherche dense après des suppressions** (non-régression,
//! 3 octobre 2026).
//!
//! Trouvé par la mise de côté avant purge, qui vide puis repeuple : après une
//! table vidée puis repeuplée par le chemin de masse, la recherche dense ne
//! rendait **rien** — zéro résultat, aucun avertissement, les vecteurs
//! pourtant en base. La cause était dans **notre** greffe de suppression dans
//! l'index HNSW (l'amont ne supprime rien dans le HNSW), trois défauts
//! corrigés ensemble par la session cœur C++ :
//!
//! 1. le nettoyage de fin d'instruction ne tournait jamais (`finalize()`
//!    appelé sur l'opérateur d'origine, pas sur la copie qui s'exécute) ;
//! 2. le point d'entrée de l'index remplacé par un nœud mort — toute
//!    recherche partait de lui et ne rendait rien (le premier test) ;
//! 3. le graphe coupé en morceaux après des suppressions partielles — les
//!    plus proches rendus étaient faux, sans erreur (le second test).
//!
//! Base en mémoire, extension vector, une entité, un embarqueur déterministe.
//!
//! Run with: ./run_e2e.sh --test e2e_recherche_dense_apres_suppressions
#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use rag3weaver::config::FieldType;
use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::embedder::{EmbedError, Embedder, HashEmbedder};
use rag3weaver::search::{Consistency, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

/// Un embarqueur déterministe qui ne se déclare pas factice.
struct Hachage(HashEmbedder);

impl Embedder for Hachage {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        self.0.embed(texts)
    }
    fn dim(&self) -> usize {
        self.0.dim()
    }
    fn is_mock(&self) -> bool {
        false
    }
    fn name(&self) -> &str {
        "hachage"
    }
}

fn ligne(texte: &str) -> BTreeMap<String, CypherValue> {
    BTreeMap::from([("texte".to_string(), CypherValue::String(texte.into()))])
}

fn catalogue() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let ext = format!(
        "{}/extension/vector/build/libvector.rag3db_extension",
        std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
            let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
            std::path::Path::new(&manifest).parent().unwrap().parent().unwrap().to_string_lossy().to_string()
        })
    );
    conn.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { embedding_dim: 8, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(Hachage(HashEmbedder::new(8))), config);
    catalog.initialize().unwrap();
    let mut fields = HashMap::new();
    fields.insert(
        "texte".to_string(),
        SimpleFieldDef { field_type: FieldType::Text, is_title: true, is_content: true, ..Default::default() },
    );
    catalog
        .register_entity("Note", EntityConfig { fields, signals: SearchSignals::BM25 | SearchSignals::VECTOR, ..Default::default() })
        .unwrap();
    catalog
}

/// La recherche dense : les uuids rendus, le compte vectoriel, les
/// avertissements.
fn dense(c: &Arc<Mutex<Catalog>>, requete: &str, limit: usize) -> (Vec<String>, usize, Vec<String>) {
    let res = Catalog::rechercher(
        c,
        "Note",
        requete,
        SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::VECTOR),
            limit,
            ..Default::default()
        },
    )
    .expect("la recherche ne doit pas échouer");
    (res.results.into_iter().map(|r| r.uuid).collect(), res.meta.vector_count, res.meta.warnings)
}

/// **Défaut 2** : une table vidée puis repeuplée par le chemin de masse
/// répond comme une table neuve.
#[test]
#[ignore]
fn la_recherche_dense_repond_apres_une_table_videe_puis_repeuplee() {
    let catalog = catalogue();
    let textes = ["Une première note.", "Une seconde note."];
    let lignes = || textes.iter().map(|t| ligne(t)).collect::<Vec<_>>();
    let uuids: Vec<String> = textes.iter().map(|t| catalog.entity_uuid("Note", &ligne(t)).unwrap()).collect();
    let c = Arc::new(Mutex::new(catalog));

    c.lock().unwrap().ingest_entities("Note", lignes()).unwrap();
    let neuve = dense(&c, textes[1], 10);
    assert_eq!(neuve.0.first(), Some(&uuids[1]), "table neuve : {neuve:?}");

    for u in &uuids {
        c.lock().unwrap().delete("Note", u).unwrap();
    }
    c.lock().unwrap().ingest_entities("Note", lignes()).unwrap();
    let repeuplee = dense(&c, textes[1], 10);
    assert_eq!(
        repeuplee.0.first(),
        Some(&uuids[1]),
        "après une table vidée puis repeuplée, la recherche dense répond comme avant : {repeuplee:?}"
    );
    assert_eq!(repeuplee.0.len(), 2, "{repeuplee:?}");
}

/// **Défaut 3** : cent notes, quatre-vingt-dix supprimées. Chaque survivante
/// est la plus proche d'elle-même — sa propre phrase en requête, distance
/// nulle — et une recherche qui demande plus que ce qui reste ne rend que
/// des survivantes, sans résultat fantôme.
#[test]
#[ignore]
fn apres_des_suppressions_partielles_les_plus_proches_sont_les_bons() {
    let mut catalog = catalogue();
    let textes: Vec<String> = (0..100).map(|i| format!("La note numéro {i}, distincte de toutes les autres.")).collect();
    let uuids: Vec<String> = textes.iter().map(|t| catalog.entity_uuid("Note", &ligne(t)).unwrap()).collect();
    catalog.ingest_entities("Note", textes.iter().map(|t| ligne(t)).collect()).unwrap();
    // On garde une note sur dix.
    for (i, u) in uuids.iter().enumerate() {
        if i % 10 != 0 {
            catalog.delete("Note", u).unwrap();
        }
    }
    let c = Arc::new(Mutex::new(catalog));
    let survivantes: Vec<usize> = (0..100).filter(|i| i % 10 == 0).collect();
    let mut faux = Vec::new();
    for &i in &survivantes {
        let (rendus, _, _) = dense(&c, &textes[i], 3);
        if rendus.first() != Some(&uuids[i]) {
            faux.push((i, rendus));
        }
    }
    assert!(faux.is_empty(), "des survivantes ne sont pas les plus proches d'elles-mêmes : {faux:?}");

    let (rendus, compte, avertissements) = dense(&c, &textes[0], 20);
    assert!(rendus.iter().all(|u| !u.is_empty()), "aucun résultat sans identité : {rendus:?}");
    let vivantes: std::collections::HashSet<&String> = survivantes.iter().map(|&i| &uuids[i]).collect();
    assert!(rendus.iter().all(|u| vivantes.contains(u)), "seules des survivantes : {rendus:?}");
    assert!(rendus.len() <= survivantes.len(), "{} rendus pour {} survivantes", rendus.len(), survivantes.len());
    eprintln!("k=20 sur 10 survivantes : {} rendus, vector_count {compte}, avertissements {avertissements:?}", rendus.len());
}
