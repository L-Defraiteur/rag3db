# Doc 03 — Architecture rag3weaver

Date : 15 mars 2026

## Vue d'ensemble

rag3weaver est un framework RAG (Retrieval-Augmented Generation) construit comme extension Rust de rag3db (graph DB). Il orchestre l'ingestion, le chunking, l'embedding et la recherche hybride 3-voies (BM25 + vector + sparse) via un DAG de dataflow typé avec checkpoint/undo.

```
┌─────────────────────────────────────────────────────────────────────┐
│                          rag3weaver                                  │
│                                                                     │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌────────────────┐  │
│  │  Catalog  │──▸│  Drain   │──▸│ Dataflow │──▸│   Indexes      │  │
│  │  (API)    │   │ (Queue)  │   │  (DAG)   │   │ FTS+Vec+Sparse │  │
│  └──────────┘   └──────────┘   └──────────┘   └────────────────┘  │
│       │                                              │              │
│       │              ┌───────────────────┐           │              │
│       └─────────────▸│  Search (Hybrid)  │◂──────────┘              │
│                      │  RRF / Weighted   │                          │
│                      └───────────────────┘                          │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                      rag3db (graph DB)                        │   │
│  │  Entity tables │ Chunk tables │ Relation tables │ _catalog_meta│  │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌────────────┐  ┌────────────────┐  ┌──────────────────────────┐  │
│  │  lucivy    │  │ sparse_vector  │  │  HNSW (rag3db builtin)   │  │
│  │  (FTS)     │  │ (WAND mmap)   │  │  (vector index)          │  │
│  └────────────┘  └────────────────┘  └──────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘

Targets : Native (Node.js, CLI) │ WASM (navigateur)
```

## Data Model

### Entities & Knowledge Bases

```
                    ┌─────────────────┐
                    │   CatalogConfig  │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
       ┌────────────┐ ┌──────────┐  ┌──────────┐
       │  EntityDef  │ │ KBConfig │  │ Relation │
       │  "Document" │ │  "main"  │  │  "REFS"  │
       └──────┬─────┘ └─────┬────┘  └──────────┘
              │              │
              │    fields:   │    signals: BM25|Vector|Sparse
              │    title_for │    fusion: RRF (k=60)
              │    content_for    chunking: Semantic (1500/200)
              │              │
              ▼              ▼
    ┌──────────────┐  ┌──────────────────┐
    │  Document    │  │  main_Index      │  (KB aggregated view)
    │  _uuid       │  │  _title          │
    │  title       │  │  _content        │
    │  body        │  │  _source_entity  │
    │  author      │  │  _source_uuid    │
    └──────┬───────┘  └────────┬─────────┘
           │                   │
           ▼                   ▼
    ┌──────────────┐  ┌──────────────────┐
    │  Doc_Chunk   │  │  main_Index_Chunk│
    │  _text       │  │  _text           │
    │  embedding[] │  │  embedding[]     │
    │  _embed_hash │  │  _embed_hash     │
    └──────────────┘  └──────────────────┘
```

### Simple Entity vs KB

| Aspect | Simple Entity | Knowledge Base |
|--------|--------------|----------------|
| Tables | `{Entity}` + `{Entity}_Chunk` | `{KB}_Index` + `{KB}_Index_Chunk` |
| Source | Un seul type d'entité | Agrège plusieurs entités |
| FTS | Sur l'entité directe | Sur `_title` + `_content` agrégés |
| Vector | Sur les chunks | Sur les chunks |
| Config | `is_content`/`is_title` sur les fields | `content_for`/`title_for` → KB name |

## Pipeline d'ingestion

```
 create("Document", {title, body})        Synchrone — enqueue
          │
          ▼
    ┌──────────┐
    │ Pending   │  inserts / updates / deletes / links
    │ Queue     │
    └─────┬────┘
          │
     drain()                               Async — exécute le DAG
          │
          ▼
 ┌──────────────────────────────────────────────────────────┐
 │                    Dataflow DAG                           │
 │                                                          │
 │  InsertRecordNode ──▸ ChunkRecordNode ──▸ EmbedNode     │
 │       │                                      │           │
 │       │              ┌───────────────────────┘           │
 │       ▼              ▼                                   │
 │  LinkRecordNode   FlushNode (FTS)                        │
 │                   SparseCommitNode                       │
 │                                                          │
 │  Pour les KBs :                                          │
 │  KBGatherNode ──▸ KBUpdateNode ──▸ KBChunkNode          │
 │                                       │                  │
 │                                       ▼                  │
 │                                   KBEmbedNode            │
 │                                   (dense+sparse)         │
 └──────────────────────────────────────────────────────────┘
```

### Nodes du Dataflow

| Node | Rôle | Undo |
|------|------|------|
| **InsertRecordNode** | MERGE entités sur `_uuid`, cache node IDs | DELETE inserted |
| **ChunkRecordNode** | Chunking parallèle (Semantic/Fixed/Sentence/Markdown) | — |
| **EmbedNode** | Embeddings dense, skip si `_embed_hash` inchangé, insert sparse | — |
| **KBEmbedNode** | Embeddings dense+sparse pour KBs (dual ou séparé) | — |
| **LinkRecordNode** | MERGE relations entre entités | DELETE relations |
| **FlushNode** | `FLUSH_LUCIVY_INDEX` — commit FTS + reload reader | re-flush |
| **SparseCommitNode** | `handle.commit_inner()` — persist sparse index | re-commit |
| **KBGatherNode** | Détecte les changements de contenu pour le KB | — |
| **KBUpdateNode** | Met à jour `{KB}_Index`, supprime les chunks stale | — |
| **KBChunkNode** | Chunk le contenu agrégé du KB | — |
| **DeleteRecordNode** | CASCADE delete entité + chunks | — |
| **UpdateRecordNode** | Update fields, re-chunk si contenu changé | — |

