//! FTS via lucivy v3 `ShardedHandle`, en Rust direct.
//!
//! Remplace les `CALL *_LUCIVY_INDEX` de l'extension C++ : l'index vit sur un
//! [`BlobStore`], les blobs font foi et le cache mmap local est jetable — même
//! modèle ACID que `SparseHandle`, donc portable Postgres gratuitement.
//!
//! Voir `docs/23-aout-2026-20h33/04-migration-fts-lucivy-v3-rust.md` pour la
//! passation depuis la session lucivy.

use std::io;
use std::sync::Arc;

use lucivy_core::blob_store::BlobStore;

/// Adaptateur `Arc<dyn BlobStore>` → `impl BlobStore`.
///
/// `BlobShardStorage<S>` exige `S: BlobStore + Sized`, alors que le Catalog
/// détient un `Arc<dyn BlobStore>` (le backend est choisi au runtime : Cypher
/// ou Postgres). Ce newtype fait le pont, sans copie : il ne clone que l'Arc.
pub struct DynBlobStore(pub Arc<dyn BlobStore>);

impl BlobStore for DynBlobStore {
    fn load(&self, index_name: &str, file_name: &str) -> io::Result<Vec<u8>> {
        self.0.load(index_name, file_name)
    }
    fn save(&self, index_name: &str, file_name: &str, data: &[u8]) -> io::Result<()> {
        self.0.save(index_name, file_name, data)
    }
    fn delete(&self, index_name: &str, file_name: &str) -> io::Result<()> {
        self.0.delete(index_name, file_name)
    }
    fn exists(&self, index_name: &str, file_name: &str) -> io::Result<bool> {
        self.0.exists(index_name, file_name)
    }
    fn list(&self, index_name: &str) -> io::Result<Vec<String>> {
        self.0.list(index_name)
    }
}

/// Préfixe des clés de blob d'un index FTS, pour ne pas collisionner avec les
/// index sparse (qui utilisent `Sparse_{table}`).
pub fn fts_index_name(table: &str) -> String {
    format!("Lucivy_{table}")
}

/// Construit le `SchemaConfig` v3 d'une table à partir de ses champs texte.
///
/// Le JSON est celui qu'attendent aussi les bindings Python/Node — il se
/// désérialise directement en `SchemaConfig`. `sfx_version: 3` est le défaut
/// pour tout nouvel index, mais on l'écrit explicitement : c'est le comportement
/// qu'on veut figer, pas celui qu'on subit.
///
/// Les `filter_fields` du DDL deviennent des champs non-texte du schéma ; ils
/// sont ensuite adressés via `QueryConfig.filters` à la requête.
pub fn build_schema_config(
    text_fields: &[String],
    filter_fields: &[(String, String)],
    shards: usize,
) -> Result<lucivy_core::query::SchemaConfig, String> {
    let mut fields: Vec<serde_json::Value> = text_fields
        .iter()
        .map(|name| {
            serde_json::json!({ "name": name, "type": "text", "stored": true })
        })
        .collect();

    for (name, ty) in filter_fields {
        // Types du DDL rag3db → types de schéma lucivy.
        let lucivy_type = match ty.to_ascii_uppercase().as_str() {
            "INT64" | "INT32" | "INT" | "SERIAL" => "i64",
            "UINT64" | "UINT32" => "u64",
            "DOUBLE" | "FLOAT" => "f64",
            "BOOL" | "BOOLEAN" => "bool",
            _ => "string",
        };
        fields.push(serde_json::json!({ "name": name, "type": lucivy_type, "stored": true }));
    }

    serde_json::from_value(serde_json::json!({
        "fields": fields,
        "sfx_version": 3,
        "shards": shards.max(1),
    }))
    .map_err(|e| format!("SchemaConfig invalide: {e}"))
}

