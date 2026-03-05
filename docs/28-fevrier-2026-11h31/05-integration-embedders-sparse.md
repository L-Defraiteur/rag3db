# 05 — Intégration embedders + sparse_vector extension dans rag3weaver

## Ce qui a été fait

### Phase 1 : API Catalog pour Arc partagé (FAIT)

**Problème** : `Catalog::new()` prenait `Box<dyn Embedder>` et `set_sparse_embedder()` prenait `Box<dyn SparseEmbedder>`. Impossible de partager un seul `Arc<BgeM3Embedder>` entre les deux rôles (2.2GB VRAM, on ne veut pas charger 2 fois).

**Solution** :
- `Catalog::new()` reste avec `Box<dyn Embedder>` (backward compatible)
- Ajout de `set_embedder(Arc<dyn Embedder>)` — remplace l'embedder par un Arc partagé
- `set_sparse_embedder()` changé de `Box<dyn SparseEmbedder>` à `Arc<dyn SparseEmbedder>`

**Usage BGE-M3 dual-role** :
```rust
let bge = Arc::new(BgeM3Embedder::new()?);
let mut catalog = Catalog::new(Box::new(conn), Box::new(placeholder), config);
catalog.set_embedder(bge.clone() as Arc<dyn Embedder>);
catalog.set_sparse_embedder(bge as Arc<dyn SparseEmbedder>);
// → un seul modèle, deux rôles, 2.2GB VRAM total
```

**Note** : on a d'abord essayé `impl Into<Arc<dyn ...>>` mais ça ne compile pas — la coercion d'unsizing (`Box<ConcreteType>` → `Arc<dyn Trait>`) ne compose pas avec `From`/`Into` en Rust. D'où les deux approches séparées (Box pour new, Arc pour set).

**Fichiers** : `catalog.rs` (struct + new + set_sparse_embedder + set_embedder)

### Phase 2 : Sparse in-memory → sparse_vector extension Cypher (FAIT, en cours de test)

**Avant** : `SparseIndex` = HashMap in-memory, rebuild complet depuis la DB à chaque `initialize()` et après chaque `drain()`. Recherche = dot-product brute force en RAM.

**Après** : L'extension C++ `sparse_vector` gère tout — persistance bincode, hooks INSERT/DELETE/UPDATE automatiques, lazy commit avec dirty flag.

#### Changements effectués :

**a) Nouveau `search_sparse_cypher()` dans search.rs** :
```rust
pub async fn search_sparse_cypher(
    conn: &dyn DbConnection,
    entity: &str,
    query_vector: &SparseVector,
    limit: usize,
) -> Result<Vec<SearchResult>, CatalogError>
```

Deux requêtes Cypher :
1. `CALL QUERY_SPARSE_VECTOR_INDEX('{entity}', [{indices}], [{weights}], {limit}) RETURN node_id, score`
2. `MATCH (n:{entity}) WHERE OFFSET(id(n)) IN [{offset_list}] RETURN OFFSET(id(n)), n._uuid`

Le join offset → uuid est fait en Rust (HashMap). `OFFSET()` est une fonction built-in de rag3db qui extrait l'offset d'un InternalID.

**b) `Catalog::initialize()`** : remplacé `rebuild_sparse_index()` par :
```rust
CALL CREATE_SPARSE_VECTOR_INDEX('{entity}', '{kb}_sparse_indices', '{kb}_sparse_weights')
```
Appelé pour chaque paire (entity, kb) qui a `sparse=true`. Idempotent — les erreurs sont ignorées (index déjà existant).

**c) `Catalog::search()`** : remplacé `search::search_sparse(sparse_idx, &qv, ...)` par `search::search_sparse_cypher(conn, entity, &qv, limit)`.

**d) `Catalog::drain()`** : supprimé le rebuild sparse post-drain. L'extension maintient son index via les hooks automatiques.

**e) `Catalog::delete()`** : supprimé `sparse_idx.remove(uuid)`. L'extension gère les suppressions via le hook DELETE.

