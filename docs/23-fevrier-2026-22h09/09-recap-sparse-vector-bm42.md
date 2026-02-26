# 09 — Recap : Extension sparse_vector + BM42 embedder

## Ce qui a été fait cette session (24 février 2026)

Deux gros morceaux : l'extension C++ `sparse_vector` pour rag3db, et le `Bm42Embedder` dans rag3weaver.

---

## 1. Extension C++ `sparse_vector` — TERMINÉ, 6/6 tests E2E

### Architecture

```
extension/sparse_vector/
├── rust/                          ← Crate Rust STANDALONE (PAS dans ld-tantivy)
│   ├── Cargo.toml                 (staticlib, deps: cxx, serde, bincode)
│   ├── build.rs                   (cxx_build + flags emscripten)
│   └── src/
│       ├── lib.rs
│       ├── index.rs               (SparseVector + SparseIndex, u64 node_ids)
│       ├── handle.rs              (SparseHandle, create/open, persistence bincode)
│       └── bridge.rs              (cxx bridge: 1 struct, 8 fonctions)
├── CMakeLists.txt                 (cargo custom target + INTERFACE lib --whole-archive)
├── src/
│   ├── main/sparse_vector_extension.cpp
│   ├── index/sparse_vector_index.cpp    (hooks insert/delete/update, dirty_, flushIfDirty)
│   ├── function/
│   │   ├── create_sparse_vector_index.cpp  (public 3 params + internal 4 params, rewriteFunc)
│   │   ├── query_sparse_vector_index.cpp   (flushIfDirty, allowed_ids, inferInputTypes)
│   │   └── drop_sparse_vector_index.cpp
│   ├── catalog/sparse_vector_catalog_entry.cpp
│   └── include/                   (6 headers)
└── test/
    ├── CMakeLists.txt             (add_dependencies → extension rebuild automatique)
    └── sparse_vector_test.cpp     (6 tests GTest E2E)
```

### API Cypher

```cypher
CALL CREATE_SPARSE_VECTOR_INDEX('Document', 'sparse_indices', 'sparse_weights')
CALL QUERY_SPARSE_VECTOR_INDEX('Document', [1, 2, 3], [0.5, 0.3, 0.2], 10) RETURN node_id, score
CALL QUERY_SPARSE_VECTOR_INDEX('Document', $qi, $qw, 10, allowed_ids := $ids) RETURN node_id, score
CALL DROP_SPARSE_VECTOR_INDEX('Document')
```

### Crate Rust (`extension/sparse_vector/rust/`)

- Port de `rag3weaver/src/sparse_index.rs` avec **u64 node_ids** au lieu de String UUIDs
- `SparseVector` et `SparseIndex` dérivent `Serialize, Deserialize` pour bincode
- Persistence V1 : `sparse_commit()` → `bincode::serialize` → `{path}/sparse.bin`
- `open_sparse_index()` → `bincode::deserialize` depuis `sparse.bin`
- 13 tests Rust unitaires (search, delete, persistence, filtered search)
- Static lib 7.9 MB

### Bridge cxx

```rust
struct SparseSearchResult { node_id: u64, score: f32 }

extern "Rust" {
    type SparseHandle;
    fn create_sparse_index(path: &str) -> Result<Box<SparseHandle>>;
    fn open_sparse_index(path: &str) -> Result<Box<SparseHandle>>;
    fn add_sparse_document(handle, node_id: u64, indices: &[u32], weights: &[f32]) -> Result<i64>;
    fn delete_sparse_document(handle, node_id: u64) -> Result<i64>;
    fn sparse_search(handle, q_indices, q_weights, limit: u32) -> Vec<SparseSearchResult>;
    fn sparse_search_filtered(handle, q_indices, q_weights, limit, allowed_ids: &[u64]) -> Vec<...>;
    fn sparse_commit(handle) -> Result<i64>;
    fn sparse_num_docs(handle) -> u64;
}
```

### Extension C++ — pattern identique à tantivy_fts

- `SparseVectorIndex` : hooks `insert()`, `delete_()`, `update()` + `dirty_` flag + `flushIfDirty()` (commit avant QUERY)
- `extractSparseVector()` helper : extrait `LIST[INT64]` + `LIST[DOUBLE]` via `ListVector::getDataVector()` + `list_entry_t`
- CREATE : bind public/internal, `rewriteFunc`, scan rows existantes, commit, catalog entry, `nodeTable.addIndex()`
- QUERY : `inferInputTypes` avec `LIST(INT64)` + `LIST(DOUBLE)` (pas juste `LIST` — sinon binder error), `flushIfDirty()`, optional `allowed_ids`
- DROP : même pattern que tantivy_fts
- Catalog entry : `SparseVectorIndexAuxInfo` (indexPath, indicesColumnName, weightsColumnName)

### Tests E2E (6/6 verts)