### Checkpoint & Crash-Recovery

```
 Node 1 ──▸ ✓ checkpoint ──▸ Node 2 ──▸ ✓ checkpoint ──▸ Node 3 ──▸ ✗ FAIL
                                                              │
                                                              ▼
                                                     undo(Node 3)
                                                     undo(Node 2)
                                                     undo(Node 1)
```

- Après chaque node : sérialise inputs/outputs/undo_context dans `_dataflow_checkpoint`
- Sur failure : rollback en ordre inverse via `node.undo(ctx, undo_data)`
- Au restart : reprend depuis le dernier checkpoint complet

## Recherche hybride

```
 catalog.search("main", "Rust safety", options)
          │
          ├──▸ embed("Rust safety") ──▸ dense_vec[1024]
          │                           ──▸ sparse_vec{indices, weights}
          │
          ├──▸ Vector Search (HNSW)
          │    QUERY_VECTOR_INDEX(chunk_table, embedding, limit)
          │    → chunk_uuids + cosine_distance
          │
          ├──▸ BM25 Search (lucivy FTS)
          │    QUERY_LUCIVY_INDEX(table, fields, query, mode)
          │    → parent_uuids + bm25_score + highlights
          │    Modes: Contains | ContainsSplit | Parse | Regex
          │
          ├──▸ Sparse Search (WAND)
          │    SparseHandle::search(sparse_vec, limit)
          │    → chunk_offsets + scores → resolve via Cypher
          │
          ▼
    ┌─────────────┐
    │   Fusion    │  RRF: score = Σ(1/(k+rank))
    │             │  Weighted: score = Σ(w × norm_score)
    └──────┬──────┘
           │
           ▼
    ┌─────────────┐
    │  Chunk      │  Group chunks by parent entity
    │  Resolution │  Keep best chunk per parent
    └──────┬──────┘
           │
           ▼
    ┌─────────────┐
    │  Enrich     │  Fetch entity data (title, body, etc.)
    └──────┬──────┘
           │
           ▼
    SearchResponse { results, meta }
```

### Signaux de recherche

```
SearchSignals (bitmask u8)
  0x1 = BM25      (fulltext)
  0x2 = Vector    (semantic)
  0x4 = Sparse    (BM25-like via sparse embeddings)

Presets:
  FULLTEXT = 0x1        BM25 seul
  SEMANTIC = 0x2        Vector seul
  HYBRID   = 0x3        BM25 + Vector
  HYBRID|SPARSE = 0x7   3-way
```

## Index Stack

```
┌──────────────────────────────────────────────────────┐
│                 Index Layer                            │
│                                                       │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │   lucivy     │  │sparse_vector │  │   HNSW     │ │
│  │   (FTS)      │  │  (WAND)      │  │  (rag3db)  │ │
│  │              │  │              │  │            │ │
│  │  LucivyHandle│  │ SparseHandle │  │  builtin   │ │
│  │  StdFsDir    │  │ BlobStore    │  │  C++       │ │
│  │  writer lock │  │ mmap flat    │  │            │ │
│  └──────┬───────┘  └──────┬───────┘  └────────────┘ │
│         │                 │                           │
│         ▼                 ▼                           │
│    Filesystem        CypherBlobStore                  │
│    lucivy_indexes/   _index_blobs table               │
│    {table}/          (rag3db graph DB)                 │
│                                                       │
│    MemBlobStore (fallback in-memory pour tests)       │
└──────────────────────────────────────────────────────┘
```

## Targets

```
┌────────────────────────────┐     ┌────────────────────────────┐
│         Native             │     │          WASM              │
│                            │     │                            │
│  Rag3dbConnection          │     │  CallbackConnection        │
│  (Arc<Database>)           │     │  (JS → Cypher callback)    │
│                            │     │                            │
│  CandleEmbedder (CUDA)    │     │  CallbackEmbedder          │
│  BGE-M3 (dense+sparse)    │     │  (JS → embedding callback) │
│                            │     │                            │
│  StdFsDirectory            │     │  MemBlobStore              │
│  CypherBlobStore           │     │  (pas de filesystem)       │
│                            │     │                            │
│  Feature flags:            │     │  Feature flags:            │
│  rag3db-native             │     │  wasm-emscripten           │
│  candle-embedder           │     │  candle-wasm               │
│  cuda                      │     │                            │
└────────────────────────────┘     └────────────────────────────┘
```

## Où on va ensuite

### Court terme
1. **Migration FTS vers Rust** — même pattern que sparse : `LucivyHandle` direct depuis rag3weaver, plus de hooks C++ NodeTable
2. **Fix BM25 Contains** — les champs `._ngram`/`._raw` ne sont pas alimentés par l'extension C++, la migration FTS règle ça
3. **Cleanup colonnes orphelines** — `ALTER TABLE DROP sparse_indices/sparse_weights` pour les DBs existantes

### Moyen terme
4. **Détection embedding_dim mismatch** — erreur si la config change de dimension sans rebuild
5. **Sharding lucivy** — quand l'instance ld-lucivy aura fini les optis (suffix FST, sharding)
6. **Multi-backend** — abstraction DB pour supporter d'autres backends que rag3db

### Long terme
7. **Démo WASM** — RAG complet dans le navigateur (rag3db WASM + lucivy statiquement linké + sparse in-memory)
8. **Cloud hosting** — DGX Spark, multi-tenant, API REST
9. **Migrations destructives** — supporter la suppression de champs, changement de type