**f) Supprimé du Catalog** :
- `sparse_indexes: HashMap<String, SparseIndex>` (champ)
- `rebuild_sparse_index()` (méthode, ~70 lignes)
- Import `SparseIndex` (seul `SparseVector` reste, utilisé par `SparseEmbedProcessor`)

#### Nettoyage warnings :
- Supprimé `ChunkOp` import inutile dans queue.rs
- Supprimé `build_embed_texts()` dead code dans catalog.rs (early attempt pré-chunking, remplacé par l'approche per-chunk avec titre prépendé)

### État compilation/tests

- `cargo check` : 0 erreur, 0 warning
- `cargo check --tests` : 0 erreur, 1 warning préexistant (`FilterOp` unused)
- `cargo test` : **351 tests passed**, 0 failed, 13 ignored

### Bug découvert (non corrigé, à noter)

**`search_bm25` retourne des offsets au lieu de UUIDs** : `QUERY_LUCIVY_INDEX` retourne `node_id` (UINT64 offset), et `search_bm25()` le stocke tel quel comme "uuid" dans `SearchResult`. En mode hybride, la fusion ne peut pas matcher les résultats BM25 (uuid="42") avec les résultats vector (uuid="abc-def-123"). Même problème potentiel pour `allowed_ids` : `RETURN id(n)` retourne un `InternalID` qui passe par le catch-all `CypherValue::String("table_id:offset")` dans `rag3db_value_to_cypher`, et `as_i64()` dessus retourne `None`.

**Fix suggéré** : utiliser `OFFSET(id(n))` au lieu de `id(n)` pour les allowed_ids, et résoudre les node_ids → UUIDs dans `search_bm25` (même pattern que `search_sparse_cypher`).

## Architecture sparse après intégration

```
Avant:
  SparseEmbedProcessor → SET colonnes DB (indices + weights)
  drain() → rebuild_sparse_index() → SparseIndex in-memory (HashMap)
  search() → sparse_index.search() (dot-product RAM)

Après:
  SparseEmbedProcessor → SET colonnes DB (indices + weights)
  ↓ hooks automatiques de l'extension ↓
  sparse_vector extension → insert dans index Rust (bincode, persistant)
  search() → QUERY_SPARSE_VECTOR_INDEX (Cypher) → resolve offsets → UUIDs
```

## Fichiers modifiés

| Fichier | Changement |
|---------|-----------|
| `src/catalog.rs` | API Arc, suppression SparseIndex in-memory, CREATE_SPARSE_VECTOR_INDEX à initialize, search via Cypher |
| `src/search.rs` | Nouveau `search_sparse_cypher()` (2 queries + join Rust) |
| `src/queue.rs` | Suppression import `ChunkOp` inutile |

## Phase 3 : WASM MultilingualMiniLM (PAS ENCORE FAIT)

- Modifier `wasm_ffi.rs` pour accepter model bytes depuis JS au lieu de MockEmbedder
- Nouvelle FFI : `rag3weaver_catalog_new_with_model(config, db_path, config_bytes, tokenizer_bytes, weights_bytes)`
- Créer un `CandleEmbedder::from_bytes()` → 384d MultilingualMiniLM
- Pas de sparse pour WASM (pour l'instant)

## Points d'attention

- **Extension doit être chargée** : `LOAD EXTENSION 'path/to/libsparse_vector.rag3db_extension'` avant initialize() en mode dynamique. En mode statique (WASM), c'est automatique.
- **`OFFSET(id(n))`** : fonction built-in rag3db, prend INTERNAL_ID → INT64. Utilisée pour résoudre les node_id retournés par l'extension.
- **SparseVector reste** : le type `SparseVector` (indices + weights) est toujours utilisé par `SparseEmbedder` trait et `SparseEmbedProcessor`. Seul `SparseIndex` (l'index in-memory) est devenu unused dans catalog.rs.
- **`sparse_index.rs` reste** : le module existe toujours, `SparseVector` est re-exporté depuis lib.rs. `SparseIndex` y reste disponible pour d'autres usages potentiels.
