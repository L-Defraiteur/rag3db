# Rapport 07 — Tests complets rag3db WASM + Node.js natif

## Résumé

Toutes les validations sont passées. rag3db compile en WASM et en Node.js natif avec les extensions tantivy_fts, vector, json, algo.

## Résultats des tests

### Node.js natif (addon NAPI)
- **139 tests mocha : TOUS PASSENT** (2s)
- Tests couvrent : Database, Connection, QueryResult, types de données (BOOL, INT8-64, UINT8-64, FLOAT, DOUBLE, STRING, BLOB, UUID, DATE, TIMESTAMP, INTERVAL, LIST, ARRAY, STRUCT, NODE, REL, RECURSIVE_REL, MAP, DECIMAL), paramètres, concurrence, version, API synchrone
- tantivy_fts testé manuellement : contains, fuzzy, phrase, parse — OK
- Extension chargée dynamiquement via `LOAD EXTENSION`
- Build dir : `packages/rag3db/build/nodejs`

### WASM NODEFS (pour tests Node.js)
- **94 tests mocha : TOUS PASSENT** (4s)
- Même couverture que le natif sauf API sync et quelques edge cases
- Build avec `-DWASM_NODEFS=TRUE` → `.js` + `.wasm` séparés
- NODERAWFS activé → accès filesystem réel Node.js
- Build dir : `packages/rag3db/build/wasm-nodefs`

### WASM standard (pour browser, MEMFS + IDBFS)
- **tantivy_fts** : 3/3 tests manuels PASS (contains, fuzzy, phrase)
- **vector HNSW** : 5/5 tests manuels PASS (create index, query cosine, hybrid search, drop index)
- Build avec `-DBUILD_WASM=TRUE` → single file `.js` 17MB (WASM inline)
- Extensions statiquement linkées : json, vector, algo, tantivy_fts
- IDBFS et WORKERFS inclus (pour persistance browser)
- Build dir : `packages/rag3db/build/wasm`

## Détail du test vector HNSW en WASM

```
Table 'docs' (id UINT64, title STRING, body STRING, embedding FLOAT[4])
4 documents insérés

Test 1: tantivy_fts fuzzy "programing" (distance 1) → 1 résultat (doc 0) ✓
Test 2: CREATE_VECTOR_INDEX cosine → OK ✓
Test 3: QUERY_VECTOR_INDEX [0.12,0.22,0.32,0.42] top 3 → OK ✓
  id=0 (Rust), distance=0.00039
  id=2 (JavaScript), distance=0.00009  (le plus proche)
  id=1 (Python), distance=0.02459
Test 4: Hybrid search (tantivy text + vector cosine intersection) → 3 résultats ✓
Test 5: DROP_VECTOR_INDEX → OK ✓
```

## Builds disponibles

### 3 variantes WASM

| Variante | Flag cmake | Output | FS | Usage |
|----------|-----------|--------|-----|-------|
| Standard | `-DBUILD_WASM=TRUE` | `rag3db_wasm.js` 17MB (inline) | MEMFS + IDBFS + WORKERFS | Browser |
| NODEFS | `-DBUILD_WASM=TRUE -DWASM_NODEFS=TRUE` | `.js` 239K + `.wasm` 15MB | NODERAWFS | Tests Node.js |
| Node.js natif | `-DBUILD_NODEJS=TRUE` | `rag3db.node` ~50MB | Natif | Serveur |

### Commandes de build

```bash
# WASM standard (browser)
cd packages/rag3db/build/wasm
source ~/emsdk/emsdk_env.sh
emcmake cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_WASM=TRUE \
  -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE -DBUILD_BENCHMARK=FALSE
emmake cmake --build . -j$(nproc)

# WASM NODEFS (tests)
cd packages/rag3db/build/wasm-nodefs
source ~/emsdk/emsdk_env.sh
emcmake cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_WASM=TRUE -DWASM_NODEFS=TRUE \
  -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE -DBUILD_BENCHMARK=FALSE
emmake cmake --build . -j$(nproc)

# Node.js natif
cd packages/rag3db/build/nodejs
cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSIONS="tantivy_fts" \
  -DBUILD_NODEJS=TRUE -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE
cmake --build . --target rag3dbjs -j$(nproc)
cmake --build . --target rag3db_tantivy_fts_extension -j$(nproc)

# Lancer les tests
cd packages/rag3db/tools/nodejs_api && npx mocha test --timeout 20000  # 139 tests
cd packages/rag3db/tools/wasm && npx mocha test --timeout 20000        # 94 tests (NODEFS)
```

## Extensions statiques WASM

Configuré dans `extension/extension_config.cmake` :
- **json** — parsing/création JSON
- **vector** — HNSW index, cosine/L2/dot_product
- **algo** — algorithmes de graphe (PageRank, etc.)
- **tantivy_fts** — fuzzy/phrase/contains/parse search, highlights, filter fields
- ~~fts~~ — retirée (remplacée par tantivy_fts, avait un bug de compilation WASM)

## Fichiers modifiés pendant cette session

| Fichier | Modification |
|---------|-------------|
| `extension/fts/src/function/query_fts_index.cpp` | Fix forward declaration constexpr |
| `extension/tantivy/ld-tantivy/tantivy_fts/rust/build.rs` | `-fexceptions -sDISABLE_EXCEPTION_CATCHING=0` pour emscripten |
| `extension/tantivy_fts/CMakeLists.txt` | nightly + atomics + build-std + panic=abort pour WASM |
| `extension/extension_config.cmake` | Retiré `fts` de la liste WASM |

## Prérequis installés

- Emscripten 5.0.1 (`~/emsdk/`)
- Rust nightly + `rust-src` + target `wasm32-unknown-emscripten`
- Node.js v20.19.6

## Prochaine étape

- **Tests browser Playwright avec IDBFS** : tester la persistance réelle (mount IDBFS → create DB → syncfs → reload → query)
- Valider le workflow complet RAG dans Chromium headless
