# 06 — Phase 3 WASM setEmbedder + fix BM25 + nettoyage SparseIndex

## Ce qui a été fait

### Fix BM25 uuid resolution (FAIT)

**Problème** : `QUERY_TANTIVY_INDEX` retourne `node_id` = UINT64 offset, pas un UUID. `search_bm25()` stockait cet offset directement comme "uuid" (`"42"` au lieu de `"abc-def-123"`). En mode hybride, la fusion RRF ne pouvait jamais matcher BM25 avec vector search.

**Fix** : même pattern que `search_sparse_cypher()` — extraction des `(offset, score)` puis résolution via `MATCH (n:Entity) WHERE OFFSET(id(n)) IN [...] RETURN OFFSET(id(n)), n._uuid`. Join en Rust via HashMap.

**Fichier** : `src/search.rs` — fonction `search_bm25()` réécrite.

### Fix allowed_ids (FAIT)

**Problème** : `RETURN id(n)` retournait un `InternalID` sérialisé en `CypherValue::String("0:42")`. `.as_i64()` dessus retourne `None` → tous les IDs de filtrage perdus silencieusement.

**Fix** : changé `RETURN id(n)` en `RETURN OFFSET(id(n))` qui retourne directement un INT64.

**Fichier** : `src/catalog.rs` — ligne ~897, dans le bloc `allowed_ids`.

### Nettoyage SparseIndex dead code (FAIT)

Supprimé `SparseIndex` (struct + impl + 8 tests) de `sparse_index.rs`. Gardé `SparseVector` (toujours utilisé par `SparseEmbedder` trait, `BgeM3Embedder`, `BM42Embedder`, etc.).

Supprimé `search_sparse()` (ancienne fonction in-memory) + 2 tests dans `search.rs`. Retiré l'import `SparseIndex` de `search.rs` et le ré-export de `lib.rs`.

**Fichiers** :
- `src/sparse_index.rs` — réduit à SparseVector uniquement (~55 lignes au lieu de 250)
- `src/search.rs` — supprimé `search_sparse()` + 2 tests
- `src/lib.rs` — `pub use sparse_index::SparseVector;` (sans SparseIndex)

**Résultat** : 341 tests passent (10 de moins = 8 SparseIndex + 2 search_sparse supprimés).

### Phase 3 : WASM FFI setEmbedder (FAIT côté code, tests en cours)

**Objectif** : Permettre à JS de remplacer le MockEmbedder par un vrai CandleEmbedder après création du Catalog.

**Approche** : Séparer en deux appels FFI (option b du plan 04) :
1. `rag3weaver_catalog_new()` — crée le Catalog avec MockEmbedder (schema init immédiat)
2. `rag3weaver_catalog_set_embedder(ctx, config_ptr, config_len, tokenizer_ptr, tokenizer_len, weights_ptr, weights_len)` — crée un `CandleEmbedder::from_bytes()` et appelle `catalog.set_embedder(Arc::new(embedder))`

Utilise l'API `set_embedder(Arc<dyn Embedder>)` ajoutée en Phase 1.

**Fichiers modifiés** :
- `src/wasm_ffi.rs` — nouvelle fonction FFI `rag3weaver_catalog_set_embedder()`, retourne JSON `{"ok":true}` ou `{"ok":false,"error":"..."}`
- `tools/wasm/src_cpp/weaver_bindings.cpp` :
  - Déclaration extern C de `rag3weaver_catalog_set_embedder`
  - Méthode C++ `Weaver::setEmbedder(val configArr, val tokenizerArr, val weightsArr)` — convertit les Uint8Array JS → vecteurs C++ → appel FFI Rust
  - Binding embind `.function("setEmbedder", &Weaver::setEmbedder)`

**Usage JS** :
```js
const weaver = new Module.Weaver(configJson, dbPath);
// ... fetch model files async ...
const result = weaver.setEmbedder(
  new Uint8Array(configBuf),
  new Uint8Array(tokenizerBuf),
  new Uint8Array(weightsBuf)
);
// result = '{"ok":true}'
```

### Tests Playwright setEmbedder (CRÉÉS, pas encore verts)

