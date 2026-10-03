//! E2E — **reproduction** (3 octobre 2026) : après une table vidée puis
//! repeuplée par le chemin de masse, la recherche dense ne rend **rien** —
//! zéro résultat, `vector_count` à 0, aucun avertissement — alors que les
//! chunks ont leurs vecteurs en base (8 flottants chacun, étape 5). Ni un
//! `CHECKPOINT`, ni une lecture des chunks, ni une lecture de leurs vecteurs
//! ne la font répondre ici.
//!
//! Trouvé par la mise de côté avant purge (une synchronisation vide puis
//! repeuple), reproduit ici sans elle : une entité, deux lignes, deux
//! `delete`, la même ingestion. Dans le montage de la synchronisation
//! (régime `ParLot`, suppression par le drain d'une fin), une lecture des
//! chunks avant la recherche l'avait fait répondre une fois — non reproduit
//! dans ce montage-ci. Ce test est **rouge** tant que le défaut vit ; il
//! affiche ce qu'on observe à chaque étape avant d'échouer.
//!
//! Run with: ./run_e2e.sh --test e2e_repro_dense_apres_table_videe
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

/// Ce que rend la recherche dense : les uuids, le compte vectoriel, les
/// avertissements.
fn dense(c: &Arc<Mutex<Catalog>>, requete: &str) -> (Vec<String>, usize, Vec<String>) {
    let res = Catalog::rechercher(
        c,
        "Note",
        requete,
        SearchOptions { consistency: Consistency::Immediate, signals: Some(SearchSignals::VECTOR), ..Default::default() },
    )
    .expect("la recherche ne doit pas échouer");
    (res.results.into_iter().map(|r| r.uuid).collect(), res.meta.vector_count, res.meta.warnings)
}

#[test]
#[ignore]
fn la_recherche_dense_repond_apres_une_table_videe_puis_repeuplee() {
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

    let textes = ["Une première note.", "Une seconde note."];
    let lignes = || textes.iter().map(|t| ligne(t)).collect::<Vec<_>>();
    let uuids: Vec<String> = textes.iter().map(|t| catalog.entity_uuid("Note", &ligne(t)).unwrap()).collect();
    let c = Arc::new(Mutex::new(catalog));

    // 1. Première ingestion (table neuve, chemin de masse) : la recherche répond.
    c.lock().unwrap().ingest_entities("Note", lignes()).unwrap();
    let avant = dense(&c, textes[1]);
    eprintln!("REPRO 1 table neuve        : {avant:?}");

    // 2. Vider la table, puis la repeupler à l'identique : la table est vide,
    //    c'est de nouveau le chemin de masse.
    for u in &uuids {
        c.lock().unwrap().delete("Note", u).unwrap();
    }
    c.lock().unwrap().ingest_entities("Note", lignes()).unwrap();
    let apres = dense(&c, textes[1]);
    eprintln!("REPRO 2 vidée, repeuplée   : {apres:?}");

    // 3. Un CHECKPOINT la fait-il répondre ?
    c.lock().unwrap().execute_raw("CHECKPOINT").unwrap();
    let apres_checkpoint = dense(&c, textes[1]);
    eprintln!("REPRO 3 après CHECKPOINT   : {apres_checkpoint:?}");

    // 4. Une simple lecture des chunks.
    let chunks = c.lock().unwrap().execute_raw("MATCH (k:Note_Chunk) RETURN k._parent_uuid").unwrap().rows.len();
    let apres_match = dense(&c, textes[1]);
    eprintln!("REPRO 4 après MATCH ({chunks} chunks) : {apres_match:?}");

    // 5. Une lecture de la colonne de vecteurs elle-même.
    let vecteurs = c
        .lock()
        .unwrap()
        .execute_raw("MATCH (k:Note_Chunk) RETURN size(k.embedding__hachage)")
        .unwrap()
        .rows;
    let apres_vecteurs = dense(&c, textes[1]);
    eprintln!("REPRO 5 après lecture des vecteurs {vecteurs:?} : {apres_vecteurs:?}");

    assert!(!avant.0.is_empty(), "sur une table neuve, la recherche dense répond");
    assert_eq!(
        apres.0.first(),
        Some(&uuids[1]),
        "après une table vidée puis repeuplée, la recherche dense doit répondre comme avant"
    );
}
