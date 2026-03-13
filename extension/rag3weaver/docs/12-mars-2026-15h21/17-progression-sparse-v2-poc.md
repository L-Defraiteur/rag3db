# Doc 17 — Progression : Sparse V2 POC (Qdrant-inspired)

Date : 13 mars 2026

Réf : doc 15 (analyse sparse), doc 16 (plan POC)

## Contexte

Objectif : remplacer le moteur sparse naïf (HashMap, bincode full load/save) par une implémentation inspirée de Qdrant (Apache 2.0). Deux phases :

1. **Phase 1** : Posting lists triées + batch scoring + WAND pruning (FAIT)
2. **Phase 2** : Compressed posting lists + mmap persistence (EN COURS)

## Phase 1 — FAIT ✅

### Ce qui a été implémenté

Crate `sparse-vector` v0.2.0 (`packages/rag3db/extension/sparse_vector/rust/src/`)

| Fichier | Lignes | Origine | Rôle |
|---------|--------|---------|------|
| `posting_list_common.rs` | 80 | Adapté Qdrant | Types : `PostingElementEx` (record_id u64, weight f32, max_next_weight f32), trait `PostingListIter` |
| `posting_list.rs` | 230 | Adapté Qdrant | `PostingList` : Vec trié + binary search + upsert/delete + propagation `max_next_weight` + `PostingBuilder` |
| `search_context.rs` | 250 | Adapté Qdrant | `SearchContext` : batch scoring par tranches de 10k IDs + WAND pruning sur la plus longue posting list |
| `top_k.rs` | 100 | Écrit maison | `TopK` : BinaryHeap min-heap, remplace `common::top_k::TopK` de Qdrant |
| `scores_memory_pool.rs` | 60 | Adapté Qdrant | `ScoresMemoryPool` : pool de buffers réutilisés entre searches |
| `index.rs` | 280 | Réécrit | `SparseIndex` V2 : dimension remapping (HashMap<u32,usize> → Vec<PostingList>), SearchContext pour search |

### Adaptations Qdrant → nôtres

