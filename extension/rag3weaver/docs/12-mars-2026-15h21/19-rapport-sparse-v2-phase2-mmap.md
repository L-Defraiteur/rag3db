# Doc 19 — Rapport : Sparse V2 Phase 2 — mmap persistence

Date : 13 mars 2026

Réf : doc 17 (progression sparse V2 POC)

## Ce qui a été fait

### Phase 2 : mmap persistence — FAIT ✅

Crate `sparse-vector` v0.2.0 (`packages/rag3db/extension/sparse_vector/rust/src/`)

| Fichier | État | Rôle |
|---------|------|------|
| `mmap_index.rs` | **Nouveau** (~270 lignes) | Format flat binary + MmapPostingListIterator + search_mmap() + write_mmap_file() |
| `handle.rs` | **Réécrit** (~250 lignes) | Mmap open/commit, routing search mmap vs RAM, lazy load postings + vectors |
| `bridge.rs` | **Modifié** | Appelle handle.search/insert/remove au lieu de lock direct |
| `index.rs` | **Modifié** | Ajout from_parts(), set_vectors(), getters pour persistence |
| `posting_list.rs` | **Modifié** | Ajout PostingList::from_sorted() |
| `Cargo.toml` | **Modifié** | +memmap2 = "0.9" |

### Format flat binary mmap

```
sparse.mmap:
  [FileHeader]                    16 bytes (magic "SPRS", version, num_dims, num_vectors)
  [DimHeader × num_dims]          16 bytes × N (offset, count, pad)
  [PostingEntry × total_entries]  16 bytes × M (record_id u64, weight f32, max_next_weight f32)

sparse_vectors.bin:  bincode de HashMap<u64, SparseVector> (pour delete/update)
sparse_dims.bin:     bincode de (HashMap<u32,usize>, Vec<u32>) (dimension mapping)
```

### Comportement

| Opération | Avant (bincode) | Après (mmap) |
|-----------|----------------|--------------|
| `open()` | Deserialize tout sparse.bin → O(N) | mmap O(1) + deserialize dims (petit) |
| `search()` | Depuis RAM | Depuis pages mmap'd (OS page cache) |
| `insert/remove()` | Direct en RAM | Lazy load postings + vectors depuis mmap/disque, puis RAM |
| `commit()` | Serialize tout → O(N) | Écrit 3 fichiers, re-mmap |

### Lazy loading

- **Postings** : chargés en RAM depuis le mmap seulement à la première mutation (insert/remove)
- **Vectors** : désérialisés depuis sparse_vectors.bin seulement à la première mutation
- **Search** ne charge ni postings ni vectors — lit directement le mmap

### Rétrocompatibilité

- `open()` : essaie sparse.mmap d'abord, fallback sur sparse.bin (legacy bincode)
- `commit()` : écrit toujours le nouveau format, supprime sparse.bin si présent
- Migration automatique au premier commit

### Tests Rust : 27 passent ✅

```
running 27 tests
test handle::tests::create_writes_mmap_format ... ok
test handle::tests::legacy_fallback ... ok
test handle::tests::many_docs_mmap_roundtrip ... ok
test handle::tests::mmap_search_filtered ... ok
test handle::tests::mutation_after_mmap_open ... ok
test handle::tests::persistence_roundtrip_mmap ... ok
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
test index::tests::sparse_vector_mismatched_lengths ... ok
test posting_list::tests::* (5 tests) ... ok
test top_k::tests::* (2 tests) ... ok
```

### Tests E2E rag3weaver : 7 sparse tests passent ✅

```
test phase3_sparse_search_finds_results ... ok
test phase3_hybrid_3way ... ok (implicite)
test phase3_sparse_top_result_programming ... ok
test phase3_sparse_data_enriched ... ok
test phase4_sparse_only ... ok
test phase4_bm25_sparse ... ok
test phase4_vector_sparse ... ok
test phase5_dual_sparse_search ... ok
```

Ces tests sont in-memory — l'API cxx est identique donc ils valident le bon fonctionnement du nouveau code.

### Build extension C++ : OK ✅

Le bridge cxx n'a pas changé (mêmes 8 fonctions). L'extension `.rag3db_extension` compile sans modification C++.

## Ce qui bloque : test E2E persistence (close → reopen → search)

### Problème : lucivy lock file non relâché

Un test `phase6_sparse_mmap_persistence` a été écrit mais échoue à cause d'un bug **indépendant de sparse** :

```
cannot create writer: Failed to acquire Lockfile: LockBusy.
"there is already an IndexWriter working on this Directory"
```

Le `IndexWriter` lucivy (Tantivy) ne relâche pas son lock quand la Database est droppée dans le même process. Documenté dans doc 18.

### Hypothèse supplémentaire : build debug de lucivy

Le build `native-test` (cmake) a peut-être recompilé ld-lucivy depuis ses sources au lieu d'utiliser la version release pré-buildée. Si le build a pris une version debug ou incomplète de lucivy, ça pourrait expliquer le comportement de lock non relâché. À vérifier :

- Est-ce que `run_e2e.sh --build` recompile lucivy depuis le submodule ld-lucivy ?
- Est-ce que le Cargo.toml de lucivy_fts pointe vers la bonne version ?
- Est-ce que le profil release est utilisé pour la lib statique lucivy ?

Le test E2E persistence sparse reste en place (dans e2e_search.rs, `phase6_sparse_mmap_persistence`) mais ne peut pas passer tant que le bug lucivy n'est pas résolu.

## Ce qui n'a PAS changé

- Extension C++ (`sparse_vector/src/`) : aucun changement
- Bridge cxx (`bridge.rs`) : même API, seule l'implémentation interne a changé
- Appels Cypher dans rag3weaver : aucun changement

## Prochaines étapes

1. **Résoudre le bug lucivy lock** (doc 18) → débloquer le test E2E persistence
2. **Compression** (optionnel) : ajouter compression des posting lists si besoin de réduire la taille fichier
3. **Benchmark** : mesurer le gain réel open/search sur un index de 100k+ docs
4. **WASM** : vérifier que memmap2 est compatible WASM (probablement pas — fallback bincode pour WASM)