/// Extrait l'offset rag3db (`_node_id`) d'un document rendu par la recherche.
///
/// C'est le pendant de [`build_document`] : ce qu'on a écrit à l'indexation,
/// on le relit ici pour résoudre un hit en entité.
pub fn node_id_of(
    handle: &lucivy_core::sharded_handle::ShardedHandle,
    doc: &ld_lucivy::LucivyDocument,
) -> Option<u64> {
    use ld_lucivy::schema::document::Value;
    let nid_field = handle.field(lucivy_core::handle::NODE_ID_FIELD)?;
    doc.field_values()
        .find(|(f, _)| *f == nid_field)
        .and_then(|(_, v)| v.as_value().as_u64())
}

/// Topologie de stockage d'un index FTS.
///
/// Ce n'est pas un détail d'implémentation, c'est une décision d'architecture :
/// les deux modes ont des propriétés opposées à l'ouverture.
#[derive(Debug, Clone)]
pub enum FtsStorage {
    /// **(a)** Le BlobStore fait foi, le cache mmap local est jetable.
    ///
    /// Simple et cohérent avec `SparseHandle` : rien à sauvegarder à côté de la
    /// base, et l'index survit à la perte du disque local.
    ///
    /// Coût : `BlobDirectory::new` efface son cache (`{pid}/{seq}`) et
    /// **rematérialise l'intégralité de l'index à chaque ouverture** ; le `Drop`
    /// le supprime. Acceptable pour un serveur long-vécu (une fois au premier
    /// usage), rédhibitoire pour un navigateur qui rouvre à chaque chargement.
    BlobBacked,

    /// **(b)** Copie locale durable, tenue à jour par deltas LUCIDS.
    ///
    /// L'index est un répertoire persistant mmapé directement ; les mises à jour
    /// arrivent par `snapshot` puis `apply_sharded_delta` depuis un `SyncServer`.
    /// Jamais de re-téléchargement complet.
    ///
    /// C'est la topologie qu'appelle le WASM offline (le build navigateur a déjà
    /// sa persistance IDBFS), et celle qui rend LUCIDS utile — en (a) un delta
    /// n'a aucun cache persistant à mettre à jour.
    LocalFs { base_path: String },
}

impl Default for FtsStorage {
    /// (a) par défaut : c'est ce que prescrit la passation lucivy, et le seul
    /// mode validé de bout en bout à ce jour (`test_acid_blob_v3.rs`).
    fn default() -> Self {
        FtsStorage::BlobBacked
    }
}

/// Construit un document lucivy pour une entité, prêt à être indexé.
///
/// `offset` est l'offset interne rag3db (celui que porte [`crate::node_id_cache`]) :
/// c'est la clé qui permettra de résoudre les résultats de recherche en entités.
///
/// **Piège payé par la session lucivy** : le champ `_node_id` doit être écrit
/// *dans le document*. Le second argument d'`add_document` ne nourrit que le
/// routeur de shards — sans le champ, les résultats ne se résolvent pas.
///
/// Les champs absents du schéma sont ignorés silencieusement : le schéma est
/// figé à la création de l'index, alors que les entités peuvent gagner des
/// champs par la suite.
pub fn build_document(
    handle: &lucivy_core::sharded_handle::ShardedHandle,
    fields: &[(String, String)],
    offset: u64,
) -> Result<ld_lucivy::LucivyDocument, String> {
    let nid_field = handle
        .field(lucivy_core::handle::NODE_ID_FIELD)
        .ok_or_else(|| format!("champ {} absent du schéma", lucivy_core::handle::NODE_ID_FIELD))?;

    let mut doc = ld_lucivy::LucivyDocument::new();
    doc.add_u64(nid_field, offset);

    for (name, value) in fields {
        if let Some(f) = handle.field(name) {
            doc.add_text(f, value);
        }
    }
    Ok(doc)
}

