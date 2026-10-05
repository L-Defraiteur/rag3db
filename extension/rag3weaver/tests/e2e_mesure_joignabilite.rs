//! **Chaque morceau se retrouve-t-il par son propre vecteur ?** (5 octobre
//! 2026). C'est une mesure, pas un témoin : elle ne tourne que si
//! `MESURE_JOIGNABILITE` dit comment bâtir l'index.
//!
//! Le banc a trouvé dans l'index HNSW une ligne **injoignable** dès sa
//! construction par des COPY successifs : l'élagage des voisins à
//! l'insertion ne garantit pas qu'une ligne garde une arête entrante. Il le
//! voit sur des vecteurs faits durs exprès, une ligne sur 128. Cette mesure
//! en donne la fréquence sur de vrais vecteurs, ceux du service
//! d'embarquement, pour les trois façons dont le produit bâtit l'index :
//!
//! - `tout` : les vecteurs posés avec les lignes (`exige: TOUT`, le défaut),
//!   des COPY successifs dans une table déjà indexée ;
//! - `fond` : un premier index sans vecteurs, puis la dette rattrapée en fond
//!   (`embarquer_le_retard`). Au-delà du seuil de dette, l'index est tombé
//!   puis rebâti sur la table pleine ;
//! - `masse` : `bulk_vector_index`, qui tombe l'index, synchronise, puis le
//!   rebâtit.
//!
//! Pour chaque morceau de chaque table vectorielle, on cherche par son propre
//! vecteur, k = 10. Il est **trouvé** s'il est parmi les dix, ou si un vecteur
//! identique au sien sort en tête (distance nulle : deux morceaux au texte
//! identique). On rend, par table : le nombre de morceaux, d'introuvables, et
//! dix exemples avec la distance du premier rendu.
//!
//! ```bash
//! RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981 \
//! RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70 \
//! MESURE_JOIGNABILITE=tout ./run_e2e.sh --test e2e_mesure_joignabilite
//! ```
//! `MESURE_JOIGNABILITE_RACINE` : le dossier à indexer (par défaut `src/` de
//! ce crate).
#![cfg(all(feature = "rag3db-native", feature = "code", feature = "daemon"))]