| Type Qdrant | Notre remplacement |
|---|---|
| `PointOffsetType` (u32) | `u64` (offsets Kuzu) |
| `common::types::ScoreType` | `f32` |
| `common::top_k::TopK` | `top_k::TopK` (BinaryHeap maison) |
| `common::defaults::POOL_KEEP_LIMIT` | `const POOL_KEEP_LIMIT: usize = 16` |
| `HardwareCounterCell` | Supprimé (monitoring Qdrant-spécifique) |
| `AtomicBool` is_stopped | Supprimé (pas de cancellation pour l'instant) |

### Dépendances ajoutées

```toml
ordered-float = "4"    # pour max_next_weight comparaison
parking_lot = "0.12"   # Mutex pour ScoresMemoryPool
```

### Tests

```
running 23 tests
test index::tests::dimension_remapping ... ok
test index::tests::index_clear ... ok
test index::tests::index_empty_search ... ok
test index::tests::index_insert_and_search ... ok
test index::tests::index_insert_replaces ... ok
test index::tests::index_remove ... ok
test index::tests::index_remove_cleans_postings ... ok
test index::tests::index_search_disjoint ... ok
test index::tests::index_search_limit ... ok
test index::tests::many_documents_search ... ok
test index::tests::persistence_compat ... ok
test index::tests::search_filtered_basic ... ok
test index::tests::sparse_vector_basics ... ok
test index::tests::sparse_vector_mismatched_lengths - should panic ... ok
test handle::tests::create_writes_empty_index ... ok
test handle::tests::persistence_roundtrip ... ok
test posting_list::tests::test_delete ... ok
test posting_list::tests::test_for_each_till_id ... ok
test posting_list::tests::test_posting_operations ... ok
test posting_list::tests::test_upsert_insert_last ... ok
test posting_list::tests::test_upsert_update_weight ... ok
test top_k::tests::basic_top_k ... ok
test top_k::tests::threshold ... ok

test result: ok. 23 passed; 0 failed
```

### Ce qui n'a PAS changé

- `bridge.rs` (cxx) : inchangé, l'API C++ reste identique
- `handle.rs` : inchangé, persistance toujours bincode (sera remplacée en phase 2)
- Extension C++ (`sparse_vector/src/`) : aucun changement
- Appels Cypher dans rag3weaver : aucun changement

### Gains attendus (phase 1 seule)

- **Search** : 3-5x plus rapide grâce au batch scoring (buffer poolé, pas de HashMap alloc à chaque search) + WAND pruning (skip des posting lists à faible contribution)
- **Insert/Delete** : légèrement plus lent (binary search + propagation max_next_weight vs. HashMap append) mais négligeable
- **Mémoire** : similaire (Vec<PostingElementEx> au lieu de Vec<(u64,f32)>, +4 bytes/element pour max_next_weight)

## Phase 2 — EN COURS (mmap persistence)

### Problème restant

`handle.rs` utilise toujours bincode full serialize/deserialize :
- `open()` = lire tout `sparse.bin` + deserialize → O(N)
- `commit()` = serialize tout + écrire → O(N)

À 100k docs : ~500ms chacun. À 1M docs : ~5s chacun.

### Plan mmap

Le format Qdrant est trop couplé à BitPacker4x (u32 only, blocs de 128, dépendance SIMD). On fait un format plus simple :

**Format flat binary mmappé** :

```
sparse.mmap:
  [FileHeader]                         # 16 bytes
  [DimHeader × num_dims]               # 16 bytes × N
  [PostingEntry × total_entries]       # 16 bytes × M

FileHeader:
  magic: u32 = 0x53505253 ("SPRS")
  version: u32 = 1
  num_dims: u32
  num_vectors: u32

DimHeader (per remapped dimension):
  offset: u64     # byte offset dans le fichier vers le premier PostingEntry
  count: u32      # nombre d'entries dans la posting list
  _pad: u32

PostingEntry (trié par record_id):
  record_id: u64
  weight: f32
  max_next_weight: f32
```

**Fichier séparé pour les vectors** (nécessaires pour delete/update) :

```
sparse_vectors.bin:
  bincode serialization de HashMap<u64, SparseVector>
  (plus petit que l'index complet, et utilisé seulement pour delete/update, pas pour search)
```

**Fichier pour le dimension mapping** :

```
sparse_dims.bin:
  bincode serialization de (HashMap<u32,usize>, Vec<u32>)
```

**Comportement** :
- `open()` = mmap `sparse.mmap` (O(1)), deserialize `sparse_vectors.bin` + `sparse_dims.bin`
- `search()` = itère directement sur les pages mmap'd (OS page cache)
- `insert/remove()` = mutate la copie RAM (PostingLists en mémoire), flag dirty
- `commit()` = écrire les 3 fichiers

**Gain principal** : le search n'a plus besoin de charger toutes les posting lists en RAM. L'OS charge seulement les pages touchées par la query.

**Limitation** : `sparse_vectors.bin` reste full load/save. Mais c'est plus petit que l'index (pas de duplication posting lists) et seulement nécessaire pour delete/update, pas pour search.

### Alternatives considérées et rejetées

- **BitPacker4x** : nécessite u32 record IDs, blocs de 128 éléments. Nos IDs sont u64. Adaptation possible mais effort disproportionné pour le gain vs. mmap simple.
- **Stocker les posting lists dans Kuzu** (Option B doc 15) : toujours viable comme alternative, mais ne donne pas le même niveau de perf que mmap direct pour le search.
- **RocksDB/LMDB** : dépendance lourde, pas WASM.

### Prochaines étapes

1. Implémenter `mmap_index.rs` : `MmapPostingListIterator` qui lit directement depuis les bytes mmap'd
2. Mettre à jour `handle.rs` : écriture format flat binary + mmap pour open
3. Tests : persistence roundtrip, search sur données mmap'd
4. Éventuellement : lazy loading des vectors (ne charger `sparse_vectors.bin` que si on fait un delete/update)

## Archi cloud — ce qu'on a appris

### Sparse est le seul signal qui nécessite une lib Rust portable

| Signal | rag3db (embedded) | Qdrant | Postgres/Supabase |
|--------|-------------------|--------|-------------------|
| Vector | extension C++ | natif Qdrant | pgvector natif |
| BM25 | lucivy (Rust+cxx) | N/A | tsvector/ParadeDB |
| Sparse | **notre lib Rust + cxx** | natif Qdrant | **notre lib Rust directe** ou posting tables SQL |

### Qdrant licence

Apache 2.0 — copier/adapter le code est légal. On garde les notices copyright dans les fichiers adaptés.

### Le code Qdrant qu'on a étudié

```
/tmp/qdrant/lib/sparse/src/
├── common/
│   ├── sparse_vector.rs     # SparseVector, RemappedSparseVector, score_vectors()
│   ├── types.rs              # Weight trait (f32, f16, QuantizedU8)
│   └── scores_memory_pool.rs # Buffer pooling
└── index/
    ├── posting_list_common.rs         # PostingElement, PostingElementEx, PostingListIter trait
    ├── posting_list.rs                # PostingList mutable + PostingBuilder
    ├── compressed_posting_list.rs     # BitPacker4x chunks + weight quantization
    ├── search_context.rs              # Batch scoring + WAND pruning
    └── inverted_index/
        ├── inverted_index_ram.rs                       # Vec<PostingList> mutable
        ├── inverted_index_compressed_immutable_ram.rs   # CompressedPostingList in RAM
        └── inverted_index_compressed_mmap.rs            # Mmap'd compressed file
```

Points clés de leur implémentation :
- **3 niveaux** : MutableRam → CompressedImmutableRam → CompressedMmap
- **Dimension remapping** : token IDs globaux → indices denses séquentiels
- **WAND pruning** : `max_next_weight` pré-calculé par élément, skip si contribution max < threshold
- **Batch scoring** : 10k IDs par batch, buffer poolé et réutilisé
- **Immutable mmap** : le format compressé est read-only, mutations reconstruisent tout
- **Weight quantization** : f32 → f16 (÷2) ou u8 (÷4), trade-off précision/mémoire

## Fichiers modifiés (depuis doc 16)

| Fichier | État |
|---------|------|
| `sparse_vector/rust/Cargo.toml` | Modifié (v0.2.0, +ordered-float, +parking_lot) |
| `sparse_vector/rust/src/lib.rs` | Modifié (nouveaux modules) |
| `sparse_vector/rust/src/index.rs` | Réécrit (SparseIndex V2) |
| `sparse_vector/rust/src/posting_list_common.rs` | Nouveau |
| `sparse_vector/rust/src/posting_list.rs` | Nouveau |
| `sparse_vector/rust/src/search_context.rs` | Nouveau |
| `sparse_vector/rust/src/top_k.rs` | Nouveau |
| `sparse_vector/rust/src/scores_memory_pool.rs` | Nouveau |
| `sparse_vector/rust/src/bridge.rs` | Inchangé |
| `sparse_vector/rust/src/handle.rs` | Inchangé (sera modifié en phase 2) |
