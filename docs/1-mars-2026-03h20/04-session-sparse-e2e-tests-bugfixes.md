# 04 — Session : Tests E2E sparse BGE-M3 + bugfixes catalog

## Ce qui a été fait

### Tests E2E sparse avec vrai modèle BGE-M3

4 tests ajoutés dans `tests/e2e_search.rs`, tous gated `#[cfg(feature = "bge-m3")]` + `#[ignore]` :

| Test | Vérifie |
|------|---------|
| `phase3_sparse_search_finds_results` | Hybrid+sparse retourne des résultats avec `sparse_count > 0` |
| `phase3_hybrid_3way` | Les 3 signaux contribuent : `vector_count > 0`, `bm25_count > 0`, `sparse_count > 0` |
| `phase3_sparse_top_result_programming` | Query "memory safety systems programming" → Rust doc en top result |
| `phase3_sparse_data_enriched` | `result.data.is_some()` directement après search (Level 1 enrichment) |

Config : `make_sparse_config()` — KB "kb" avec `SearchMode::Hybrid`, `sparse: true`, `embedding_dim: 1024`.
Setup : `setup_sparse_catalog()` — BGE-M3 comme `Embedder` (dense) ET `SparseEmbedder` (sparse), 3 docs thématiques.
Mode BM25 : `ContainsSplit` (pas `Contains` par défaut qui cherche une sous-chaîne contiguë).

### Bug 1 : Sparse index créé sur la mauvaise table

**Symptôme** : `sparse_count = 0` alors que les sparse embeddings existent sur les chunks.

**Cause** : Dans `catalog.rs` `initialize()`, le `CREATE_SPARSE_VECTOR_INDEX` itérait `kb_meta.entities` qui contient `"Document"`, mais les sparse embeddings sont stockées sur `"Document_Chunk"` (via le `SparseEmbedProcessor` qui écrit sur l'entité du chunk).

**Fix** : Quand l'entité a des chunks (`entity_has_chunks`), créer l'index sparse sur `"{entity}_Chunk"` au lieu de `"{entity}"`.

```rust
// Avant
for entity in &kb_meta.entities {
    CALL CREATE_SPARSE_VECTOR_INDEX('{entity}', ...)
}

// Après
let target = if entity_has_chunks(...) {
    format!("{entity}_Chunk")
} else {
    entity.clone()
};
CALL CREATE_SPARSE_VECTOR_INDEX('{target}', ...)
```

### Bug 2 : enrich_fields du parent passés au sparse search sur chunks

**Symptôme** : `Binder exception: Cannot find property title for n.` — erreur avalée par `.unwrap_or_default()`.

**Cause** : `search_sparse_cypher()` était appelé avec `&vector_entity` = `"Document_Chunk"` et `&enrich_fields` = `["title", "body"]` (champs du parent `Document`). Le chunk n'a pas ces colonnes.

**Fix** : Passer `&[]` au sparse search quand `is_chunked` — l'enrichment est géré ensuite par `resolve_vector_chunks()` qui fait chunk→parent + enrichment.

```rust
let sparse_fields = if is_chunked { &[][..] } else { &enrich_fields };
search::search_sparse_cypher(conn, &vector_entity, &qv, limit, sparse_fields)
```

### Bug 3 : erreur sparse avalée silencieusement

**Symptôme** : `sparse_count = 0` sans aucune erreur visible.

**Cause** : `.unwrap_or_default()` sur le résultat de `search_sparse_cypher()` dans `Catalog::search()`.

**Fix** : Remplacé par `?` pour propager l'erreur.

### Bug BM25 (pas un vrai bug, mauvais mode dans les tests)

`BM25Mode::Contains` (le défaut) cherche la query comme sous-chaîne **contiguë**. "systems programming memory safety" n'existe nulle part verbatim → 0 résultats. Fix : utiliser `BM25Mode::ContainsSplit` qui split par mot.

### Activation GPU (CUDA) pour les tests

**Symptôme** : drain de 3 docs prenait 71 secondes.

**Cause** : Le feature `cuda` n'était pas activé dans `run_e2e.sh`. Candle tournait sur CPU malgré une RTX 2070 disponible.

**Fix** dans `run_e2e.sh` :
- Features : `rag3db-native,candle-embedder,bge-m3` → `rag3db-native,candle-embedder,bge-m3,cuda`
- Ajout `export PATH="/usr/local/cuda/bin:$PATH"` (pour `nvcc`)
- Ajout `export CUDA_ROOT="/usr/local/cuda"`
- Ajout `/usr/local/cuda/lib64` dans `LD_LIBRARY_PATH`

**Résultat** :

| Métrique | CPU | GPU |
|----------|-----|-----|
| Drain 3 docs | 71s | 0.3s |
| Total setup | 80s | 9s |
| 4 tests phase3 | ~5 min | **9.3s** |

Le modèle BGE-M3 est chargé une seule fois via `LazyLock` (~8.6s), les 4 tests tournent en parallèle.

## Fichiers modifiés

```
extension/rag3weaver/src/catalog.rs     — fix sparse index table, fix enrich_fields chunked, fix unwrap_or_default
extension/rag3weaver/tests/e2e_search.rs — 4 tests phase3, imports SparseEmbedder, timing instrumentation, cleanup
extension/rag3weaver/run_e2e.sh          — cuda feature, CUDA_ROOT, PATH, LD_LIBRARY_PATH
```

## Fichiers modifiés non commités (cumul sessions 03+04)

```
extension/rag3weaver/src/search.rs      — resolve_and_enrich_chunked, resolve_vector_chunks, search_bm25_chunked, refacto search_bm25/search_sparse/fuse_results
extension/rag3weaver/src/catalog.rs     — Level 1 composed queries + sparse bugfixes
extension/rag3weaver/tests/e2e_search.rs — phase3 sparse tests + timing
extension/rag3weaver/run_e2e.sh         — CUDA support
```

## Build & Tests

```
cargo check                    ✓
cargo test --lib               ✓ 345 passed
run_e2e.sh phase3              ✓ 4 passed (9.3s avec GPU)
```

## Ce qui reste à faire

1. **Commiter** les modifications Level 1 + sparse bugfixes + tests phase3 + CUDA
2. **Retirer le timing instrumentation** de `setup_sparse_catalog()` (ou le garder, utile pour debug)
3. **Vérifier les autres phases** : `run_e2e.sh` (phase0, phase1, phase2) pour s'assurer pas de régression avec les changements catalog.rs
4. **Test fuse_results data preservation** : test unitaire pur vérifiant que `.data` survit à la fusion (doc 03)
5. **Test data enrichment phase1/phase2** : ajouter `assert result.data.is_some()` aux tests existants
