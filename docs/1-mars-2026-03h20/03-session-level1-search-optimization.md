# 03 — Session Level 1 : Search Optimization + Tests manquants

## Ce qui a été fait

### Level 1 — Composition Cypher (search.rs)

6 nouvelles fonctions/structs ajoutées dans `extension/rag3weaver/src/search.rs` :

| Fonction | Rôle | Queries |
|----------|------|:-------:|
| `resolve_and_enrich()` | Merge offset→UUID + data enrichment en 1 query | 1 |
| `resolve_and_enrich_chunked()` | Merge offset→UUID + chunks + data en 1 query (flat join, group en Rust) | 1 |
| `resolve_vector_chunks()` | Merge chunk→parent + data enrichment en 1 query (vector/sparse chunked) | 1 |
| `search_bm25_chunked()` | CALL + resolve_and_enrich_chunked + highlight→chunk matching | 2 |
| `ResolvedParent` | Struct intermédiaire : uuid + data + Vec<ChunkRecord> | — |
| `ChunkRecord` | Struct chunk promue au niveau module (était local dans resolve_bm25_to_chunks) | — |

### Refactoring existant

| Fonction modifiée | Changement |
|-------------------|-----------|
| `search_bm25()` | +`return_fields` param, utilise `resolve_and_enrich()` au lieu du bloc inline |
| `search_sparse_cypher()` | +`return_fields` param, utilise `resolve_and_enrich()` au lieu du bloc inline |
| `fuse_results()` | +`data_map` (même pattern que `chunk_map`), re-attache `.data` après fusion |

### Catalog::search() (catalog.rs)

- `enrich_fields` calculé en amont (avant les search calls)
- BM25 chunked → `search_bm25_chunked()` au lieu de `search_bm25_raw()` + `resolve_bm25_to_chunks()`
- Vector/sparse chunked → `resolve_vector_chunks()` au lieu de `resolve_chunk_results()`
- Sparse → passe `&enrich_fields` à `search_sparse_cypher()`
- BM25 non-chunked → passe `&enrich_fields` à `search_bm25()`
- Enrichment post-pagination conditionnel : seulement si `any(r.data.is_none())`

### Réduction de queries

| Search type | Avant | Après | Gain |
|-------------|:-----:|:-----:|:----:|
| BM25 non-chunked | 3 (CALL + resolve + enrich) | 2 (CALL + resolve_and_enrich) | -33% |
| BM25 chunked | 4 (CALL + resolve + chunks + enrich) | 2 (CALL + resolve_and_enrich_chunked) | -50% |
| Vector/sparse chunked | 3 (CALL + chunk→parent + enrich) | 2 (CALL + resolve_vector_chunks) | -33% |
| Sparse non-chunked | 3 (CALL + resolve + enrich) | 2 (CALL + resolve_and_enrich) | -33% |

### Build

```
cargo check   ✓
cargo test    ✓ 345 passed, 0 failed
```

### Commit

```
9433ece3d feat: geo extension + Level 1 search optimization (resolve_and_enrich)
```
Pushé sur `origin/master`.

Note : ce commit contenait aussi toute l'extension geo (R-tree, 22 scalar functions, etc.) + le doc 02.

## Ce qui reste à faire

### Commit Level 1 complet

Les modifications post-commit (search_bm25_chunked, resolve_vector_chunks, refacto catalog.rs, fuse_results data_map, search_sparse return_fields) ne sont PAS encore committées. Faire :

```bash
cd packages/rag3db
git add extension/rag3weaver/src/search.rs extension/rag3weaver/src/catalog.rs
git commit -m "feat(rag3weaver): Level 1 search optimization — composed Cypher queries"
git push
```

### Tests E2E manquants — sparse via vrai modèle

Aucun test E2E pour sparse search n'existe. Les tests sparse dans search.rs sont uniquement sur la logique de fusion pure (fuse_results avec des SearchResult construits à la main).

À ajouter dans `tests/e2e_search.rs` — Phase 3 :

1. **`load_extensions()`** : déjà modifié pour charger `sparse_vector` (en plus de vector + lucivy_fts)

2. **`setup_sparse_catalog()`** à créer :
   - Config avec `sparse: true` sur le KB
   - Utiliser BGE-M3 comme embedder (fait dense + sparse nativement)
   - `catalog.set_sparse_embedder(BGE_M3.clone())` — BGE-M3 implémente SparseEmbedder
   - Mêmes 3-4 docs thématiques que les autres phases
   - Drain, vérifier que sparse index est créé

3. **Tests à écrire** (tous `#[cfg(feature = "bge-m3")]`) :
   - `phase3_sparse_search_finds_results` — search BM25Only avec sparse activé, vérifier sparse_count > 0
   - `phase3_sparse_top_result` — search avec query "systems programming", top result = Rust doc
   - `phase3_hybrid_3way` — search Hybrid avec sparse activé, vérifier vector_count + bm25_count + sparse_count > 0
   - `phase3_sparse_data_enriched` — vérifier que `result.data` est Some (pas None) grâce au Level 1

4. **Vérifier** que `BgeM3Embedder` implémente `SparseEmbedder` :
   ```
   grep "impl SparseEmbedder for BgeM3" extension/rag3weaver/src/bge_m3_embedder.rs
   ```

### Tests data enrichment (Level 1 spécifique)

Vérifier que les résultats de search ont `data: Some(...)` directement après search, sans attendre l'enrichment post-pagination :

- Ajouter à phase1 ou phase2 : assert `result.data.is_some()` sur les résultats
- Ajouter test fuse_results data preservation (unitaire pur, pas besoin de mock)

### Fonctions conservées (ne PAS supprimer)

Ces fonctions restent utiles standalone / pour les tests existants :
- `search_bm25_raw()` — utilisée par search_bm25_chunked internals, tests
- `resolve_bm25_to_chunks()` — standalone, tests
- `resolve_chunk_results()` — standalone, tests
- `enrich_results_with_data()` — fallback pour vector non-chunked

## Fichiers modifiés (non commités)

```
extension/rag3weaver/src/search.rs   — 6 fonctions ajoutées + 3 refactorées
extension/rag3weaver/src/catalog.rs  — enrich_fields en amont, nouveaux helpers, enrichment conditionnel
extension/rag3weaver/tests/e2e_search.rs — load_extensions + sparse_vector, import MockSparseEmbedder
```
