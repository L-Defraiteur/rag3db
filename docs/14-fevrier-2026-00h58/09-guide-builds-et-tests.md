# Guide — Builds et Tests rag3db

Ce document couvre tous les builds validés et comment les reproduire.

---

## Prérequis communs

```bash
# Rust (nightly + stable)
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly

# Emscripten (pour WASM)
cd ~/emsdk && ./emsdk install latest && ./emsdk activate latest
source ~/emsdk/emsdk_env.sh

# Node.js (v20+)
node --version  # v20.x
```

---

## 1. Tests unitaires Rust (ld-lucivy)

Tests des 1015 fonctions de la lib Lucivy + crate lucivy_fts.

```bash
cd packages/rag3db/extension/lucivy/ld-lucivy
cargo test --lib
```

Résultat attendu : `test result: ok. 1015 passed`

---

## 2. Build natif + tests GTest E2E

Build de rag3db en natif avec l'extension lucivy_fts et les 9 tests E2E.

```bash
cd packages/rag3db
mkdir -p build/release && cd build/release

cmake ../.. \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_EXTENSION_TESTS=TRUE \
  -DBUILD_EXTENSIONS="lucivy_fts" \
  -DBUILD_SHELL=FALSE \
  -DBUILD_TESTS=FALSE

cmake --build . --target lucivy_fts_test -j$(nproc)
```

Exécuter les tests :

```bash
./test/runner/lucivy_fts_test
```

Résultat attendu : 9 tests (CREATE/QUERY/DROP, fuzzy, phrase, contains, filter fields, delete/update, lazy commit).

---

## 3. Build Node.js natif (NAPI addon)

Produit `rag3dbjs.node` + l'extension lucivy_fts en `.rag3db_extension`.

```bash
cd packages/rag3db
mkdir -p build/nodejs && cd build/nodejs

cmake ../.. \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_EXTENSIONS="lucivy_fts" \
  -DBUILD_NODEJS=TRUE \
  -DBUILD_SHELL=FALSE \
  -DBUILD_TESTS=FALSE

cmake --build . --target rag3dbjs -j$(nproc)
cmake --build . --target rag3db_lucivy_fts_extension -j$(nproc)
```

### Tests mocha Node.js natif

```bash
cd packages/rag3db/tools/nodejs_api
npm test
```

Résultat attendu : `139 passing`

### Tester lucivy_fts manuellement

```javascript
const rag3db = require('./tools/nodejs_api/build/Release/rag3dbjs.node');
const db = new rag3db.NodeDatabase(':memory:');
const conn = new rag3db.NodeConnection(db);

conn.querySync("CREATE NODE TABLE docs (id UINT64, body STRING, PRIMARY KEY(id))");
conn.querySync("CREATE (:docs {id: 0, body: 'Rust is a programming language'})");
conn.querySync("CALL CREATE_LUCIVY_INDEX('docs', ['body'])");

// L'extension doit être dans le même répertoire ou chargée via LOAD EXTENSION
const r = conn.querySync(
  `CALL QUERY_LUCIVY_INDEX('docs', '{"type":"fuzzy","field":"body","value":"programing","distance":1}', 10) RETURN node_id, score`
);
console.log(r.toString());
```

---

## 4. Build WASM standard (browser, IDBFS)

Produit un seul fichier JS (~17MB) avec le WASM inline, IDBFS et pthreads.

```bash
cd packages/rag3db
source ~/emsdk/emsdk_env.sh
mkdir -p build/wasm && cd build/wasm

emcmake cmake ../.. \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_WASM=TRUE \
  -DBUILD_SHELL=FALSE \
  -DBUILD_TESTS=FALSE \
  -DBUILD_BENCHMARK=FALSE

emmake cmake --build . -j$(nproc)
```

Sortie : `tools/wasm/build/rag3db/rag3db_wasm.js` (17MB, single file)

### Config clé

| Flag | Valeur | Rôle |
|------|--------|------|
| SINGLE_FILE | 1 | WASM inline dans le JS |
| PTHREAD_POOL_SIZE | 16 | Pré-création de 16 Web Workers |
| lidbfs.js | lié | Persistence IndexedDB |
| lworkerfs.js | lié | WORKERFS pour Web Workers |
| Lucivy writer threads | 1 | Via `#[cfg(target_arch = "wasm32")]` |

### Extensions statiquement liées

- `lucivy_fts` (recherche full-text fuzzy)
- `vector` (index HNSW)
- `json`
- `algo`