/// Nombre de shards par défaut d'un nouvel index.
pub const DEFAULT_SHARDS: usize = 4;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_name_is_prefixed_to_avoid_sparse_collision() {
        assert_eq!(fts_index_name("Product"), "Lucivy_Product");
        assert_ne!(fts_index_name("Product"), "Sparse_Product");
    }

    #[test]
    fn schema_config_has_text_fields_and_v3() {
        let cfg = build_schema_config(
            &["_title".to_string(), "_content".to_string()],
            &[],
            2,
        )
        .expect("config valide");
        let json = serde_json::to_value(&cfg).expect("sérialisable");
        assert_eq!(json["sfx_version"], 3, "v3 explicite, pas subi");
        assert_eq!(json["shards"], 2);
        let names: Vec<&str> = json["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"_title") && names.contains(&"_content"));
    }

    #[test]
    fn filter_fields_are_mapped_to_lucivy_types() {
        let cfg = build_schema_config(
            &["body".to_string()],
            &[("year".to_string(), "INT64".to_string()),
              ("category".to_string(), "STRING".to_string())],
            1,
        )
        .expect("config valide");
        let json = serde_json::to_value(&cfg).expect("sérialisable");
        let fields = json["fields"].as_array().unwrap();
        let ty_of = |n: &str| -> String {
            fields
                .iter()
                .find(|f| f["name"] == n)
                .and_then(|f| f["type"].as_str())
                .unwrap_or("?")
                .to_string()
        };
        assert_eq!(ty_of("body"), "text");
        assert_eq!(ty_of("year"), "i64");
        assert_eq!(ty_of("category"), "string");
    }

    /// Chaîne complète sur un index en mémoire : créer → indexer → chercher →
    /// résoudre en offsets. Aucune dépendance externe, aucun service.
    ///
    /// C'est le test qui prouve que le câblage v3 est correct — notamment que
    /// `_node_id` est bien écrit dans le document, le piège signalé par la
    /// passation.
    #[test]
    fn index_search_and_resolve_offsets_end_to_end() {
        use lucivy_core::blob_store::MemBlobStore;
        use lucivy_core::sharded_handle::{BlobShardStorage, ShardedHandle};

        let tmp = std::env::temp_dir().join(format!(
            "rag3weaver_fts_test_{}",
            std::process::id()
        ));
        let store = Arc::new(MemBlobStore::new());
        let cfg = build_schema_config(&["content".to_string()], &[], 2).unwrap();

        let storage = BlobShardStorage::new(store.clone(), fts_index_name("Doc"), &tmp);
        let handle = ShardedHandle::create_with_storage(Box::new(storage), &cfg)
            .expect("création de l'index");

        // Des offsets non contigus et non nuls : si le code confondait l'index
        // de boucle avec l'offset, le test le verrait.
        let corpus = [
            (41_u64, "le noyau alloue via kmalloc puis relâche le spinlock"),
            (77_u64, "la compilation incrémentale garde un cache de requêtes"),
            (1337_u64, "kmalloc est appelé dans le chemin d'allocation"),
        ];
        for (offset, text) in &corpus {
            let doc = build_document(&handle, &[("content".to_string(), text.to_string())], *offset)
                .expect("document");
            handle.add_document(doc, *offset).expect("indexation");
        }
        handle.commit().expect("commit");

        let q: lucivy_core::query::QueryConfig = serde_json::from_value(serde_json::json!({
            "type": "contains", "field": "content", "value": "kmalloc"
        }))
        .expect("QueryConfig");

        let hits = handle.search_with_docs(&q, 10).expect("recherche");
        let mut found: Vec<u64> = hits
            .iter()
            .filter_map(|h| node_id_of(&handle, &h.doc))
            .collect();
        found.sort_unstable();

        assert_eq!(
            found,
            vec![41, 1337],
            "les deux documents contenant kmalloc, résolus par leurs vrais offsets"
        );

        handle.close().ok();
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn shards_never_zero() {
        let cfg = build_schema_config(&["b".to_string()], &[], 0).unwrap();
        let json = serde_json::to_value(&cfg).unwrap();
        assert_eq!(json["shards"], 1, "0 shard n'a pas de sens");
    }
}