mod common;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rag3weaver::code::{default_scope_chunking, register_code_schema};
use rag3weaver::code_sync::{sync_source, SourceSyncOptions};
use rag3weaver::code_tools::WorkingTree;
use rag3weaver::connection::{CypherValue, DbConnection, QueryParam};
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::Embedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn racine_moteur() -> PathBuf {
    std::env::var("RAG3DB_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap())
}

/// Les tables de morceaux qui portent un vecteur.
fn tables_vectorielles(catalog: &Catalog) -> Vec<String> {
    let mut t: Vec<String> = catalog
        .entity_configs()
        .iter()
        .filter(|(_, c)| c.has_simple_pipeline() && c.chunked != Some(false) && c.signals.vector())
        .map(|(e, _)| format!("{e}_Chunk"))
        .collect();
    t.sort();
    t
}

/// Par table : (morceaux, introuvables, exemples).
fn mesurer(catalog: &Catalog) -> Vec<(String, usize, usize, Vec<String>)> {
    let mut rendu = Vec::new();
    for table in tables_vectorielles(catalog) {
        let Ok(st) = catalog.vector_storage(&table) else { continue };
        let lignes = catalog
            .execute_raw(&format!("MATCH (c:{table}) WHERE c.{col} IS NOT NULL RETURN c._uuid, c.{col}", col = st.column))
            .expect("lire les morceaux")
            .rows;
        let q = format!("CALL QUERY_VECTOR_INDEX('{table}', '{}', $v, 10) RETURN node._uuid, distance", st.index);
        let (mut introuvables, mut exemples) = (0usize, Vec::new());
        for l in &lignes {
            let uuid = l[0].as_str().unwrap_or("");
            let r = catalog.conn().execute_with_params(&q, &[QueryParam::new("v", l[1].clone())]).expect("chercher");
            let le_sien = r.rows.iter().any(|x| x.first().and_then(CypherValue::as_str) == Some(uuid));
            let premier = r.rows.first().and_then(|x| x.get(1)).and_then(CypherValue::as_f64);
            if !le_sien && !premier.is_some_and(|d| d.abs() < 1e-6) {
                introuvables += 1;
                if exemples.len() < 10 {
                    exemples.push(format!("{uuid} (premier à {premier:?}, {} rendus)", r.rows.len()));
                }
            }
        }
        rendu.push((table, lignes.len(), introuvables, exemples));
    }
    rendu
}

#[test]
#[ignore]
fn chaque_morceau_se_retrouve_par_son_propre_vecteur() {
    let Ok(mode) = std::env::var("MESURE_JOIGNABILITE") else {
        println!("MESURE_JOIGNABILITE absente (tout | fond | masse) : rien à mesurer");
        return;
    };
    let source = std::env::var("MESURE_JOIGNABILITE_RACINE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
    let dossier = PathBuf::from(std::env::var("HOME").unwrap())
        .join(".cache/rag3weaver-build/joignabilite")
        .join(format!("{mode}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dossier);
    std::fs::create_dir_all(&dossier).unwrap();

    let embedder: Arc<dyn Embedder> = common::burn::GRANITE_278M.clone();
    let conn = Rag3dbConnection::new(dossier.join("base.rag3db")).expect("base");
    conn.execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", racine_moteur().display()))
        .expect("extension vecteur");
    let config = CatalogConfig { name: Some("joignabilite".into()), embedding_dim: embedder.dim(), ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(embedder), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();

    let arbre = WorkingTree::new(&source);
    let options = |exige| SourceSyncOptions { batch_files: 64, exige, ..Default::default() };
    let t = std::time::Instant::now();
    match mode.as_str() {
        "tout" => {
            let r = sync_source(&mut catalog, &arbre, &options(Disponibilites::TOUT), &mut |_| {}).expect("synchroniser");
            assert_eq!(r.failed, 0, "{r:?}");
        }
        "fond" => {
            let r = sync_source(&mut catalog, &arbre, &options(Disponibilites::RECHERCHE_TEXTE), &mut |_| {}).expect("synchroniser");
            assert_eq!(r.failed, 0, "{r:?}");
            while catalog.embarquer_le_retard(Disponibilites::TOUT, 512, None).expect("rattraper") > 0 {}
        }
        "masse" => {
            let entites: Vec<String> = catalog.entity_configs().keys().cloned().collect();
            let entites: Vec<&str> = entites.iter().map(String::as_str).collect();
            let r = catalog
                .bulk_vector_index(&entites, |c| sync_source(c, &arbre, &options(Disponibilites::TOUT), &mut |_| {}))
                .expect("en masse")
                .expect("synchroniser");
            assert_eq!(r.failed, 0, "{r:?}");
        }
        autre => panic!("MESURE_JOIGNABILITE={autre} : tout, fond ou masse"),
    }
    let bati = t.elapsed();
    let t = std::time::Instant::now();
    let mesures = mesurer(&catalog);
    let vus: HashSet<&str> = mesures.iter().map(|m| m.0.as_str()).collect();
    assert!(!vus.is_empty(), "aucune table vectorielle mesurée");
    for (table, n, manques, exemples) in &mesures {
        println!("JOIGNABILITE mode={mode} table={table} morceaux={n} introuvables={manques} exemples={exemples:?}");
    }
    let total: usize = mesures.iter().map(|m| m.1).sum();
    let manques: usize = mesures.iter().map(|m| m.2).sum();
    println!(
        "JOIGNABILITE mode={mode} total morceaux={total} introuvables={manques} — bâti en {:.0} s, mesuré en {:.0} s ({})",
        bati.as_secs_f64(),
        t.elapsed().as_secs_f64(),
        source.display()
    );
    drop(catalog);
    let _ = std::fs::remove_dir_all(&dossier);
}