**Note** : `fts` (l'ancien FTS basé sur BM25 natif) est retiré du WASM car il dépend de `fuzzy_fst` qui n'est pas compilé pour WASM.

### Utilisation dans le browser

```html
<script src="rag3db_wasm.js"></script>
<script>
  // Le serveur DOIT envoyer ces headers pour SharedArrayBuffer (pthreads) :
  // Cross-Origin-Opener-Policy: same-origin
  // Cross-Origin-Embedder-Policy: require-corp

  const Module = await rag3db();
  const config = new Module.SystemConfig();
  config.maxNumThreads = 2;  // limiter pour le browser
  const db = new Module.Database(":memory:", config);
  const conn = new Module.Connection(db);

  // Queries via Embind (synchrones, à exécuter dans un Web Worker)
  const r = conn.query("RETURN 'hello' AS msg");
  console.log(r.getAsJsArrayOfObjects());
  r.delete();
</script>
```

**Important** : exécuter les opérations WASM dans un **Web Worker dédié** pour éviter de bloquer le main thread et permettre la gestion des pthreads.

### Tests Playwright browser (IDBFS)

```bash
cd packages/rag3db/tools/wasm
npm install   # installe Playwright + deps
npx playwright test --reporter=line
```

Résultat attendu : `2 passed`

Tests couverts :
1. **Phase 1** : create DB → mount IDBFS → insert 4 docs → CREATE_LUCIVY_INDEX → CREATE_VECTOR_INDEX → query contains/fuzzy/phrase/vector → syncfs(false) → sauvé dans IndexedDB
2. **Phase 2** : mount IDBFS → syncfs(true) → reopen DB → re-query → mêmes résultats → persistence validée

Pour voir les tests dans un navigateur visible :

```bash
npx playwright test --headed
```

---

## 5. Build WASM NODEFS (pour tests Node.js)

Variante WASM qui utilise le filesystem Node.js directement. Séparé en .js + .wasm.

```bash
cd packages/rag3db
source ~/emsdk/emsdk_env.sh
mkdir -p build/wasm-nodefs && cd build/wasm-nodefs

emcmake cmake ../.. \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_WASM=TRUE \
  -DWASM_NODEFS=TRUE \
  -DBUILD_SHELL=FALSE \
  -DBUILD_TESTS=FALSE \
  -DBUILD_BENCHMARK=FALSE

emmake cmake --build . -j$(nproc)
```

Sortie : `tools/wasm/build/rag3db/rag3db_wasm.js` (239K) + `rag3db_wasm.wasm` (15MB)

**Attention** : les deux builds (standard et NODEFS) écrivent dans le MÊME répertoire. Supprimer les anciens fichiers avant de changer de variante :

```bash
rm -f tools/wasm/build/rag3db/rag3db_wasm.*
```

Pour savoir quel build est en place :
- ~17MB `.js` seul = standard (browser)
- ~240K `.js` + 15MB `.wasm` = NODEFS

### Copier vers package/nodejs pour les tests mocha

```bash
cp tools/wasm/build/rag3db/rag3db_wasm.* tools/wasm/package/nodejs/rag3db/
```

### Tests mocha WASM NODEFS

```bash
cd packages/rag3db/tools/wasm
npm test
```

Résultat attendu : `94 passing`

---

## 6. Persistence IDBFS (usage browser)

Pattern pour persister une base rag3db dans IndexedDB (browser) :

```javascript
// === Création et sauvegarde ===
const Module = await rag3db();

// 1. Monter IDBFS
Module.FS.mkdir("/database");
Module.FS.mount(Module.FS.filesystems.IDBFS, {}, "/database");

// 2. Créer la DB SOUS le point de montage
//    (important : les lucivy indexes vont dans parent_path(dbPath)/lucivy_indexes/)
const config = new Module.SystemConfig();
config.maxNumThreads = 2;
const db = new Module.Database("/database/mydb", config);
const conn = new Module.Connection(db);

// 3. Créer tables, index, insérer des données...
conn.query("CREATE NODE TABLE ...");
conn.query("CALL CREATE_LUCIVY_INDEX(...)");

// 4. Fermer AVANT de sync
conn.delete();
db.delete();

// 5. Sauvegarder dans IndexedDB
await new Promise((resolve, reject) => {
  Module.FS.syncfs(false, (err) => err ? reject(err) : resolve());
});
Module.FS.unmount("/database");


// === Rechargement (après refresh de page) ===
const Module = await rag3db();
Module.FS.mkdir("/database");
Module.FS.mount(Module.FS.filesystems.IDBFS, {}, "/database");

// Charger depuis IndexedDB
await new Promise((resolve, reject) => {
  Module.FS.syncfs(true, (err) => err ? reject(err) : resolve());
});

// Rouvrir la DB — tout est là (données + lucivy index + vector HNSW)
const db = new Module.Database("/database/mydb", config);
const conn = new Module.Connection(db);
// Les queries marchent immédiatement
```

**Point clé** : le path de la DB doit être un sous-dossier du point de montage IDBFS (`/database/mydb`, pas `/database`), sinon les lucivy indexes (`parent_path(dbPath)/lucivy_indexes/`) sortent du montage et ne sont pas persistés.

---

## 7. Headers serveur requis (COOP/COEP)

Pour que `SharedArrayBuffer` soit disponible (requis par les pthreads WASM) :

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

Exemple Express :

```javascript
app.use((req, res, next) => {
  res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
  res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
  next();
});
```

Sans ces headers, le navigateur refuse `SharedArrayBuffer` et le WASM crash au démarrage.

---

## Récapitulatif des résultats de tests

| Build | Tests | Résultat |
|-------|-------|----------|
| Rust (ld-lucivy) | `cargo test --lib` | 1015 pass |
| Natif GTest E2E | `lucivy_fts_test` | 9 pass |
| Node.js natif mocha | `npm test` (nodejs_api) | 139 pass |
| WASM NODEFS mocha | `npm test` (tools/wasm) | 94 pass |
| WASM browser Playwright | `npx playwright test` | 2 pass (8 sub-tests) |

Toutes les extensions validées en WASM browser : **lucivy_fts** (contains, fuzzy, phrase) + **vector HNSW** (cosine) + **persistence IDBFS**.
