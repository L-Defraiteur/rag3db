# 04 — Recherche sur les concessions : résultats

## 1. Core offsets — IMPLÉMENTÉ (Option B, core-first)

### Algo retrouvé

`packages/ragforge-core-exp-kuzu/kuzu-wasm-exp/src/lib/l3/SemanticChunker.ts`

L'ancienne implémentation L3 calcule les cores d'abord, puis étend avec l'overlap. On a adopté cette approche (Option B) directement dans le Chunker Rust.

### Ce qui a été fait

Refactor complet de `chunker.rs` — algo core-first en deux phases :

**Phase 1 : Cores via text-splitter (sans overlap)**
```
core_size = max_size - overlap   (ex: 1500 - 200 = 1300 chars)
text-splitter appelé avec max_size=core_size, overlap=0
→ cores aux boundaries sémantiques, contiguës, sans trou
```

**Phase 2 : Extension avec overlap (`extend_cores_to_chunks()`)**
```
for each core:
    start_byte = max(0, core_start - overlap)   // snap UTF-8
    end_byte = min(text.len, core_end + overlap) // snap UTF-8
    chunk = text[start_byte..end_byte]
    core offsets = (core_start, core_end)  // inchangés
```

### Struct Chunk

4 champs ajoutés :
```rust
pub core_start_byte: usize,  // début de la zone possédée
pub core_end_byte: usize,    // fin de la zone possédée
pub core_start_line: usize,
pub core_end_line: usize,
```

### Propriétés garanties (testées)

- **Contiguïté** : `core_end[i] == core_start[i+1]` pour tous les chunks consécutifs
- **Couverture** : `core_start[0] == 0` et `core_end[last] == text.len()`
- **Inclusion** : `start_byte <= core_start_byte` et `core_end_byte <= end_byte`
- **Single chunk** : core = full range (pas d'overlap inutile)

### Différence vs Option A (approximation)

Option A aurait déduit le core a posteriori en prenant le milieu de la zone overlap entre voisins. L'Option B est exacte : les cores sont calculés par text-splitter, pas approximés.

### Impact sur catalog.rs

`build_chunk_ops()` utilise directement `chunk.core_start_byte` etc. au lieu du TODO qui copiait les offsets normaux.

### Tests

+6 tests : `cores_are_contiguous`, `cores_cover_full_text`, `core_within_chunk_bounds`, `single_chunk_core_equals_full`, `core_lines_within_chunk_lines`, `fixed_cores_contiguous`

345 tests total, 0 failures.

## 2. Cache Chunker

Pas implémenté. Recommandation inchangée : registre par config hash sur le Catalog. Faible priorité.

## 3. Batching des processors — DÉJÀ FAIT

**Les processeurs sont déjà correctement batchés.** Résultat du plan V2 (1A et 1B).

### EmbedProcessor

```
Phase 1: Collect all EmbedWorks (uuid, text, entity_name, col)
Phase 2: ONE call to embedder.embed(&texts) for entire batch
Phase 3: Group by (entity_name, col) → ONE UNWIND query per group
```

### SparseEmbedProcessor

Même pattern : 1 appel `embed_sparse(&texts)` + 1 UNWIND par groupe.

### Batch sizes (ops.rs)

| Processor | batch_size | max_retries |
|---|---|---|
| Insert | 50 | 3 |
| Link | 50 | 3 |
| Embed | 32 | 3 |
| SparseEmbed | 32 | 2 |

10 chunks → 1 batch → 1 appel embedder + 1 query UNWIND. 100 chunks → 4 batches.

**Aucune concession sur le batching.**

## 4. Clones de textes

Pas implémenté. `Arc<String>` reste la solution prévue (plan V2). ~4.5 KB de copies par chunk, pas bloquant.

## 5. Invalidation content hash

L'invalidation reste au niveau document entier (delete-all chunks + re-create). Le `_text_hash` par chunk est stocké mais pas utilisé pour l'invalidation fine. L'ancienne version L3 fait pareil.

Flow futur possible : comparer `text_hash` ancien vs nouveau par index pour ne re-embedder que les chunks changés. Hors scope pour l'instant.

## 6. Parallélisme chunking

Pas implémenté. Faible priorité (Chunker est CPU-only, <1ms pour 10KB).

## Résumé

| # | Concession | Statut |
|---|---|---|
| 1 | Core offsets | **FAIT** (Option B, core-first) |
| 2 | Cache Chunker | À faire (faible prio) |
| 3 | Batch processors | **DÉJÀ FAIT** |
| 4 | Arc textes | À faire (plan V2) |
| 5 | Invalidation fine | À faire (futur) |
| 6 | Parallélisme chunking | À faire (faible prio) |
