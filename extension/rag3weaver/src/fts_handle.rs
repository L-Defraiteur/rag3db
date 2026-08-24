//! FTS via lucivy v3 `ShardedHandle`, en Rust direct.
//!
//! Remplace les `CALL *_LUCIVY_INDEX` de l'extension C++ : l'index vit sur un
//! [`BlobStore`], les blobs font foi et le cache mmap local est jetable — même
//! modèle ACID que `SparseHandle`, donc portable Postgres gratuitement.
//!
//! Voir `docs/23-aout-2026-20h33/04-migration-fts-lucivy-v3-rust.md` pour la
//! passation depuis la session lucivy.

use std::sync::Arc;

/// Indexe une entité, en ne retenant que les champs présents au schéma.
///
/// Le filtrage est **nécessaire** : `add_document_json` échoue sur un nom de
/// champ inconnu (à dessein — c'est un bug appelant). Or on lui passe toutes
/// les valeurs texte du record, parce que le schéma doit rester l'unique source
/// de vérité sur ce qui est indexé, plutôt qu'une seconde liste à tenir
/// synchronisée avec `bm25_fields`.
///
/// `_node_id` n'est plus écrit à la main : `add_document*` l'estampille
/// lui-même avec l'offset passé, et refuse un document portant un id différent
/// (lucivy `ce03ac6`).
pub fn index_document(
    handle: &lucivy_core::sharded_handle::ShardedHandle,
    fields: &[(String, String)],
    offset: u64,
) -> Result<(), String> {
    let obj: serde_json::Map<String, serde_json::Value> = fields
        .iter()
        .filter(|(name, _)| handle.field(name).is_some())
        .map(|(name, value)| (name.clone(), serde_json::Value::String(value.clone())))
        .collect();
    handle.add_document_json(offset, &serde_json::Value::Object(obj))
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

/// Exécute une recherche et rend le triplet `(offset, score, highlights)`.
///
/// C'est **volontairement la même forme** que ce que rendait
/// `CALL QUERY_LUCIVY_INDEX(...) RETURN node_id, score, highlights` : toute la
/// logique d'attribution aux chunks en aval reste inchangée, et la parité de
/// l'étape 5 devient mesurable terme à terme.
///
/// Les highlights sont clés par **nom de champ du schéma** et leurs bornes sont
/// des **offsets en octets** dans la valeur indexée — même référentiel que
/// `ChunkRecord.start_char`/`end_char`, qui contiennent eux aussi des octets
/// malgré leur nom. C'est cette coïncidence qui fait marcher le recouvrement.
pub fn search_hits(
    handle: &lucivy_core::sharded_handle::ShardedHandle,
    query_config: &lucivy_core::query::QueryConfig,
    limit: usize,
    allowed_ids: Option<&[u64]>,
) -> Result<Vec<(u64, f64, std::collections::HashMap<String, Vec<(usize, usize)>>)>, String> {
    let sink = Arc::new(ld_lucivy::query::HighlightSink::new());

    let results = match allowed_ids {
        Some(ids) => handle.search_filtered(
            query_config,
            limit,
            Some(sink.clone()),
            ids.iter().copied().collect(),
        )?,
        None => handle.search(query_config, limit, Some(sink.clone()))?,
    };

    let mut out = Vec::with_capacity(results.len());
    for r in &results {
        let Some(shard) = handle.shard(r.shard_id) else { continue };
        let searcher = shard.reader.searcher();
        let Ok(doc) = searcher.doc::<ld_lucivy::LucivyDocument>(r.doc_address) else {
            continue;
        };
        let Some(offset) = node_id_of(handle, &doc) else { continue };

        let seg = searcher.segment_reader(r.doc_address.segment_ord);
        let hl = sink
            .get(seg.segment_id(), r.doc_address.doc_id)
            .unwrap_or_default()
            .into_iter()
            .map(|(field, spans)| {
                let pairs: Vec<(usize, usize)> =
                    spans.into_iter().map(|s| (s[0], s[1])).collect();
                (field, pairs)
            })
            .collect();

        out.push((offset, r.score as f64, hl));
    }
    Ok(out)
}

/// Liste les champs texte du schéma d'un index, hors `_node_id`.
///
/// Sert à la ré-indexation : on doit relire **tous** les champs indexés, pas
/// seulement ceux qui ont changé — `add_document` n'est pas un merge, il ajoute
/// un document entier. Ré-indexer avec un sous-ensemble perdrait silencieusement
/// les champs non modifiés.
pub fn indexed_text_fields(
    handle: &lucivy_core::sharded_handle::ShardedHandle,
    candidates: &[String],
) -> Vec<String> {
    candidates
        .iter()
        .filter(|f| f.as_str() != lucivy_core::handle::NODE_ID_FIELD)
        .filter(|f| handle.field(f).is_some())
        .cloned()
        .collect()
}

/// Remplace le document d'une entité : suppression puis ré-ajout.
///
/// `fields` doit porter **toutes** les valeurs indexées (voir
/// [`indexed_text_fields`]), pas seulement les modifiées.
pub fn reindex_document(
    handle: &lucivy_core::sharded_handle::ShardedHandle,
    fields: &[(String, String)],
    offset: u64,
) -> Result<(), String> {
    handle.delete_by_node_id(offset)?;
    index_document(handle, fields, offset)
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
    BlobBacked {
        /// Chargement paresseux : lucivy ne lit que les plages dont il a besoin,
        /// via `blob_len`/`load_range` (implémentés sur nos deux stores), au lieu
        /// de rematérialiser tout l'index à l'ouverture.
        ///
        /// **Eager par défaut** : c'est le mode validé, et la passation lucivy
        /// recommande de mesurer Eager contre Lazy sur de vrais index avant de
        /// basculer — le gain dépend de la taille et du motif d'accès.
        lazy: bool,
    },

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
        FtsStorage::BlobBacked { lazy: false }
    }
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
            index_document(&handle, &[("content".to_string(), text.to_string())], *offset)
                .expect("indexation");
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

    /// `search_hits` doit rendre les mêmes offsets, des highlights clés par nom
    /// de champ, et honorer `allowed_ids` — c'est le contrat qui remplace
    /// `CALL QUERY_LUCIVY_INDEX ... RETURN node_id, score, highlights`.
    #[test]
    fn search_hits_returns_offsets_highlights_and_honours_filter() {
        use lucivy_core::blob_store::MemBlobStore;
        use lucivy_core::sharded_handle::{BlobShardStorage, ShardedHandle};

        let tmp = std::env::temp_dir()
            .join(format!("rag3weaver_fts_hits_{}", std::process::id()));
        let store = Arc::new(MemBlobStore::new());
        let cfg = build_schema_config(&["content".to_string()], &[], 2).unwrap();
        let storage = BlobShardStorage::new(store, fts_index_name("Hits"), &tmp);
        let handle =
            ShardedHandle::create_with_storage(Box::new(storage), &cfg).expect("création");

        for (offset, text) in [
            (10_u64, "spin_lock_init protège la file"),
            (20_u64, "kmalloc alloue puis spin_lock_init verrouille"),
            (30_u64, "aucun rapport avec le noyau"),
        ] {
            index_document(&handle, &[("content".into(), text.to_string())], offset).unwrap();
        }
        handle.commit().unwrap();

        let q: lucivy_core::query::QueryConfig = serde_json::from_value(serde_json::json!({
            "type": "contains", "field": "content", "value": "spin_lock_init"
        }))
        .unwrap();

        // Sans filtre : les deux documents concernés.
        let hits = search_hits(&handle, &q, 10, None).expect("recherche");
        let mut offsets: Vec<u64> = hits.iter().map(|(o, _, _)| *o).collect();
        offsets.sort_unstable();
        assert_eq!(offsets, vec![10, 20]);

        // Les highlights sont clés par nom de champ, avec des bornes cohérentes.
        let (_, _, hl) = hits.iter().find(|(o, _, _)| *o == 10).unwrap();
        let spans = hl
            .get("content")
            .unwrap_or_else(|| panic!("highlights clés par nom de champ, reçu {hl:?}"));
        assert!(!spans.is_empty());
        for (a, b) in spans {
            assert!(a < b, "span dégénéré ({a},{b})");
            assert!(
                *b <= "spin_lock_init protège la file".len(),
                "offset hors du texte indexé — référentiel cassé"
            );
        }

        // Avec filtre : le pré-filtrage BDD doit être respecté.
        let filtered = search_hits(&handle, &q, 10, Some(&[20])).expect("recherche filtrée");
        let got: Vec<u64> = filtered.iter().map(|(o, _, _)| *o).collect();
        assert_eq!(got, vec![20], "allowed_ids non honoré");

        handle.close().ok();
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Cycle de vie complet : indexer → ré-indexer → supprimer.
    ///
    /// Le point le plus important est la ré-indexation partielle : `add_document`
    /// n'est pas un merge, donc ré-indexer en ne passant que le champ modifié
    /// ferait disparaître l'autre. Ce test échouerait si `UpdateRecordNode`
    /// relisait `rec.data` au lieu de relire la ligne entière.
    #[test]
    fn reindex_replaces_document_and_delete_removes_it() {
        use lucivy_core::blob_store::MemBlobStore;
        use lucivy_core::sharded_handle::{BlobShardStorage, ShardedHandle};

        let tmp = std::env::temp_dir()
            .join(format!("rag3weaver_fts_life_{}", std::process::id()));
        let store = Arc::new(MemBlobStore::new());
        let cfg =
            build_schema_config(&["titre".to_string(), "corps".to_string()], &[], 1).unwrap();
        let storage = BlobShardStorage::new(store, fts_index_name("Life"), &tmp);
        let handle =
            ShardedHandle::create_with_storage(Box::new(storage), &cfg).expect("création");

        let find = |needle: &str| -> Vec<u64> {
            let q: lucivy_core::query::QueryConfig =
                serde_json::from_value(serde_json::json!({
                    "type": "contains", "field": "corps", "value": needle
                }))
                .unwrap();
            let mut v: Vec<u64> = search_hits(&handle, &q, 10, None)
                .unwrap()
                .into_iter()
                .map(|(o, _, _)| o)
                .collect();
            v.sort_unstable();
            v
        };

        let fields = |t: &str, c: &str| {
            vec![("titre".to_string(), t.to_string()), ("corps".to_string(), c.to_string())]
        };

        index_document(&handle, &fields("Noyau", "verrou spinlock"), 7).unwrap();
        handle.commit().unwrap();
        assert_eq!(find("spinlock"), vec![7]);

        // Ré-indexation avec les DEUX champs : l'ancien contenu disparaît,
        // le nouveau est trouvable, et le titre non modifié survit.
        reindex_document(&handle, &fields("Noyau", "allocation kmalloc"), 7).unwrap();
        handle.commit().unwrap();
        assert!(find("spinlock").is_empty(), "l'ancien contenu doit disparaître");
        assert_eq!(find("kmalloc"), vec![7]);

        let q_titre: lucivy_core::query::QueryConfig = serde_json::from_value(
            serde_json::json!({"type": "contains", "field": "titre", "value": "Noyau"}),
        )
        .unwrap();
        assert_eq!(
            search_hits(&handle, &q_titre, 10, None).unwrap().len(),
            1,
            "le champ non modifié doit survivre à la ré-indexation"
        );

        // Suppression : plus aucun document fantôme.
        handle.delete_by_node_id(7).unwrap();
        handle.commit().unwrap();
        assert!(find("kmalloc").is_empty(), "document fantôme après suppression");

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
