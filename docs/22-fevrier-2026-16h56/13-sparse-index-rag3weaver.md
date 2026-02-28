# 13 — Index Sparse dans rag3weaver (V1)

## Contexte

Suite au doc 12 (extension sparse vector plan) et a la preuve que candle tourne en WASM, on decide de ne PAS creer une extension C++ separee pour les sparse vectors. A la place, l'index inverse vit directement dans rag3weaver en Rust pur.

**Pourquoi dans rag3weaver et pas une extension C++ ?**
- On ecrirait le code deux fois (C++ pour l'extension + Rust pour BM42 dans candle)
- rag3weaver orchestre deja tout le pipeline search (dense + BM25 + fusion)
- L'index inverse est simple (~150 lignes Rust), pas besoin de l'infra extension rag3db
- Si plus tard on veut l'extraire en extension, le code Rust est deja ecrit — on ajoute juste un cxx bridge

## Decisions d'architecture

### 1. SparseEmbedder = trait separe

```rust
#[async_trait]
pub trait SparseEmbedder: Send + Sync {
    async fn embed_sparse(&self, texts: &[String]) -> Result<Vec<SparseVector>, EmbedError>;
}
```

Pas une extension du trait `Embedder` existant. Raisons :
- Le trait `Embedder` est propre (3 implementations, zero ML deps)
- Un sparse embedder est une capacite differente (BM42, SPLADE, ou callback)
- Le `Catalog` tient un `Option<Arc<dyn SparseEmbedder>>` — quand `None`, pas de sparse

### 2. sparse = flag orthogonal sur KBConfig

```rust
pub struct KBConfig {
    pub search: SearchMode,     // Hybrid | Semantic | Fulltext (inchange)
    pub sparse: bool,           // NOUVEAU: activer le sparse sur ce KB
    pub sparse_weight: f64,     // poids dans la fusion weighted (default 0.2)
}
```

Pas de nouveaux variants SearchMode. Le sparse se combine avec n'importe quel mode :
- Hybrid + sparse = 3-way (dense + BM25 + sparse)
- Semantic + sparse = 2-way (dense + sparse)
- Fulltext + sparse = 2-way (BM25 + sparse)

### 3. SparseIndex en memoire, persist via colonnes DB

```
┌─────────────────────────────────────────────────────┐
│ In-Memory (Catalog)                                  │
│                                                       │
│  sparse_indexes: HashMap<kb_name, SparseIndex>       │
│    SparseIndex:                                       │
│      postings: HashMap<token_id, Vec<(uuid, weight)>>│
│      vectors:  HashMap<uuid, SparseVector>            │
└──────────────┬──────────────────────────────────────┘
               │ rebuild on initialize()
               │ refresh after drain()
┌──────────────▼──────────────────────────────────────┐
│ On-Disk (rag3db columns)                             │
│                                                       │
│  Node: Document {                                     │
│    _uuid: "abc",                                      │
│    main_embedding: [0.1, 0.2, ...],    ← dense       │
│    main_sparse_indices: [42, 156, 3001], ← sparse    │
│    main_sparse_weights: [0.8, 0.65, 0.45],           │
│  }                                                    │
└─────────────────────────────────────────────────────┘
```

Cycle de vie :
1. `initialize()` → DDL cree les colonnes → charge depuis DB → build index
2. `create()` → enqueue EmbedOp + SparseEmbedOp
3. `drain()` → EmbedProcessor stocke dense, SparseEmbedProcessor stocke sparse → rebuild index
4. `search()` → query l'index en memoire → fuse avec dense + BM25
5. `delete()` → remove uuid de l'index

## Structure de donnees : SparseVector + SparseIndex

```rust
/// Vecteur sparse : paires (token_id, poids) triees par token_id
pub struct SparseVector {
    pub indices: Vec<u32>,  // token_ids (tries pour dot product efficace)
    pub values: Vec<f32>,   // poids correspondants
}

/// Index inverse en memoire
pub struct SparseIndex {
    postings: HashMap<u32, Vec<(String, f32)>>,  // token_id → [(uuid, weight)]
    vectors: HashMap<String, SparseVector>,       // uuid → vecteur (pour delete)
}
```

**Search (dot product via posting lists) :**
```
Pour chaque (token_id, q_weight) dans le query vector :
    Pour chaque (uuid, d_weight) dans postings[token_id] :
        scores[uuid] += q_weight * d_weight
Retourner top-k par score decroissant
```

Complexite : O(Q * avg_posting_len) ou Q = nombre de tokens non-zero dans la query.

## Fusion 3-way

La fusion RRF existante (`rrf_fuse`) accepte deja N listes — pas de changement necessaire :

```rust
pub fn rrf_fuse(ranked_lists: &[&[(String, f32)]], k: f32) -> Vec<(String, f32)>
```

Pour les strategies Weighted et Boost :
- **Weighted 3-way** : `(1 - kw - sw) * vector + kw * bm25_norm + sw * sparse_norm`
- **Boost 3-way** : fallback vers RRF (boost ne s'etend pas naturellement a 3 signaux)

## Pipeline search complet (avec sparse)

```
Query "machine learning"
      │
      ├─ embed(query) → dense Vec<f32> [384d]
      ├─ embed_sparse(query) → SparseVector {indices: [42, 156], values: [0.8, 0.3]}
      │
      ├─ search_vector(dense)     → Vec<SearchResult>  [cosine sim via Cypher]
      ├─ search_bm25(text)        → Vec<SearchResult>  [BM25 via Tantivy]
      └─ search_sparse(sparse)    → Vec<SearchResult>  [dot product via index inverse]
              │
              ▼
      fuse_results(vector, bm25, sparse)
              │
              ├─ RRF: rrf_fuse([vector, bm25, sparse], k=60)
              ├─ Weighted: (1-kw-sw)*vec + kw*bm25 + sw*sparse
              └─ Boost: fallback RRF
              │
              ▼
      SearchResponse { results, meta { sparse_count, ... } }
```

## Fichiers modifies

Chemin de base : `packages/rag3db/extension/rag3weaver/src/`

| Fichier | Changement |
|---|---|
| `sparse_index.rs` | **NOUVEAU** — SparseVector + SparseIndex + tests |
| `embedder.rs` | SparseEmbedder trait, MockSparseEmbedder, CallbackSparseEmbedder |
| `config.rs` | `sparse: bool` + `sparse_weight: f64` sur KBConfig |
| `schema.rs` | Colonnes `{kb}_sparse_indices INT64[]` + `{kb}_sparse_weights DOUBLE[]` |
| `ops.rs` | SparseEmbedOp, CatalogOp::SparseEmbed, OP_SPARSE_EMBED |
| `search.rs` | search_sparse(), fuse_rrf_n(), fuse_weighted_3way(), fuse_results etendu |
| `catalog.rs` | sparse_embedder, sparse_indexes, SparseEmbedProcessor, rebuild, wiring |
| `lib.rs` | pub mod sparse_index + re-exports |

## Prochaines etapes

- **V2** : BM42 dans candle — extraction attention weights du [CLS] token, merge WordPiece sous-mots
- **V3** : Integration dans le pipeline rag3weaver en production (config yaml, E2E tests)
- **Optionnel** : Extraction en extension C++ si besoin d'API SQL native