| Test | Ce qu'il valide |
|---|---|
| CreateAndQuery | Créer index + 3 docs + query dot product |
| Delete | Supprimer un doc → absent des résultats |
| Update | Modifier sparse vector → ancien token absent, nouveau présent |
| Persistence | Créer → fermer DB → rouvrir → mêmes résultats |
| FilteredSearch | `allowed_ids` filtre correctement |
| Drop | Drop index → query échoue |

### Fichiers modifiés (existants)

- `extension/extension_config.cmake` : ajouté `sparse_vector` à EXTENSION_LIST + blocs WASM/Android/Swift
- `extension/CMakeLists.txt` : ajouté `add_extension_if_enabled("sparse_vector")` en ligne 96

### Build

```bash
cd packages/rag3db/build/sparse_test
cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSION_TESTS=TRUE \
  -DBUILD_EXTENSIONS="sparse_vector" -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE
cmake --build . --target sparse_vector_test -j$(nproc)
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu ./extension/sparse_vector/test/sparse_vector_test
```

Un seul target suffit — `add_dependencies(sparse_vector_test rag3db_sparse_vector_extension)` dans `test/CMakeLists.txt` garantit le rebuild de l'extension + Rust lib.

---

## 2. BM42 Embedder — TERMINÉ, 6/6 tests

### Problème résolu

Le `BertSelfAttention` de candle ne retourne pas les attention_probs — il les calcule dans `forward()` mais ne les expose pas. Tous les structs internes sont privés.

### Solution : BERT réécrit avec attention output

**`bm42_model.rs`** (~210 lignes) dans `extension/rag3weaver/src/` :

- Réécrit compact des couches BERT (Embeddings, SelfAttention, Attention, FeedForward, Layer, Encoder, Model)
- `SelfAttention::forward()` retourne `(context_layer, attention_probs)` au lieu de juste `context_layer`
- Même poids safetensors (mêmes noms VarBuilder), même fallback model_type prefix
- Pas de tracing spans (inutile pour notre usage), pas de Dropout (no-op en inférence)
- `Bm42Model::forward()` → `(hidden_states, last_attention_probs)`
  - hidden_states : `[batch, seq_len, hidden_size]`
  - last_attention_probs : `[batch, num_heads, seq_len, seq_len]`

**`bm42_embedder.rs`** (~220 lignes) :

- `Bm42Embedder` implémente `SparseEmbedder`
- Même constructeurs que `CandleEmbedder` : `new(DefaultModel)`, `from_repo()`, `from_bytes()` (WASM-compatible)
- Même modèle (all-MiniLM-L6-v2, ~22MB) — zéro download supplémentaire
- Feature gates : `candle-embedder` (natif + HF Hub) / `candle-wasm` (from_bytes seulement)

### Pipeline BM42

```
Texte → Tokenizer WordPiece → BERT forward → attention_probs [batch, heads, seq, seq]
  → mean across heads → [batch, seq, seq]
  → CLS row (position 0) → [batch, seq]  (attention du CLS vers chaque token)
  → pour chaque position :
      - skip special tokens ([CLS], [SEP], [PAD]) via special_tokens_mask
      - skip weights ≤ 0
      - accumule weight par token_id (somme si même sub-word apparaît 2x)
  → sort par token_id → SparseVector { indices, values }
```

Les dimensions du sparse vector sont des **token_ids WordPiece** (vocab ~30k), pas des mots entiers. C'est plus simple et ça fonctionne correctement car query et document utilisent le même tokenizer → mêmes token_ids → dot product exact.

### Tests (6/6 verts, ~2.6s total)

| Test | Ce qu'il valide |
|---|---|
| sparse_basic | Non-vide, ≥ 2 tokens, poids positifs |
| batch | 2 textes → 2 sparse vectors |
| deterministic | Même texte → même résultat |
| empty_batch | 0 texte → 0 résultat |
| similar_texts_share_tokens | "rust programming" et "rust systems programming" partagent des token_ids |
| as_sparse_embedder_trait | Fonctionne comme `Box<dyn SparseEmbedder>` |

Tous marqués `#[ignore]` (nécessitent le modèle téléchargé). Lancer avec :

```bash
cd packages/rag3db/extension/rag3weaver
cargo test --features candle-embedder bm42 -- --ignored
```

---

## 3. Bugs corrigés et leçons

### Extension sparse_vector

| Bug | Cause | Fix |
|---|---|---|
| Binder error "LIST without child information" | `LogicalTypeID::LIST` dans la signature du TableFunction ne spécifie pas le child type | Ajouté `inferInputTypes` qui retourne `LIST(INT64)` + `LIST(DOUBLE)` |
| `getValue()` rvalue bind error | `auto&` sur une rvalue retournée par `getValue()` | Changé en `auto` (copie par valeur) |
| `LogicalType` private copy constructor | Initializer list `{LogicalType::STRING(), ...}` dans `inferInputTypes` | Utilisé `push_back` au lieu d'initializer list |
| Test ne rebuild pas l'extension | `sparse_vector_test` ne dépendait pas de `rag3db_sparse_vector_extension` | Ajouté `add_dependencies()` dans `test/CMakeLists.txt` |
| Wrong test include | `graph_test/api_graph_test.h` n'existe pas | Changé en `api_test/api_test.h` (pattern tantivy_fts) |

