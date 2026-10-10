//! **Une entité qui ne déclare pas de vecteurs le dit** (10 octobre 2026).
//!
//! Symbol est en plein texte seul (`signals: BM25`) : son état de l'index
//! disait « vecteurs never (0 %) », qui se lisait comme une panne ou une
//! dette jamais payée. Après un index, il dit `not_declared`, et les entités
//! qui déclarent des vecteurs (Scope) disent `ready`.
//!
//! ```bash
//! ./run_e2e.sh --test e2e_etat_non_declare
//! ```
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::path::Path;

use rag3weaver::catalog::Level;
use rag3weaver::code::{default_scope_chunking, register_code_schema, SCOPE, SYMBOL};
use rag3weaver::code_sync::{sync_source, SourceSyncOptions};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::connection::DbConnection;
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn racine_moteur() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap().display().to_string()
    })
}

#[test]
#[ignore]
fn apres_un_index_symbol_dit_ses_vecteurs_non_declares() {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    conn.execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", racine_moteur()))
        .expect("extension vecteur");
    let config = CatalogConfig { name: Some("etat-non-declare".into()), embedding_dim: 16, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(16)), config);
    catalog.initialize().expect("initialize");
    register_code_schema(&mut catalog, default_scope_chunking()).expect("schéma du code");
    let corpus: Vec<(String, String)> =
        (0..20).map(|i| (format!("src/f{i:02}.rs"), format!("pub fn f{i}() -> u32 {{ {i} }}\n"))).collect();
    let options = SourceSyncOptions { batch_files: 8, exige: Disponibilites::TOUT, ..Default::default() };
    let r = sync_source(&mut catalog, &Snapshot::new("depot", corpus), &options, &mut |_| {}).expect("synchroniser");
    assert_eq!(r.failed, 0, "{r:?}");
    catalog.refresh_index_states(&[SCOPE, SYMBOL], None, 1, false).expect("états");

    let symbole = catalog.index_state_for(SYMBOL).expect("état de Symbol");
    let scope = catalog.index_state_for(SCOPE).expect("état de Scope");
    let json = serde_json::to_value(symbole).unwrap();
    println!("▸ Symbol {json} ; Scope vecteurs {:?} {} %", scope.vectors, scope.vectors_percent);
    assert_eq!(symbole.text, Level::Ready, "les symboles sont cherchables par mots");
    assert_eq!(json["vectors"], "not_declared", "Symbol ne déclare pas de vecteurs : {json}");
    assert_eq!(scope.vectors, Level::Ready, "Scope déclare des vecteurs, et ils sont faits");
    let ligne = catalog.index_progress_for(SYMBOL).unwrap().line(None, 1);
    assert!(ligne.contains("sans vecteurs (non déclarés)"), "{ligne}");
}
