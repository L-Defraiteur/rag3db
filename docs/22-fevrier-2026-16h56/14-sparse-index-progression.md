# 14 — Sparse Index V1 : rapport de progression

## Statut global : ~60% fait

## Ce qui est FAIT (fichiers ecrits et complets)

### 1. Doc 13 ecrit
- `docs/22-fevrier-2026-16h56/13-sparse-index-rag3weaver.md` — architecture et decisions

### 2. `sparse_index.rs` — NOUVEAU, COMPLET
- Chemin : `extension/rag3weaver/src/sparse_index.rs`
- `SparseVector` : struct avec `indices: Vec<u32>` + `values: Vec<f32>`, constructeur avec assert longueur
- `SparseIndex` : index inverse en memoire
  - `postings: HashMap<u32, Vec<(String, f32)>>` — token_id → [(uuid, weight)]
  - `vectors: HashMap<String, SparseVector>` — uuid → vecteur (pour delete)
  - Methodes : `new()`, `len()`, `is_empty()`, `insert()`, `remove()`, `search()`, `clear()`
  - `search()` : accumule dot product via posting lists, retourne top-k
- 9 tests unitaires : basics, insert/search, remove, replace, limit, disjoint, empty, clear, clean postings

### 3. `embedder.rs` — MODIFIE, COMPLET
- Chemin : `extension/rag3weaver/src/embedder.rs`
- Ajout `use crate::sparse_index::SparseVector;` et `use std::collections::HashMap;`
- Nouveau trait `SparseEmbedder` : `async fn embed_sparse(&self, texts: &[String]) -> Result<Vec<SparseVector>, EmbedError>`
- `MockSparseEmbedder` : hash djb2 des mots → token_ids dans [0, 30000), poids = 1/num_words, indices tries
- `CallbackSparseEmbedder` : meme pattern que CallbackEmbedder, avec `SparseEmbedFn` type alias
- 6 tests : basic, deterministic, empty, indices sorted, callback basic, trait object

### 4. `config.rs` — MODIFIE, COMPLET
- Chemin : `extension/rag3weaver/src/config.rs`
- Ajout sur `KBConfig` :
  - `pub sparse: bool` (default false, serde default)
  - `pub sparse_weight: f64` (default 0.2, via `default_sparse_weight()`)
- Default impl mis a jour avec `sparse: false, sparse_weight: default_sparse_weight()`

### 5. `ops.rs` — MODIFIE, COMPLET
- Chemin : `extension/rag3weaver/src/ops.rs`
- Nouveau struct `SparseEmbedOp { entity_ref, kb_name, texts }`
- Nouveau variant `CatalogOp::SparseEmbed(SparseEmbedOp)`
- Les 3 match (priority, operation_type, config) mis a jour pour le nouveau variant
- Nouvelle constante `OP_SPARSE_EMBED` : priority 3, batch_size 32, max_retries 2

### 6. `schema.rs` — MODIFIE, COMPLET
- Chemin : `extension/rag3weaver/src/schema.rs`
- `generate_node_table_ddl` : nouveau param `kb_configs: &HashMap<String, KBConfig>`
- Quand `kb.sparse == true`, ajoute colonnes `{kb}_sparse_indices INT64[]` et `{kb}_sparse_weights DOUBLE[]`
- Appel dans `generate_full_schema` mis a jour avec `&config.knowledge_bases`
- 4 tests existants mis a jour (ajout `&HashMap::new()` param)
- 1 nouveau test `node_table_with_sparse` qui verifie les colonnes sparse

## Ce qui est EN COURS

### 7. `search.rs` — EN COURS (~20% fait)
- Import ajoute : `use crate::sparse_index::{SparseIndex, SparseVector};`
- Module doc mis a jour
- **RESTE A FAIRE :**
  - Ajouter `sparse_weight: Option<f64>` a `SearchOptions` + Default
  - Ajouter `sparse_count: usize` a `SearchMeta`
  - Nouvelle fonction `search_sparse(sparse_index, query_vector, entity, limit) -> Vec<SearchResult>`
  - Etendre `fuse_results()` : ajouter params `sparse_results` et `sparse_weight`
  - Nouvelle fonction `fuse_rrf_n(result_lists: &[&[SearchResult]], rrf_k) -> Vec<SearchResult>` — utilise `fusion::rrf_fuse` qui gere deja N listes
  - Nouvelle fonction `fuse_weighted_3way(vector, bm25, sparse, keyword_weight, sparse_weight)` — normalise BM25 et sparse, puis `(1-kw-sw)*vec + kw*bm25 + sw*sparse`
  - Strategie Boost avec sparse : fallback vers RRF (boost ne s'etend pas a 3 signaux)
  - Tests pour les nouvelles fonctions

## Ce qui RESTE A FAIRE

### 8. `catalog.rs` — PAS COMMENCE
- Nouveaux champs sur `Catalog` : `sparse_embedder: Option<Arc<dyn SparseEmbedder>>`, `sparse_indexes: HashMap<String, SparseIndex>`
- `set_sparse_embedder()` setter
- `SparseEmbedProcessor` struct + impl Processor (meme pattern que EmbedProcessor)
- `rebuild_sparse_indexes()` : Cypher MATCH pour charger sparse depuis DB
- `initialize()` : registrer SparseEmbedProcessor, init indexes, rebuild
- `create()` : enqueue SparseEmbedOp si KB sparse
- `update()` : re-enqueue SparseEmbedOp si content change
- `delete()` : sparse_index.remove(uuid)
- `drain()` : appeler rebuild_sparse_indexes apres drain
- `search()` : embed query sparse, search_sparse, passer 3 listes a fuse_results

### 9. `lib.rs` — PAS COMMENCE
- `pub mod sparse_index;`
- Re-exports : `SparseIndex`, `SparseVector`, `SparseEmbedder`, `MockSparseEmbedder`, `CallbackSparseEmbedder`, `SparseEmbedOp`, `OP_SPARSE_EMBED`

### 10. Tests — PAS COMMENCE
- `cargo test --lib` pour verifier compilation + tous tests passent
- Fix des erreurs de compilation eventuelles

## Decisions d'architecture (rappel)

1. **SparseEmbedder = trait separe** de Embedder (pas d'extension)
2. **sparse = flag orthogonal** sur KBConfig (pas de nouveau SearchMode)
3. **SparseIndex en memoire** sur Catalog, persist via colonnes DB, rebuild a initialize()
4. **fusion::rrf_fuse** gere deja N listes — ajouter sparse = juste append a la liste
5. **Pas de nouveau dep Cargo** — tout en std

## Fichiers de reference

| Fichier | Statut |
|---|---|
| `src/sparse_index.rs` | FAIT |
| `src/embedder.rs` | FAIT |
| `src/config.rs` | FAIT |
| `src/ops.rs` | FAIT |
| `src/schema.rs` | FAIT |
| `src/search.rs` | EN COURS (import fait, reste fonctions + types) |
| `src/catalog.rs` | A FAIRE |
| `src/lib.rs` | A FAIRE |

## Plan de reprise

Le plan complet est dans `/home/luciedefraiteur/.claude/plans/federated-dancing-firefly.md`.

Pour reprendre :
1. Finir search.rs (SearchOptions, SearchMeta, search_sparse, fuse_results etendu, fuse_rrf_n, fuse_weighted_3way)
2. Wiring catalog.rs (le plus gros morceau restant)
3. lib.rs (exports)
4. cargo test --lib
