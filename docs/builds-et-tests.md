# Guide — Builds et Tests

Ce document couvre tous les builds valides et comment les reproduire. Tous les chemins sont relatifs a la racine du repo rag3db.

---

## Prerequis

```bash
# Rust (nightly + stable)
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly

# Emscripten (pour WASM uniquement)
git clone https://github.com/emscripten-core/emsdk.git ~/emsdk
cd ~/emsdk && ./emsdk install latest && ./emsdk activate latest
source ~/emsdk/emsdk_env.sh

# Node.js v20+
node --version
```

---

## 1. Tests unitaires Rust (ld-tantivy)

1015 tests de la lib Tantivy + crate tantivy_fts.

```bash
cd extension/tantivy/ld-tantivy
cargo test --lib
```

Resultat attendu : `test result: ok. 1015 passed`

---

## 2. Build natif + tests GTest E2E

9 tests E2E de l'extension tantivy_fts (CREATE/QUERY/DROP, fuzzy, phrase, contains, filter fields, delete/update, lazy commit).

```bash
mkdir -p build/release && cd build/release

cmake ../.. \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_EXTENSION_TESTS=TRUE \
  -DBUILD_EXTENSIONS="tantivy_fts" \
  -DBUILD_SHELL=FALSE \
  -DBUILD_TESTS=FALSE

cmake --build . --target tantivy_fts_test -j$(nproc)

# Lancer les tests
./test/runner/tantivy_fts_test
```

---

## 3. Build Node.js natif (NAPI addon)

Produit `rag3dbjs.node` (addon NAPI) + l'extension tantivy_fts en `.rag3db_extension` chargeable dynamiquement.

```bash
mkdir -p build/nodejs && cd build/nodejs

cmake ../.. \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_EXTENSIONS="tantivy_fts" \
  -DBUILD_NODEJS=TRUE \
  -DBUILD_SHELL=FALSE \
  -DBUILD_TESTS=FALSE

cmake --build . --target rag3dbjs -j$(nproc)
cmake --build . --target rag3db_tantivy_fts_extension -j$(nproc)
```

### Tests mocha (139 tests)

```bash
cd tools/nodejs_api
npm install
npm test
```

### Usage

```javascript
const rag3db = require('./tools/nodejs_api/build/Release/rag3dbjs.node');
const db = new rag3db.NodeDatabase(':memory:');
const conn = new rag3db.NodeConnection(db);

// Charger l'extension (chemin vers le .rag3db_extension)
conn.querySync("LOAD EXTENSION 'build/nodejs/extension/tantivy_fts/tantivy_fts.rag3db_extension'");

conn.querySync("CREATE NODE TABLE docs (id UINT64, body STRING, PRIMARY KEY(id))");
conn.querySync("CREATE (:docs {id: 0, body: 'Rust is a programming language'})");
conn.querySync("CALL CREATE_TANTIVY_INDEX('docs', ['body'])");

const r = conn.querySync(
  `CALL QUERY_TANTIVY_INDEX('docs',
    '{"type":"fuzzy","field":"body","value":"programing","distance":1}', 10)
   RETURN node_id, score`
);
console.log(r.toString());
```

---

## 4. Build WASM browser (IDBFS)

Produit un seul fichier JS (~17MB) avec le WASM inline. Extensions liees statiquement : tantivy_fts, vector, json, algo.

```bash
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

Sortie : `tools/wasm/build/rag3db/rag3db_wasm.js` (~17MB, single file)

### Config WASM

| Flag cmake | Valeur | Role |
|------------|--------|------|
| SINGLE_FILE | 1 | WASM inline dans le JS (pas de .wasm separe) |
| PTHREAD_POOL_SIZE | 16 | 16 Web Workers pre-crees au demarrage |
| lidbfs.js | lie | Persistence IndexedDB (IDBFS) |
| lworkerfs.js | lie | Filesystem pour Web Workers |

Cote Rust, Tantivy utilise 1 writer thread sur `wasm32` (`#[cfg(target_arch = "wasm32")]` dans `handle.rs`) pour eviter d'epuiser le pool de pthreads.

### Usage dans le browser

```html
<script src="rag3db_wasm.js"></script>
<script>
  // IMPORTANT : le serveur DOIT envoyer ces headers (sinon SharedArrayBuffer refuse) :
  //   Cross-Origin-Opener-Policy: same-origin
  //   Cross-Origin-Embedder-Policy: require-corp

  const Module = await rag3db();
  const config = new Module.SystemConfig();
  config.maxNumThreads = 2;
  const db = new Module.Database(":memory:", config);
  const conn = new Module.Connection(db);

  const r = conn.query("RETURN 'hello' AS msg");
  console.log(r.getAsJsArrayOfObjects());
  r.delete();

  conn.delete();
  db.delete();
</script>
```

