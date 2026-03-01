# 11 — Fix Tantivy lock, 3 modes BM25, debug pipeline vector

## Ce qu'on a fait

### 1. Fix Tantivy lock conflict pour DB in-memory

**Problème** : toutes les DB in-memory partageaient le même chemin Tantivy sur disque (`/tmp/rag3db_tantivy/tantivy_indexes/<table>/`). Deux DB créant la même table se battaient pour le même lock Tantivy → `LockBusy`.

**Fix** : utiliser l'adresse du `Database*` comme identifiant unique dans le chemin :

```cpp
// Avant (bugué)
basePath = std::filesystem::temp_directory_path().string() + "/rag3db_tantivy";

// Après (unique par instance)
auto dbId = std::to_string(reinterpret_cast<uintptr_t>(context.clientContext->getDatabase()));
basePath = std::filesystem::temp_directory_path().string() + "/rag3db_tantivy/" + dbId;
```

**Fichiers modifiés** :
- `extension/tantivy_fts/src/function/create_tantivy_index.cpp` (lignes 238-248)
- `extension/sparse_vector/src/function/create_sparse_vector_index.cpp` (lignes 150-160)

**Résultat** : les 6 tests Phase 0 passent **en parallèle** (plus besoin de `--test-threads=1`), y compris `phase0_hashsafe_dedup` qui crée 2 catalogs dans le même process.

### 2. Trois modes BM25 dans rag3weaver

Le mode `Contains` (NgramContainsQuery) fait du substring matching — "Rust safety" cherche cette phrase contiguë, pas les mots séparément. "neural networks" marchait car c'est une sous-chaîne contiguë dans le texte.

**Ajouté dans `search.rs`** :

| Mode | Comportement |
|------|-------------|
| `Contains` | Substring fuzzy exact (existant, inchangé) |
| `ContainsSplit` | **Nouveau** — split les mots, boolean `should` de contains par mot. "Rust safety" → cherche docs contenant "Rust" ET/OU "safety" n'importe où |
| `Parse` | **Nouveau** — native Tantivy QueryParser, BM25 standard term-by-term |
| `Regex` | Regex substring (existant, inchangé) |

**Fichiers modifiés** :
- `extension/rag3weaver/src/search.rs` — enum `BM25Mode` + refactoring de `build_bm25_query()` avec `build_contains_clauses()` helper

### 3. Tests Phase 1 BM25 — 6/6 PASS

| Test | Mode | Query | Résultat |
|------|------|-------|----------|
| `phase1_bm25_contains_exact` | Contains | "neural networks" | ✅ 1 result |
| `phase1_bm25_contains_no_distant` | Contains | "Rust safety" | ✅ 0 (attendu) |
| `phase1_bm25_split_distant_words` | ContainsSplit | "Rust safety" | ✅ 1 result |
| `phase1_bm25_split_french` | ContainsSplit | "cuisine française" | ✅ 1 result |
| `phase1_bm25_parse_multi_term` | Parse | "Rust safety" | ✅ 1 result |
| `phase1_bm25_no_results` | Contains | "xyznonexistent" | ✅ 0 results |

### 4. Phase 2 Vector — pipeline raw OK, Catalog KO

**Embedders cachés** : `LazyLock<Arc<dyn Embedder>>` pour charger chaque modèle une seule fois entre tests (MiniLM, MultilingualMiniLM, BGE-M3).

**Test `phase2_raw_vector_pipeline` — ✅ PASS** :
- Crée table, embed avec MiniLM réel (384d), insère, crée index HNSW, query → **3 résultats**, distances cohérentes
- `rust_doc` distance 0.28, `ml_doc` 0.85, `cuisine_doc` 1.04 pour query "systems programming language"
- Prouve que les extensions vector + le CandleEmbedder marchent correctement

**Test `phase2_vector_minilm_programming` — ❌ FAIL** :
- Via `Catalog.search()` en mode Semantic → `vector_count=0`
- Le drain dit `processed=3, failed=0` mais les embeddings ne semblent pas arriver dans la DB
- **Cause probable** : le `EmbedProcessor` utilise `self.embedder` qui est l'Arc. On fait `set_embedder()` APRÈS `Catalog::new()`, mais `initialize()` enregistre les processors qui capturent `self.embedder` au moment de l'initialisation. Si `set_embedder` est appelé entre `new()` et `initialize()`, il devrait être OK car `initialize()` lit `self.embedder` à ce moment. À investiguer : est-ce que le drain produit effectivement des `CatalogOp::Embed` items dans la queue ?

### 5. Mise à jour run_e2e.sh

- Retiré `--test-threads=1` (plus nécessaire grâce au fix lock)
- Features étendues : `rag3db-native,candle-embedder,bge-m3`

## État des tests (12/12 + 1/1 + 0/9)

### Phase 0 — 6/6 ✅ (tous en parallèle)
### Phase 1 — 6/6 ✅
### Phase 2 — 1 raw pipeline ✅, 9 via Catalog ❌ (vector_count=0)

## Fichiers créés/modifiés

| Fichier | Action |
|---------|--------|
| `extension/tantivy_fts/src/function/create_tantivy_index.cpp` | Fix lock — dbId unique |
| `extension/sparse_vector/src/function/create_sparse_vector_index.cpp` | Fix lock — dbId unique |
| `extension/rag3weaver/src/search.rs` | 3 modes BM25 (ContainsSplit, Parse) |
| `extension/rag3weaver/tests/e2e_search.rs` | Phase 1 (6 tests) + Phase 2 (10 tests) + LazyLock embedders |
| `extension/rag3weaver/run_e2e.sh` | Features candle-embedder+bge-m3, retiré --test-threads=1 |

## Prochaines étapes

1. **Debugger le pipeline Catalog vector** — le raw pipeline marche, donc le problème est dans Catalog (drain/embed). Vérifier que `EmbedProcessor` reçoit bien les items Embed, que l'embedder Arc est bien le bon (pas le MockEmbedder initial), et que les embeddings sont stockés en DB.
2. **Finir Phase 2** — une fois le bug fixé, les 9 tests via Catalog devraient passer (3 MiniLM, 3 Multilingual, 3 BGE-M3)
3. **Continuer Phases 3-10** du cahier des charges (doc 09)
