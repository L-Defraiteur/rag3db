# 04 — Plan d'intégration : BGE-M3 + sparse_vector extension + MultilingualMiniLM

## Contexte

On a 3 briques prêtes mais pas câblées dans le Catalog :
1. **BGE-M3** (dense 1024d + sparse learned) — natif, GPU
2. **sparse_vector extension** (C++ Cypher) — remplacer l'index sparse in-memory
3. **MultilingualMiniLM** (dense 384d) — WASM, 50+ langues

## État actuel du câblage

### Catalog (catalog.rs)

```rust
pub struct Catalog {
    embedder: Arc<dyn Embedder>,                      // dense — injecté via Catalog::new()
    sparse_embedder: Option<Arc<dyn SparseEmbedder>>, // sparse — set_sparse_embedder()
    sparse_indexes: HashMap<String, SparseIndex>,     // IN-MEMORY, rebuild à chaque drain()
    ...
}
```

- Le Catalog est **embedder-agnostic** : on passe un `Box<dyn Embedder>` à la construction
- `EmbeddingConfig` dans la config est **dead code** — jamais lu par le Catalog
- L'embedder est toujours injecté de l'extérieur (l'appelant choisit)

### Sparse : 100% in-memory

- `SparseIndex` = HashMap postings + HashMap vectors, pur RAM
- Rebuild complet depuis la DB après chaque `drain()` (lignes 675-688)
- Rebuild complet à `initialize()` (lignes 271-277)
- Recherche : `sparse_index.search(query_vector, limit)` — dot-product brute force
- **Aucune référence** à `CREATE_SPARSE_VECTOR_INDEX` / `QUERY_SPARSE_VECTOR_INDEX` nulle part

### WASM FFI (wasm_ffi.rs)

- `rag3weaver_catalog_new()` utilise un **MockEmbedder** (vecteurs zéro) en dur
- `rag3weaver_candle_embed()` existe comme FFI standalone mais PAS intégré au Catalog
- Pas de `set_embedder()` dans le FFI, pas de moyen de changer l'embedder après création

### Search flow (catalog.rs:792-1076)

1. Embed query → dense embedder
2. Vector search → `CALL QUERY_VECTOR_INDEX(...)` (Cypher)
3. BM25 search → `CALL QUERY_TANTIVY_INDEX(...)` (Cypher)
4. Sparse search → `sparse_index.search()` (in-memory)
5. Fusion RRF ou Weighted 3-way

## Plan d'intégration

### 1. BGE-M3 comme embedder natif (dense + sparse en un seul objet)

**Avantage clé** : `BgeM3Embedder` implémente à la fois `Embedder` ET `SparseEmbedder`.

```rust
// Un seul modèle, deux rôles
let bge = Arc::new(BgeM3Embedder::new()?);   // GPU auto
let mut catalog = Catalog::new(conn, Box::new(bge.clone()), config);
catalog.set_sparse_embedder(Box::new(bge));   // même instance
```

**Fichiers à modifier** :
- Rien dans `catalog.rs` — le design injecté fonctionne déjà
- Il faut juste un point d'entrée qui fait ce câblage : exemple/test/helper

**Impact** :
- `embedding_dim` passe à 1024 dans la config (au lieu de 384)
- Les colonnes vector index passent à 1024 dims
- Forward pass ~80ms sur GPU (RTX 2070)

### 2. Remplacer SparseIndex in-memory par sparse_vector extension

**Étapes** :

a) **Nouveau module `sparse_cypher.rs`** — wrapper Cypher autour de l'extension :
```rust
pub async fn create_sparse_index(conn: &dyn DbConnection, entity: &str, indices_col: &str, weights_col: &str) -> Result<()>;
pub async fn query_sparse_index(conn: &dyn DbConnection, entity: &str, q_indices: &[u32], q_weights: &[f32], limit: usize) -> Result<Vec<(String, f32)>>;
pub async fn drop_sparse_index(conn: &dyn DbConnection, entity: &str) -> Result<()>;
```

b) **Modifier `search::search_sparse()`** (search.rs:671) :
- Actuel : prend `&SparseIndex` (in-memory), appelle `sparse_index.search()`
- Nouveau : prend `&dyn DbConnection`, appelle `QUERY_SPARSE_VECTOR_INDEX` via Cypher
- Plus besoin de `sparse_indexes: HashMap<String, SparseIndex>` dans Catalog

