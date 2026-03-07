# Doc 31 — Session : FLUSH_LUCIVY_INDEX + FlushFTSNode + Décomposition AggregateRecordNode

Date : 7 mars 2026

## Contexte

Deux tâches réalisées dans cette session :

1. **Finalisation de la décomposition AggregateRecordNode** (doc 30) — il restait le rewire de `build_ingestion_graph()` dans catalog.rs
2. **Nouveau : FLUSH_LUCIVY_INDEX** — après un gros drain(), les hooks Lucivy ont écrit dans le writer mais sans commit. La première recherche payait le coût du lazy flush. Solution : flush explicite via un nœud du graphe.

## E.4 — Finalisation catalog.rs (suite doc 30)

### Rewire build_ingestion_graph() ✅

Remplacé la section `if has_aggregates` (ancien `AggregateRecordNode`) par les 3 nœuds KB + 3 downstream :

```rust
// gather → update → chunk → insert/link/embed
graph.add_node(Box::new(GatherKBNode::new("gather_kb")));
graph.add_node(Box::new(UpdateKBNode::new("update_kb")));
graph.add_node(Box::new(ChunkKBNode::new("chunk_kb")));
graph.add_node(Box::new(InsertRecordNode::new("agg_inserts")));
graph.add_node(Box::new(LinkRecordNode::new("agg_links")));
graph.add_node(Box::new(EmbedRecordNode::new("agg_embeds", 32)));
```

### Commentaires mis à jour ✅

- Ligne 363 : `GatherKBNode` (était `AggregateRecordNode`)
- Lignes 870-880 : topology doc avec les 3 KB nodes

### Validation ✅

- `cargo check --lib` : compile clean
- `cargo test --lib` : 392 pass, 0 fail
- Plus aucune référence à `AggregateRecordNode` dans le codebase

## FLUSH_LUCIVY_INDEX — Extension C++

### Problème

Après ingestion, les hooks Lucivy (`insert`/`update`/`delete_`) écrivent dans le writer Tantivy mais ne commit pas. Le commit+reload n'arrive qu'au premier `QUERY_LUCIVY_INDEX` (lazy flush via `flushIfDirty()`). Sur un gros batch, la première recherche est anormalement lente.

### Solution

Nouvelle fonction Cypher `CALL FLUSH_LUCIVY_INDEX('table')` — commit + reload_reader si dirty.

### Fichiers créés/modifiés

| Fichier | Changement |
|---|---|
| `extension/lucivy_fts/src/include/function/flush_lucivy_index.h` | **Nouveau** — `FlushLucivyFunction` + `InternalFlushLucivyFunction` |
| `extension/lucivy_fts/src/function/flush_lucivy_index.cpp` | **Nouveau** — pattern identique à `drop_lucivy_index.cpp` : BindData, bindFunc, rewriteFunc, internalTableFunc |
| `extension/lucivy_fts/src/function/CMakeLists.txt` | Ajout `flush_lucivy_index.cpp` |
| `extension/lucivy_fts/src/main/lucivy_fts_extension.cpp` | `#include` + 2 lignes registration dans `load()` |

### Logique interne

```cpp
auto& lucivyIndex = nodeTable.getIndex(bd.indexName)->cast<LucivyIndex>();
lucivyIndex.flushIfDirty();  // commit() + reload_reader() si dirty_
```

### Validation C++ ✅

- `cmake --build . --target rag3db_lucivy_fts_extension` : compile clean
- `lucivy_fts_test` : **24 tests, 24 PASSED** (aucune régression)

## FlushFTSNode — Nœud dataflow

### Design

- Input : `trigger` (Empty, optional) — connecté à `update_kb.done`
- Output : `done` (Empty)
- Services : `conn` (DbConnection), `flush_kb_names` (Vec<String>)
- Execute : boucle `CALL FLUSH_LUCIVY_INDEX('{kb}_Index')` pour chaque KB touchée
- Metrics : `kb_count`, `flushed`

### Topologie finale

```
AggregateRecord[] → GatherKBNode("gather_kb")
                        └── kb_content → UpdateKBNode("update_kb")
                                            ├── kb_content → ChunkKBNode("chunk_kb")
                                            │                    ├── entities → InsertRecordNode("agg_inserts")
                                            │                    ├── relations → LinkRecordNode("agg_links")
                                            │                    └── agg_inserts.inserted → EmbedRecordNode("agg_embeds")
                                            └── done → FlushFTSNode("flush_fts")  ← EN PARALLÈLE
```

Le flush FTS tourne **en parallèle** du chunking/insert/embed — zéro temps perdu. Sémantiquement correct car l'index Lucivy est sur `{KB}_Index` (modifié par UpdateKBNode), pas sur `{KB}_Index_Chunk`.

### Fichiers modifiés (Rust)

| Fichier | Changement |
|---|---|
| `src/dataflow/record_nodes.rs` | +`FlushFTSNode` (struct + impl Node) |
| `src/dataflow/mod.rs` | Export `FlushFTSNode` |
| `src/catalog.rs` | Import FlushFTSNode, capture `flush_kb_names` avant consommation de pending, register service, wire `update_kb.done → flush_fts.trigger`, supprimé boucle post-pipeline dans drain() |

### Validation Rust ✅

- `cargo check --lib` : compile clean
- `cargo test --lib` : 392 pass, 0 fail

## Note importante

Le binding C++ de ld-lucivy (le crate Rust) doit être **rebuild** car des updates ont été faites (normalisation accents et autres). Commande :

```bash
cd packages/rag3db/extension/lucivy/ld-lucivy
cargo build --release -p ld-lucivy -p lucivy-fts
```

Puis rebuild l'extension :

```bash
cd packages/rag3db/build/release
cmake --build . --target rag3db_lucivy_fts_extension -j$(nproc)
```

Et le cmake avait un vieux cache `BUILD_EXTENSIONS=tantivy_fts` — reconfigurer avec :

```bash
cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSION_TESTS=TRUE \
  -DBUILD_EXTENSIONS="lucivy_fts;sparse_vector;vector" -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE
```

## Résumé des fichiers modifiés (toute la session)

| Fichier | Changement |
|---|---|
| `src/records.rs` | +KBContentRecord, +RecordSourceContent |
| `src/dataflow/port.rs` | +PortType::KBContent |
| `src/dataflow/record_nodes.rs` | -AggregateRecordNode, +GatherKBNode, +UpdateKBNode, +ChunkKBNode, +FlushFTSNode |
| `src/dataflow/mod.rs` | Exports mis à jour |
| `src/catalog.rs` | Rewire build_ingestion_graph() (7 nœuds KB), flush_kb_names service, drain() simplifié |
| `extension/lucivy_fts/src/include/function/flush_lucivy_index.h` | **Nouveau** |
| `extension/lucivy_fts/src/function/flush_lucivy_index.cpp` | **Nouveau** |
| `extension/lucivy_fts/src/function/CMakeLists.txt` | +flush_lucivy_index.cpp |
| `extension/lucivy_fts/src/main/lucivy_fts_extension.cpp` | +include +registration |

## État

- **Rust** : ✅ 392 pass, 0 fail
- **C++ extension** : ✅ 24 GTests pass
- **Rebuild ld-lucivy** : ❌ À faire (normalisation accents + autres updates)
- **E2E** : ❌ À valider après rebuild complet
