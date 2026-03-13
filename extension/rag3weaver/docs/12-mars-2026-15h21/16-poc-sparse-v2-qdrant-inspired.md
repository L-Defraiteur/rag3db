# Doc 16 — POC Sparse V2 : moteur inspiré Qdrant

Date : 12 mars 2026

Réf : doc 15 (analyse sparse), Qdrant `lib/sparse/` (Apache 2.0)

## 1. Objectif

Remplacer le moteur interne de notre crate `sparse-vector` (HashMap naïf, ~150 lignes) par une implémentation inspirée de Qdrant (posting lists triées, batch scoring, WAND pruning, compression bitpacking).

**Ce qui change** : `index.rs` (moteur) + nouveaux fichiers internes
**Ce qui ne change PAS** : `bridge.rs` (cxx), `handle.rs` (persistance bincode), l'extension C++, les appels Cypher

## 2. Source : Qdrant `lib/sparse/` (Apache 2.0)

Fichiers copiés/adaptés depuis https://github.com/qdrant/qdrant (commit shallow clone 12 mars 2026) :

| Fichier Qdrant | → Notre fichier | Lignes | Adaptations |
|----------------|-----------------|--------|-------------|
| `index/posting_list_common.rs` | `posting_list_common.rs` | ~90 | `PointOffsetType` → `u32` |
| `index/posting_list.rs` | `posting_list.rs` | ~300 | idem |
| `index/compressed_posting_list.rs` | `compressed_posting_list.rs` | ~680 | Virer `HardwareCounterCell`, `PointOffsetType` → `u32` |
| `index/search_context.rs` | `search_context.rs` | ~420 | Virer `HardwareCounterCell`, `AtomicBool`, remplacer `TopK` par `BinaryHeap` |
| `common/types.rs` | `weight.rs` | ~160 | Renommer, garder Weight trait (f32, f16, QuantizedU8) |
| `common/scores_memory_pool.rs` | `scores_memory_pool.rs` | ~60 | `POOL_KEEP_LIMIT` = 16 hardcodé |
| `common/sparse_vector.rs` | On garde le nôtre | — | On ajoute juste `RemappedSparseVector` + `score_vectors()` |

**Total** : ~1700 lignes copiées, ~200 lignes d'adaptations (suppression deps Qdrant).

### Notice copyright

Chaque fichier copié porte en tête :
```rust
// Based on Qdrant sparse index (https://github.com/qdrant/qdrant)
// Copyright 2021-2026 Qdrant Team <info@qdrant.tech>
// Licensed under Apache License 2.0
// Modified for rag3db sparse-vector extension
```

## 3. Adaptations des types Qdrant → nôtres

| Type Qdrant | Remplacement |
|---|---|
| `common::types::PointOffsetType` | `pub type PointOffsetType = u32;` (dans notre code) |
| `common::types::ScoreType` | `f32` |
| `common::top_k::TopK` | `TopK` maison (~30 lignes, `BinaryHeap<Reverse<ScoredPoint>>`) |
| `common::defaults::POOL_KEEP_LIMIT` | `const POOL_KEEP_LIMIT: usize = 16;` |
| `common::counter::HardwareCounterCell` | Supprimé (monitoring Qdrant-spécifique) |
| `common::mmap::*` | Pas utilisé dans le POC (phase 2) |
| `gridstore::Blob` | Supprimé |
| `validator::Validate` | Supprimé |
| `schemars::JsonSchema` | Supprimé |
| `AtomicBool` (is_stopped) | Supprimé pour le POC (pas de cancellation) |

## 4. Architecture résultante

