# rag3db

Fork de [Kuzu](https://github.com/kuzudb/kuzu) v0.11.2.2 — graph database embarquable avec full-text search (Tantivy), vector search (HNSW), sparse vector search, et un framework RAG complet (rag3weaver). Natif + WASM navigateur.

## Extensions

### tantivy_fts — Full-Text Search

Recherche full-text via [Tantivy](https://github.com/quickwit-oss/tantivy) (fork Rust, bridge cxx) :

```cypher
-- Creer un index (champs texte + filter fields optionnels)
CALL CREATE_TANTIVY_INDEX('docs', ['title', 'body'],
     filter_fields := [('category', 'STRING'), ('year', 'INT64')])

-- Substring (trigram-accelere, BM25, highlights multi-champs)
CALL QUERY_TANTIVY_INDEX('docs',
     '{"type":"contains","field":"body","value":"programming"}', 10)
RETURN node_id, score, highlights

-- Fuzzy (tolere les fautes de frappe)
CALL QUERY_TANTIVY_INDEX('docs',
     '{"type":"contains","field":"body","value":"programing","distance":1}', 10)
RETURN node_id, score, highlights

-- Regex (trigram-accelere + verification regex + BM25)
CALL QUERY_TANTIVY_INDEX('docs',
     '{"type":"contains","field":"body","value":"program[a-z]+","regex":true}', 10)
RETURN node_id, score, highlights

-- Boolean multi-champs
CALL QUERY_TANTIVY_INDEX('docs',
     '{"type":"boolean","should":[
       {"type":"contains","field":"title","value":"rust"},
       {"type":"contains","field":"body","value":"rust"}
     ]}', 10)
RETURN node_id, score, highlights

-- Filtrage par IDs
CALL QUERY_TANTIVY_INDEX('docs', '...', 10, allowed_ids := [1, 5, 12])
RETURN node_id, score, highlights

-- Supprimer l'index
CALL DROP_TANTIVY_INDEX('docs')
```

8 types de requetes : contains, fuzzy, regex, phrase, term, boolean, parse, regex+fuzzy hybride.

Highlights multi-champs : chaque champ dans une requete boolean produit ses propres byte offsets (`{"title":[[0,4]],"body":[[100,120]]}`).

Hooks DELETE/UPDATE : l'index se met a jour automatiquement sur mutations Cypher. Lazy commit (dirty flag, commit+reload une seule fois avant chaque QUERY).

### vector — HNSW Vector Search

Extension vector de Kuzu avec support DELETE et UPDATE :

```cypher
CALL CREATE_VECTOR_INDEX('docs', 'emb_idx', 'embedding', metric := 'cosine')
CALL QUERY_VECTOR_INDEX('docs', 'emb_idx', $query_embedding, 10)
RETURN node.id, node.title, distance
```

Les suppressions et mises a jour de noeuds se propagent automatiquement a l'index HNSW (batched edge cleanup).

### sparse_vector — Sparse Vector Search

Index sparse pour BM42/SPLADE :

```cypher
CALL CREATE_SPARSE_VECTOR_INDEX('docs', 'sparse_idx', 'sparse_indices', 'sparse_weights')
CALL QUERY_SPARSE_VECTOR_INDEX('docs', 'sparse_idx', [1,5,42], [0.8,0.3,0.1], 10)
RETURN node_id, score
```

## rag3weaver — Framework RAG

Framework Rust haut niveau qui orchestre les 3 extensions ci-dessus. Fournit CRUD, ingestion, chunking, embeddings, et search hybride via une API Catalog :

```rust
let catalog = Catalog::new(conn, config, embedder);
catalog.initialize().await?;

// Ingestion
catalog.create("Document", uuid, data)?;
catalog.link("WRITTEN_BY", doc_ref, author_ref)?;
catalog.drain().await?; // pipeline: chunk -> insert -> link -> embed

// Search hybride (dense + BM25 + sparse)
let response = catalog.search("main", "rust programming", &options).await?;
// -> SearchResult { uuid, score, chunk: Some(ChunkInfo { text, start_char, ... }), data }
```

Fonctionnalites :
- **Chunking** : decoupe les champs longs en chunks avec overlap, indexation chunk-level
- **Search hybride** : fusion dense+BM25+sparse (Boost, RRF, Weighted)
- **Chunk resolution** : les resultats vector/sparse/BM25 resolvent vers le parent avec `ChunkInfo`
- **BM25 multi-champs** : highlights per-field permettent de matcher les chunks aux offsets BM25
- **Filtrage** : `FilterCondition` (AND/OR/NOT) compile vers Cypher WHERE + Tantivy filter fields
- **Embedders** : CandleEmbedder (MiniLM, Multilingual-MiniLM, BGE-M3), BM42, CallbackEmbedder
- **Queue async** : operations priorisees (chunk < insert < link < embed < sparse_embed)
- **Hashsafe** : deduplication par hash de contenu
- **Events** : bus d'evenements (EntityPrepared, EmbeddingCompleted, DrainCompleted, ...)
- **WASM** : FFI complete pour navigateur (async drain/search via polling)

## Targets

| Target | Statut | Tests |
|--------|--------|-------|
| Natif (Linux x86_64) | OK | 15 GTest E2E + 1064 tests Rust (ld-tantivy) + 345 tests Rust (rag3weaver) |
| Node.js natif (NAPI) | OK | contains/fuzzy/regex/phrase/parse valides |
| WASM navigateur | OK | Playwright (FTS + vector + persistence IDBFS) |

Extensions liees statiquement en WASM : tantivy_fts, vector, sparse_vector, json, algo.

## Builds

Voir **[BUILD.md](BUILD.md)** pour le guide complet.

```bash
# Natif
mkdir -p build/release && cd build/release
cmake ../.. -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_EXTENSIONS="tantivy_fts" -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE
cmake --build . -j$(nproc)

# Tests E2E rag3weaver
cd extension/rag3weaver && bash run_e2e.sh phase0
```

## Architecture

```
rag3db (fork Kuzu v0.11.2.2)
|-- extension/tantivy_fts/           Extension C++ FTS (CREATE/QUERY/DROP)
|-- extension/tantivy/ld-tantivy/    Submodule (fork Tantivy v0.26.0)
|   +-- tantivy_fts/rust/            Crate FFI (bridge cxx)
|-- extension/vector/                Extension HNSW (+ DELETE/UPDATE)
|-- extension/sparse_vector/         Extension sparse vector
|-- extension/rag3weaver/            Framework RAG Rust
|   |-- src/                         Catalog, search, chunker, queue, embedders
|   +-- tests/                       E2E natif (e2e_search.rs)
|-- tools/wasm/                      Build + tests WASM
+-- tools/nodejs_api/                Build Node.js natif
```

Le bridge **cxx** (pas extern C) donne des structs typees Rust <-> C++, zero JSON sur le hot path.

## Provenance

- **Kuzu** : [kuzudb/kuzu](https://github.com/kuzudb/kuzu) v0.11.2.2 (MIT License, voir NOTICE)
- **Tantivy** : [quickwit-oss/tantivy](https://github.com/quickwit-oss/tantivy) v0.26.0 (MIT License)

## License

[Luciform Research Source License (LRSL) v1.2](LICENSE) — source-available, gratuit sous 100K EUR/an de revenus.