**Important** : les appels WASM sont synchrones et bloquent le thread. Il faut les executer dans un **Web Worker dedie** pour eviter de bloquer le main thread (voir `tools/wasm/test/browser/worker.js` pour un exemple).

### Tests Playwright (IDBFS persistence)

```bash
cd tools/wasm
npm install
npx playwright test --reporter=line
```

Resultat attendu : `2 passed`

Phase 1 : create DB + mount IDBFS + insert docs + tantivy index + vector index + query + syncfs → sauve dans IndexedDB.
Phase 2 : mount IDBFS + syncfs → reload + requery → memes resultats.

Pour voir le navigateur :

```bash
npx playwright test --headed
```

---

## 5. Build WASM NODEFS (tests Node.js)

Variante WASM qui utilise le filesystem Node.js directement. Separee en .js + .wasm.

```bash
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

Sortie : `tools/wasm/build/rag3db/rag3db_wasm.js` (~239K) + `rag3db_wasm.wasm` (~15MB)

### Copier vers package/nodejs et lancer les tests

```bash
cp tools/wasm/build/rag3db/rag3db_wasm.* tools/wasm/package/nodejs/rag3db/
cd tools/wasm
npm test
```

Resultat attendu : `94 passing`

### Attention : conflit de sortie

Les builds standard et NODEFS ecrivent dans le MEME repertoire (`tools/wasm/build/rag3db/`). Supprimer avant de changer de variante :

```bash
rm -f tools/wasm/build/rag3db/rag3db_wasm.*
```

Pour identifier quel build est en place : ~17MB `.js` seul = standard. ~240K `.js` + 15MB `.wasm` = NODEFS.

---

## 6. Persistence IDBFS (browser)

Pattern pour persister une base rag3db dans IndexedDB :

```javascript
// --- Creation et sauvegarde ---
const Module = await rag3db();

// Monter IDBFS
Module.FS.mkdir("/database");
Module.FS.mount(Module.FS.filesystems.IDBFS, {}, "/database");

// Creer la DB SOUS le point de montage
// (les tantivy indexes vont dans parent_path(dbPath)/tantivy_indexes/,
//  donc le path de la DB doit etre un sous-dossier du montage IDBFS)
const config = new Module.SystemConfig();
config.maxNumThreads = 2;
const db = new Module.Database("/database/mydb", config);
const conn = new Module.Connection(db);

// Creer tables, index, inserer des donnees...
conn.query("CREATE NODE TABLE docs (id UINT64, body STRING, PRIMARY KEY(id))");
conn.query("CALL CREATE_TANTIVY_INDEX('docs', ['body'])");

// Fermer AVANT de sync
conn.delete();
db.delete();

// Sauvegarder dans IndexedDB
await new Promise((resolve, reject) => {
  Module.FS.syncfs(false, (err) => err ? reject(err) : resolve());
});
Module.FS.unmount("/database");


// --- Rechargement (apres refresh de page) ---
const Module = await rag3db();
Module.FS.mkdir("/database");
Module.FS.mount(Module.FS.filesystems.IDBFS, {}, "/database");

// Charger depuis IndexedDB
await new Promise((resolve, reject) => {
  Module.FS.syncfs(true, (err) => err ? reject(err) : resolve());
});

// Rouvrir — tout est la (donnees + tantivy index + vector HNSW)
const db = new Module.Database("/database/mydb", config);
const conn = new Module.Connection(db);
```

**Piege courant** : si la DB est creee directement a `/database` (sans sous-dossier), les tantivy indexes sont ecrits dans `/tantivy_indexes/` qui est EN DEHORS du montage IDBFS et ne sont pas persistes.

---

## 7. Headers serveur (COOP/COEP)

Le WASM utilise `SharedArrayBuffer` (pthreads). Les navigateurs exigent ces headers :

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

---

## Recapitulatif

| Build | Commande de test | Resultat |
|-------|------------------|----------|
| Rust (ld-tantivy) | `cargo test --lib` | 1015 pass |
| Natif GTest E2E | `./tantivy_fts_test` | 9 pass |
| Node.js natif mocha | `npm test` (tools/nodejs_api) | 139 pass |
| WASM NODEFS mocha | `npm test` (tools/wasm) | 94 pass |
| WASM browser Playwright | `npx playwright test` (tools/wasm) | 2 pass (8 sub-tests) |
