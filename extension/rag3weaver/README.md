# rag3weaver

RAG pipeline orchestrator for [rag3db](../../README.md). Handles document ingestion, chunking, embedding, and hybrid search — all in Rust, with native and WASM targets.

## Overview

rag3weaver provides a high-level `Catalog` API that coordinates:

- **Ingestion**: `create()` / `link()` / `update()` / `delete()` with automatic chunking and embedding
- **Search**: hybrid fusion of BM25 + dense vector (HNSW) + sparse vector signals
- **Queue**: priority-based async pipeline (chunk → insert → link → embed)
- **Embedders**: pluggable via traits — bring your own model or use the built-in ones

```rust
let mut catalog = Catalog::new(conn, embedder, config);
catalog.set_dual_embedder(bge_m3.clone());
catalog.initialize().await?;

// Ingest documents
let doc_ref = catalog.create("Document", data)?;
catalog.link("WRITTEN_BY", doc_ref, author_ref, props)?;
catalog.drain().await?;

// Hybrid search
let response = catalog.search("main", "rust ownership model", &SearchOptions {
    bm25_mode: BM25Mode::ContainsSplit,
    consistency: Consistency::Immediate,
    ..Default::default()
}).await?;

for result in &response.results {
    println!("{} (score={:.4})", result.uuid, result.score);
    if let Some(chunk) = &result.chunk {
        println!("  chunk: {}..{}", chunk.start_char, chunk.end_char);
    }
}
```

## Search Signals

Three independent search signals, combinable via bitflags:

```rust
SearchSignals::BM25                              // Full-text only
SearchSignals::VECTOR                            // Dense vector only
SearchSignals::SPARSE                            // Sparse lexical only
SearchSignals::BM25 | SearchSignals::VECTOR      // Classic hybrid
SearchSignals::VECTOR | SearchSignals::SPARSE     // Dense + sparse
SearchSignals::BM25 | SearchSignals::VECTOR | SearchSignals::SPARSE  // All three
```

### BM25 Full-Text

Powered by rag3db's Lucivy extension. 4 query modes:

| Mode | Behavior |
|------|----------|
| `Contains` | Trigram-accelerated substring, fuzzy-tolerant |
| `ContainsSplit` | Auto-splits multi-word queries with boolean OR |
| `Regex` | Trigram-accelerated regex matching |
| `Parse` | Native Lucivy QueryParser (standard BM25) |

Multi-field highlights with per-field byte offsets. Chunk-level resolution maps BM25 hits to the correct chunk via `ChunkInfo`.

### Dense Vector (HNSW)

Cosine similarity via rag3db's vector extension. L2-normalized embeddings, filtered search support.

### Sparse Vector

Token-weight dot-product via rag3db's sparse_vector extension. Compatible with BM42, SPLADE, and BGE-M3 learned sparse.

### Fusion

Results from active signals are fused into a single ranked list:

```rust
FusionConfig {
    strategy: FusionStrategy::Rrf,  // or Weighted
    rrf_k: 60.0,
    bm25:   SignalConfig { weight: 1.0, role: SignalRole::Fuse, .. },
    vector: SignalConfig { weight: 1.0, role: SignalRole::Fuse, .. },
    sparse: SignalConfig { weight: 0.5, role: SignalRole::Boost, boost_type: BoostType::Multiplicative, .. },
}
```

- **Fuse**: signal contributes candidates to the merged list
- **Boost**: signal multiplies/adds to scores of existing candidates
- **Normalize**: MinMax or None per signal before fusion

## Embedding Models

### Built-in (via candle, feature-gated)

| Model | Dims | Size | Languages | Feature |
|-------|------|------|-----------|---------|
| all-MiniLM-L6-v2 | 384 | ~23MB | EN | `candle-embedder` |
| bge-base-en-v1.5 | 768 | ~110MB | EN | `candle-embedder` |
| paraphrase-multilingual-MiniLM-L12-v2 | 384 | ~471MB | 50+ | `candle-embedder` |
| BGE-M3 (XLM-RoBERTa) | 1024 | ~2.2GB | 100+ | `bge-m3` |

### BM42 Sparse

CLS attention weights extracted from any BERT-family model. Works with MiniLM, Multilingual-MiniLM, or any model loaded via `Bm42Model`.

### Custom (via traits)

Zero ML dependencies in the core library. Plug in any embedding provider:

```rust
// Dense
let embedder = CallbackEmbedder::new(384, |texts| {
    // Call OpenAI, Cohere, local model, etc.
    Ok(my_embed(texts))
});

// Sparse
let sparse = CallbackSparseEmbedder::new(|texts| {
    Ok(my_sparse_embed(texts))
});

// Dual (dense + sparse in one call)
let dual = CallbackDualEmbedder::new(384, |texts| {
    let (dense, sparse) = my_dual_embed(texts);
    Ok((dense, sparse))
});
```

## DualEmbedder — Single Forward Pass

When both dense and sparse embeddings are needed, `DualEmbedder` computes both in a **single GPU forward pass** instead of two:

```
Without DualEmbedder:                  With DualEmbedder:
  forward_pass() → dense (CLS)          forward_pass() → hidden_states
  forward_pass() → sparse (attention)     ├─ extract_dense() → dense
  = 2 forward passes                      └─ extract_sparse() → sparse
                                         = 1 forward pass
```

The `DualEmbedProcessor` receives mega-batches (~500 items) from the queue, subdivides into GPU mini-batches of 32 internally, then writes all results in a single UNWIND transaction per column. Timing events (`GpuBatchCompleted`, `DbWriteCompleted`) are emitted for observability.

**Measured gain**: ~55% faster embedding on BGE-M3 (146ms vs 319ms for 3 documents with chunks).

This optimization also applies at **search time**: when a query needs both dense and sparse vectors, the dual embedder computes both in one pass.

## Chunking

Documents are automatically split into chunks at ingestion time:

```rust
ChunkingConfig {
    max_size: 1500,    // max chars per chunk
    overlap: 200,      // overlap between adjacent chunks
    strategy: ChunkStrategy::Markdown, // or Semantic, Sentence, Fixed
}
```

Each chunk tracks **core** (non-overlapping) and **full** (with overlap) byte/line offsets. Core offsets enable precise BM25 highlight-to-chunk matching.

Chunks are stored as separate graph nodes (`Document_Chunk`) linked to their parent via `Document_HAS_CHUNK` relations. Search results include `ChunkInfo` with text, offsets, and parent data.

## Operation Queue

Priority-based async pipeline with state machine:

```
Priority 0: Chunk    (batch=10000)  — split documents into chunks
Priority 1: Insert   (batch=50)    — create entity nodes (UNWIND)
Priority 2: Link     (batch=50)    — create relations (UNWIND)
Priority 3: Embed    (batch=32)    — dense embeddings + UNWIND SET
Priority 3: DualEmbed(batch=500)   — dense+sparse in one pass, 32-item GPU sub-batches
```

Processors can inject downstream operations: `ChunkProcessor` at priority 0 emits Insert/Link/Embed ops that are processed in subsequent priority rounds.

Event bus (`async_broadcast`) emits `QueueEvent`s for monitoring:
- `Enqueued`, `ProcessingBatch`, `BatchCompleted`, `BatchFailed`
- `GpuBatchCompleted` (per GPU mini-batch timing)
- `DbWriteCompleted` (per UNWIND timing)

## Feature Flags

| Feature | Description |
|---------|-------------|
| `candle-embedder` | Local embeddings via candle (MiniLM, BgeBase, MultilingualMiniLM) |
| `candle-wasm` | Candle for WASM (CPU-only, no CUDA) |
| `bge-m3` | BGE-M3 dual embedder (native only, ~2.2GB) |
| `cuda` | GPU acceleration for candle models |
| `rag3db-native` | Native rag3db connection |
| `wasm-emscripten` | WASM emscripten FFI bindings |

## Tests

```bash
# Unit tests (350+)
cargo test --lib

# E2E tests (requires rag3db native build)
./run_e2e.sh phase0          # CRUD (6 tests)
./run_e2e.sh phase1 phase2   # BM25 + Vector (10 tests)
./run_e2e.sh phase3          # Sparse hybrid (4 tests)
./run_e2e.sh phase4          # Per-signal combinations (7 tests)
./run_e2e.sh phase5          # DualEmbedder path (3 tests)
```

## Architecture

```
src/
├── catalog.rs          Catalog API: CRUD + search + processors
├── search.rs           Search functions: BM25, vector, sparse, fusion
├── queue.rs            Operation queue with priority dispatch
├── embedder.rs         Embedder/SparseEmbedder/DualEmbedder traits
├── bge_m3_embedder.rs  BGE-M3: 1024d dense + learned sparse
├── candle_embedder.rs  CandleEmbedder (MiniLM, Bge, Multilingual) + CandleDualEmbedder
├── bm42_embedder.rs    BM42: CLS attention sparse vectors
├── bm42_model.rs       Modified BERT returning hidden_states + attn_probs
├── chunker.rs          Semantic/Markdown/Sentence/Fixed chunking
├── fusion.rs           Score fusion: RRF, Weighted, Boost
├── config.rs           Schema: EntityDef, FieldType, KBConfig
├── schema.rs           Cypher DDL generation
├── filter.rs           Filter compiler: conditions → Cypher WHERE
├── ops.rs              Operation types: Insert, Link, Embed, DualEmbed, Chunk
├── refs.rs             Async-awaitable EntityRef/RelationRef
├── connection.rs       DbConnection trait + CypherValue
├── events.rs           EventBus (async_broadcast)
├── sparse_index.rs     SparseVector: parallel indices/values
├── wasm_ffi.rs         WASM emscripten FFI
└── rag3db_connection.rs  Native rag3db connection
```

## License

[Luciform Research Source License (LRSL) v1.2](../../../../LICENSE)