3 fichiers créés :
- `tools/wasm/test/browser/set_embedder_worker.js` — worker paramétrable, accepte `{ model: "minilm" | "multilingual" }`
- `tools/wasm/test/browser/set_embedder.html` — lit modèle depuis `?model=` URL param
- `tools/wasm/test/browser/set_embedder.spec.js` — 2 tests séparés (MiniLM ~23MB 3min, Multilingual ~471MB 10min)

**Sélection des tests** :
```bash
npx playwright test -g "MiniLM"          # petit seulement
npx playwright test -g "Multilingual"    # gros seulement
npx playwright test set_embedder         # les deux
```

Test flow : créer Weaver (MockEmbedder) → fetch model HuggingFace → `setEmbedder()` → create 3 docs → drain async → search "programming language" → vérifier résultats.

### Build WASM avec sparse_vector (FAIT, mais tests cassés)

Build WASM reconfiguré avec `BUILD_EXTENSIONS="json;vector;algo;tantivy_fts;sparse_vector"`.
- `sparse_vector/CMakeLists.txt` supporte déjà EMSCRIPTEN (target wasm32-unknown-emscripten, nightly, atomics)
- Build complet réussi : `[100%] Built target rag3db_wasm`
- Extension linkée statiquement via `generated_extension_loader.cpp`

### Bug découvert : Tantivy schema panic avec sparse_vector linké

**Symptôme** : `thread panicked at src/schema/schema.rs:202: Field already exists in schema title`

Le panic arrive dans le SchemaBuilder de ld-tantivy quand `CREATE_TANTIVY_INDEX` est appelé. Le champ `title` est ajouté deux fois au schema Tantivy.

**Important** : Ce bug casse AUSSI le test existant `rag3weaver.spec.js` (qui marchait avant). Ce n'est donc PAS un problème dans nos nouveaux tests.

**Hypothèse probable** : Le `--allow-multiple-definition` dans le linker WASM (nécessaire pour rayon_core dupliqué entre rag3weaver et tantivy_fts) résout mal certains symboles quand `sparse_vector` est ajouté comme troisième lib Rust. Le symbole résolu pourrait pointer vers la mauvaise implémentation.

**Piste d'investigation** :
1. Vérifier si le build SANS `sparse_vector` fonctionne toujours (il devrait — c'était le cas avant)
2. Le problème est dans le linkage, pas dans le code Rust rag3weaver (les 341 tests cargo passent)
3. Possible conflit de symboles entre `sparse_vector` et `tantivy_fts` — les deux ont des crates Rust avec des dépendances communes (serde, bincode, cxx)

**Pour tester sans sparse_vector** : supprimer `CMakeCache.txt` dans `build_wasm/`, reconfigurer sans sparse_vector, rebuild.

## État compilation/tests

- `cargo check` : 0 erreur, 0 warning
- `cargo check --tests` : 0 erreur, 1 warning préexistant (`FilterOp` unused)
- `cargo test` : 341 passed, 0 failed, 13 ignored
- `cargo check --features wasm-emscripten` : 0 erreur, 1 warning préexistant (doc comment sur thread_local)
- Build WASM : OK (avec sparse_vector)
- Tests Playwright : CASSÉS (panic Tantivy schema, à investiguer — problème de linkage)

## Commit poussé

```
c0118e066 feat(rag3weaver): sparse via Cypher extension, Arc API, fix BM25 uuid resolution
```
Contient : Phase 1 (Arc API) + Phase 2 (sparse Cypher) + fix BM25 + fix allowed_ids + nettoyage dead code + docs 04/05.

## Fichiers modifiés cette session (non committés)

| Fichier | Changement |
|---------|-----------|
| `src/wasm_ffi.rs` | Nouvelle FFI `rag3weaver_catalog_set_embedder()` |
| `tools/wasm/src_cpp/weaver_bindings.cpp` | extern C + méthode C++ + embind pour setEmbedder |
| `tools/wasm/test/browser/set_embedder_worker.js` | Nouveau — worker paramétrable (minilm/multilingual) |
| `tools/wasm/test/browser/set_embedder.html` | Nouveau — page HTML test |
| `tools/wasm/test/browser/set_embedder.spec.js` | Nouveau — 2 tests Playwright |

## Prochaines étapes

1. **Résoudre le bug de linkage WASM** : tester sans sparse_vector pour confirmer, puis investiguer le conflit de symboles `--allow-multiple-definition`
2. **Faire passer les tests Playwright** setEmbedder
3. **Commit + push** Phase 3