### BM42 embedder

Aucun bug — compilé et testé du premier coup.

---

## 4. État complet du projet

### Ce qui est fait

| Composant | Statut |
|---|---|
| Extension tantivy_fts (BM25) | ✓ 9 tests E2E, Node.js natif, WASM |
| Extension sparse_vector | ✓ 6 tests E2E, Rust 13 tests |
| BM42 Embedder (candle) | ✓ 6 tests, même modèle que dense |
| Extension vector (HNSW) | ✓ (préexistant + wiring doc 01) |
| Fusion 3-way (RRF/Weighted/Boost) | ✓ (dans rag3weaver search.rs) |
| SparseEmbedder trait + MockSparseEmbedder | ✓ (dans rag3weaver embedder.rs) |
| SparseEmbedOp dans la queue | ✓ (dans rag3weaver ops.rs) |

### Ce qui reste (Phase 4 — intégration rag3weaver)

L'objectif : remplacer le `SparseIndex` in-memory de rag3weaver par l'extension `sparse_vector`.

**Avant** (rag3weaver gère tout en mémoire) :
```
create() → enqueue SparseEmbedOp → drain() → store colonnes DB → rebuild_sparse_index (scan O(N))
search() → sparse_indexes.get(kb).search(query, k) ← HashMap in-memory
```

**Après** (extension gère l'index) :
```
create() → enqueue SparseEmbedOp → drain() → store colonnes DB → hooks extension indexent auto
search() → CALL QUERY_SPARSE_VECTOR_INDEX(table, $qi, $qw, k) ← via Cypher
```

Changements rag3weaver :
1. **Supprimer** : `sparse_indexes: HashMap<String, SparseIndex>` du Catalog + `rebuild_sparse_index()`
2. **Modifier** : `search()` dans catalog.rs → appeler `QUERY_SPARSE_VECTOR_INDEX` via Cypher
3. **Modifier** : `initialize()` → ajouter `CALL CREATE_SPARSE_VECTOR_INDEX(...)` dans la génération de schéma
4. **Garder** : `SparseEmbedOp`, `SparseEmbedProcessor` (stocke les colonnes), `Bm42Embedder` / `SparseEmbedder` trait
5. **Optionnel** : brancher `Bm42Embedder` au lieu de `MockSparseEmbedder` dans le Catalog

### Évolutions futures (pas pour maintenant)

- **V2 persistance** : format on-disk par token + mmap + cache LRU (quand > 100k docs)
- **SPLADE** : modèle dédié MLM head (~65-130MB), expansion de termes, qualité max
- **Single forward pass** : dense + sparse BM42 en un seul passage BERT (optimisation perf)
- **Sub-word merging** : fusionner les sous-mots WordPiece ("hel" + "##lo" → "hello") pour moins de dimensions

---

## 5. Fichiers créés/modifiés (liste complète)

### Créés

```
# Rust crate (standalone)
extension/sparse_vector/rust/Cargo.toml
extension/sparse_vector/rust/build.rs
extension/sparse_vector/rust/src/lib.rs
extension/sparse_vector/rust/src/index.rs
extension/sparse_vector/rust/src/handle.rs
extension/sparse_vector/rust/src/bridge.rs

# Extension C++
extension/sparse_vector/CMakeLists.txt
extension/sparse_vector/src/main/CMakeLists.txt
extension/sparse_vector/src/main/sparse_vector_extension.cpp
extension/sparse_vector/src/index/CMakeLists.txt
extension/sparse_vector/src/index/sparse_vector_index.cpp
extension/sparse_vector/src/function/CMakeLists.txt
extension/sparse_vector/src/function/create_sparse_vector_index.cpp
extension/sparse_vector/src/function/query_sparse_vector_index.cpp
extension/sparse_vector/src/function/drop_sparse_vector_index.cpp
extension/sparse_vector/src/catalog/CMakeLists.txt
extension/sparse_vector/src/catalog/sparse_vector_catalog_entry.cpp
extension/sparse_vector/src/include/main/sparse_vector_extension.h
extension/sparse_vector/src/include/index/sparse_vector_index.h
extension/sparse_vector/src/include/function/create_sparse_vector_index.h
extension/sparse_vector/src/include/function/query_sparse_vector_index.h
extension/sparse_vector/src/include/function/drop_sparse_vector_index.h
extension/sparse_vector/src/include/catalog/sparse_vector_catalog_entry.h
extension/sparse_vector/test/CMakeLists.txt
extension/sparse_vector/test/sparse_vector_test.cpp

# BM42 embedder
extension/rag3weaver/src/bm42_model.rs
extension/rag3weaver/src/bm42_embedder.rs
```

### Modifiés

```
extension/extension_config.cmake        (sparse_vector dans EXTENSION_LIST + WASM/Android/Swift)
extension/CMakeLists.txt                (add_extension_if_enabled sparse_vector)
extension/rag3weaver/src/lib.rs         (pub mod bm42_model + bm42_embedder)
```