c) **Modifier `Catalog::initialize()`** :
- Remplacer `rebuild_sparse_index()` par `create_sparse_index()` (idempotent)
- L'extension gère la persistance (bincode), plus de rebuild RAM

d) **Modifier `SparseEmbedProcessor::process()`** :
- Actuel : stocke dans colonnes DB (indices + weights)
- Ajouter : appeler `add_sparse_document()` après le SET Cypher
- Ou : laisser le SET Cypher et utiliser les hooks INSERT de l'extension

e) **Modifier post-drain** :
- Supprimer `rebuild_sparse_index()` — l'extension est déjà à jour via les hooks
- Ou au pire : `sparse_commit()` pour flusher le dirty flag

f) **Supprimer** :
- `sparse_indexes` du Catalog struct
- `rebuild_sparse_index()`
- L'import de `SparseIndex` dans catalog.rs

**Fichiers** :
- `src/sparse_cypher.rs` (nouveau)
- `src/catalog.rs` (retirer in-memory sparse, appeler Cypher)
- `src/search.rs` (search_sparse via Cypher au lieu de &SparseIndex)
- `src/schema.rs` (ajouter `CREATE_SPARSE_VECTOR_INDEX` au DDL)
- `src/lib.rs` (déclarer le module)

### 3. MultilingualMiniLM pour WASM

**Étapes** :

a) **Modifier `rag3weaver_catalog_new()`** dans `wasm_ffi.rs` :
- Au lieu de `MockEmbedder`, accepter des bytes model depuis JS
- Nouvelle FFI : `rag3weaver_catalog_new_with_model(config, db_path, config_bytes, tokenizer_bytes, weights_bytes)`
- Crée un `CandleEmbedder::from_bytes(config, tokenizer, weights)` → 384d

b) **Ou** : séparer en deux appels FFI :
```c
WeaverContext* ctx = rag3weaver_catalog_new(config, db_path);       // MockEmbedder temp
rag3weaver_catalog_set_embedder(ctx, config_bytes, tok_bytes, weights_bytes); // CandleEmbedder
```

c) **Pas de sparse pour WASM** (pour l'instant) — BM42 est là "au cas où" mais pas câblé. Le search WASM sera Semantic ou Hybrid (vector + BM25) sans sparse.

d) **Côté JS** : fetch les 3 fichiers du modèle (config.json 645B + tokenizer.json 9MB + model.safetensors 471MB), passer en Uint8Array au FFI.

**Fichiers** :
- `src/wasm_ffi.rs` (nouvelle FFI avec model bytes)
- `tools/wasm/src_cpp/weaver_bindings.cpp` (exposer à emscripten)

## Ordre d'exécution recommandé

1. **BGE-M3 câblage** — le plus simple, juste un exemple/test qui montre l'injection, 0 modif Catalog
2. **sparse_vector Cypher** — plus gros morceau, remplace le in-memory par l'extension
3. **WASM MultilingualMiniLM** — modifier le FFI layer

## Points d'attention

- **embedding_dim mismatch** : BGE-M3 = 1024d, MiniLM = 384d. Si on veut que la même DB serve natif et WASM, il faut choisir. Options :
  - Colonnes séparées par KB (déjà le cas : `{kb_name}_embedding`)
  - Ou forcer un dim unique par déploiement

- **Arc partagé** pour BGE-M3 : un seul `Arc<BgeM3Embedder>` sert de dense ET sparse embedder. Le modèle (2.2GB VRAM) est chargé une seule fois.

- **Performance sparse_vector extension vs in-memory** : L'extension utilise bincode + HashMap, similaire au in-memory mais persistant. Pas de régression de perf attendue (le goulot c'est le forward pass, pas le search).

- **Rebuild supprimé** : Plus de `rebuild_sparse_index()` après chaque drain. L'extension a ses propres hooks INSERT/DELETE + dirty flag + lazy commit.

## Embedders par contexte

| Contexte | Dense Embedder | Sparse Embedder | Dim |
|----------|---------------|-----------------|-----|
| Natif (Node.js/C++/Rust) | BgeM3Embedder (CUDA) | BgeM3Embedder (même instance) | 1024 |
| WASM default | CandleEmbedder(MultilingualMiniLM) | aucun (ou BM42 si besoin) | 384 |
| WASM light | CandleEmbedder(MiniLM) | aucun | 384 |
| Test/dev | MockEmbedder | MockSparseEmbedder | configurable |