```
sparse_vector/rust/src/
├── lib.rs                      # exports
├── bridge.rs                   # cxx bridge (INCHANGÉ)
├── handle.rs                   # SparseHandle, bincode persist (INCHANGÉ pour le POC)
├── index.rs                    # SparseIndex V2 : Vec<PostingList> + dimension remapping
├── posting_list_common.rs      # Types : PostingElement, PostingElementEx, PostingListIter trait
├── posting_list.rs             # PostingList mutable : sorted Vec + max_next_weight + upsert/delete
├── compressed_posting_list.rs  # CompressedPostingList : bitpacking + weight quantization (phase 2)
├── search_context.rs           # SearchContext : batch scoring + WAND pruning
├── weight.rs                   # Weight trait : f32, f16, QuantizedU8
├── scores_memory_pool.rs       # PooledScores : buffer reuse entre searches
└── top_k.rs                    # TopK : BinaryHeap wrapper
```

## 5. Plan d'implémentation

### Phase 1 : Posting lists triées + batch scoring (~aujourd'hui)

1. Copier les fichiers Qdrant, adapter les types
2. Réécrire `SparseIndex` :
   - `postings: Vec<PostingList>` au lieu de `HashMap<u32, Vec<(u64, f32)>>`
   - Dimension remapping : `dim_map: HashMap<u32, u32>` (token_id global → index dense)
   - `vectors: HashMap<u64, SparseVector>` reste (pour delete/update)
3. `search()` utilise `SearchContext` (batch scoring + pruning)
4. API externe identique : `insert()`, `remove()`, `search()`, `search_filtered()`
5. `cargo test` passe — mêmes tests qu'avant + nouveaux benchmarks

**Gain attendu** : 3-5x plus rapide en search (pruning + batch scoring + moins d'allocations)

### Phase 2 : Compression + mmap (futur)

- Conversion `PostingList` → `CompressedPostingList` à la persistance
- Format binaire custom mmappé au lieu de bincode
- `open()` = mmap, quasi instantané
- `commit()` = écriture incrémentale

**Gain attendu** : open/commit O(1) au lieu de O(N), RAM = working set seulement

### Phase 3 : Weight quantization (optionnel)

- f32 → f16 (÷2 mémoire, perte négligeable)
- f32 → u8 (÷4 mémoire, perte mesurable mais acceptable pour le sparse)

## 6. Dépendances ajoutées au Cargo.toml

```toml
bitpacking = "0.9"       # SIMD bitpacking pour CompressedPostingList
half = "2"                # f16 weight quantization
ordered-float = "4"       # OrderedFloat pour max_next_weight
parking_lot = "0.12"      # Mutex pour ScoresMemoryPool
```

Note : `bitpacking` et `compressed_posting_list.rs` ne sont pas strictement nécessaires pour la phase 1. On peut les ajouter en phase 2.

## 7. Ce qu'on ne fait PAS dans le POC

- Pas de mmap (reste bincode)
- Pas de compression (posting lists non compressées)
- Pas de changement au bridge cxx
- Pas de changement à l'extension C++
- Pas de changement aux appels Cypher dans rag3weaver
- Pas de `AtomicBool` cancellation (pas de queries longues pour l'instant)

## 8. Validation

```bash
# Tests unitaires sparse-vector
cd packages/rag3db/extension/sparse_vector/rust
cargo test

# Tests E2E rag3weaver (vérifie que le bridge cxx fonctionne toujours)
cd packages/rag3db/extension/rag3weaver
./run_e2e.sh --test e2e_idempotent_registration --summary
```

## 9. Mapping doc_id : u64 → u32

Notre index actuel utilise `u64` pour les node_ids (offsets Kuzu internes). Qdrant utilise `u32` (`PointOffsetType`).

**Question** : est-ce que les offsets Kuzu dépassent u32 (4 milliards) ? Probablement pas pour nos volumes. Mais pour être safe, on garde `u64` dans notre API externe (`insert(u64, ...)`, `search() → Vec<(u64, f32)>`) et on convertit en interne si nécessaire, ou on typedef `PointOffsetType = u64` dans notre version.

**Décision** : utiliser `u64` partout pour l'instant. Adapter les types Qdrant en conséquence. L'impact sur la compression bitpacking est minime (les deltas entre IDs triés restent petits).
