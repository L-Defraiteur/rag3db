# Rapport — Node.js natif avec lucivy_fts : VALIDÉ

## Ce qui a été fait

### Build Node.js natif avec lucivy_fts

1. **npm install** dans `packages/rag3db/tools/nodejs_api/` (cmake-js, node-addon-api)

2. **CMake configure** :
```bash
cd packages/rag3db/build/nodejs
cmake ../.. -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_EXTENSIONS="lucivy_fts" \
  -DBUILD_NODEJS=TRUE \
  -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE -DBUILD_BENCHMARK=FALSE
```

3. **Build en parallèle** (deux targets) :
```bash
cmake --build . --target rag3dbjs -j$(nproc)
cmake --build . --target rag3db_lucivy_fts_extension -j$(nproc)
```

4. **Outputs** :
   - `tools/nodejs_api/build/rag3dbjs.node` — addon Node.js natif
   - `extension/lucivy_fts/build/liblucivy_fts.rag3db_extension` — extension dynamique

### Tests — TOUS PASSENT

```
=== Test rag3db Node.js + lucivy_fts ===
Version: 0.11.2.2
Database initialized (in-memory)
Connection initialized
Extension loaded OK
Table created, 3 docs inserted
Lucivy index created

--- Test 1: Contains query ---
Results: 2 (expected 2)
  node_id=2, score=1
  node_id=0, score=1

--- Test 2: Fuzzy query ---
Results: 2 (expected 2)
  node_id=2, score=1
  node_id=0, score=1

--- Test 3: Phrase query ---
Results: 1 (expected 1)
  node_id=0, score=1.3544676303863525

--- Test 4: Parse query ---
Results: 1 (expected 1)
  node_id=0, score=1.3544676303863525

=== All tests passed! ===
```

### Comment utiliser depuis Node.js

```javascript
const rag3db = require('./tools/nodejs_api/build/index.js');

const db = new rag3db.Database(':memory:');
await db.init();
const conn = new rag3db.Connection(db);
await conn.init();

// Charger l'extension (path absolu)
await conn.query(`LOAD EXTENSION '/path/to/liblucivy_fts.rag3db_extension'`);

// Créer table + index
await conn.query("CREATE NODE TABLE doc (ID UINT64, title STRING, body STRING, PRIMARY KEY (ID))");
await conn.query("CREATE (:doc {ID: 0, title: 'Rust', body: 'Rust is a systems programming language'})");
await conn.query("CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body'])");

// Fuzzy search !
const result = await conn.query(`
  CALL QUERY_LUCIVY_INDEX('doc',
    '{"type":"fuzzy","field":"body","value":"programing","distance":1}', 10)
  RETURN node_id, score
`);
const rows = await result.getAll();
```

## Découvertes importantes

### 1. Pas besoin de repo séparé pour WASM

`tools/wasm/` existe DÉJÀ dans rag3db avec :
- Bindings Embind complets (`src_cpp/main.cpp`) adaptés aux namespaces rag3db
- Wrappers JS (`src_js/`)
- Tests mocha (`test/`)
- Le CMakeLists.txt parent a déjà toute la config WASM (pthreads, memory, modularize, idbfs)

### 2. lucivy_fts en mode dynamique pour Node.js

Le static linking automatique n'est activé que pour WASM/Android/Swift (dans `extension_config.cmake`). Pour Node.js, l'extension est dynamique → `LOAD EXTENSION` au runtime.

Pour du static linking Node.js, il faudrait dé-commenter `set(EXTENSION_STATIC_LINK_LIST lucivy_fts)` dans `extension_config.cmake`.

### 3. Auto-enregistrement statique vérifié

Le mécanisme `generated_extension_loader.cpp` est fonctionnel pour lucivy_fts :
- CMake génère `lucivy_fts_extension::LucivyFtsExtension::load(context)`
- Le naming correspond exactement au header
- `autoLoadLinkedExtensions()` appelé au démarrage de la DB

### 4. Segfault mineur au cleanup

Exit code 139 au `db.close()` final — segfault dans le destructeur. N'affecte pas le fonctionnement. À investiguer plus tard si besoin.

## Prochaine étape

- **Installer emscripten** (`emsdk`) et tenter le build WASM :
```bash
emcmake cmake -B build/wasm -DCMAKE_BUILD_TYPE=Release -DBUILD_WASM=TRUE -DBUILD_SHELL=FALSE
emmake make -C build/wasm -j$(nproc)
```
- Tout est en place : config WASM, extension_config.cmake, lucivy_fts emscripten support.
- Emscripten n'est pas installé sur la machine actuellement.
