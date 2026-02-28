# Rapport — Build WASM rag3db + tantivy_fts : VALIDÉ

## Ce qui a été fait

### 1. Installation emscripten

```bash
cd ~
git clone https://github.com/emscripten-core/emsdk.git
cd emsdk && ./emsdk install latest && ./emsdk activate latest
# Version: emscripten 5.0.1
```

Prérequis Rust (déjà installés) :
- `rustup install nightly`
- `rustup component add rust-src --toolchain nightly`
- `rustup target add wasm32-unknown-emscripten --toolchain nightly`

### 2. Bugs corrigés pendant le build

#### Bug 1 : `DOC_FREQUENCY_PROP_NAME` non déclaré (extension fts originale)
- **Fichier** : `extension/fts/src/function/query_fts_index.cpp`
- **Problème** : constexpr déclarée ligne 370 mais utilisée ligne 325
- **Fix** : déplacé les 5 `static constexpr` avant la fonction `getDFsFuzzy()`

#### Bug 2 : `cannot use 'throw' with exceptions disabled` (cxx bridge)
- **Problème** : cc-rs compile le bridge C++ avec `-fno-exceptions` par défaut sur emscripten
- **Fix** : modifié `tantivy_fts/rust/build.rs` pour ajouter `-fexceptions -sDISABLE_EXCEPTION_CATCHING=0` quand target contient "emscripten"

#### Bug 3 : `--shared-memory disallowed` (atomics manquants)
- **Problème** : Rust compilé sans `+atomics,+bulk-memory` mais rag3db WASM utilise pthreads (shared memory)
- **Fix** : ajouté `RUSTFLAGS=-C target-feature=+atomics,+bulk-memory,+mutable-globals` et `cargo +nightly build -Z build-std=std,panic_abort`

#### Bug 4 : `undefined symbol: __cpp_exception` (conflit modèles exceptions)
- **Problème** : Rust utilisait les WASM native exceptions, rag3db utilise le modèle JS d'emscripten
- **Fix** : ajouté `-C panic=abort` aux RUSTFLAGS pour supprimer le exception handling Rust

#### Bug 5 : `fuzzy_fst` non compilé pour WASM (extension fts originale)
- **Problème** : `libfuzzy_fst.a` compilé en x86_64, pas en wasm32
- **Fix** : retiré `fts` de la liste static link WASM (tantivy_fts la remplace)

### 3. Fichiers modifiés

| Fichier | Modification |
|---------|-------------|
| `extension/fts/src/function/query_fts_index.cpp` | Déplacé constexpr avant usage |
| `extension/tantivy/ld-tantivy/tantivy_fts/rust/build.rs` | `-fexceptions` pour emscripten |
| `extension/tantivy_fts/CMakeLists.txt` | nightly + atomics + build-std + panic=abort |
| `extension/extension_config.cmake` | Retiré `fts` de la liste WASM |

### 4. Commandes de build

```bash
# Configure
cd packages/rag3db/build/wasm
source ~/emsdk/emsdk_env.sh
emcmake cmake ../.. -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_WASM=TRUE -DBUILD_SHELL=FALSE \
  -DBUILD_TESTS=FALSE -DBUILD_BENCHMARK=FALSE

# Build (~8 min sur 16 cores)
emmake cmake --build . -j$(nproc)
```

### 5. Outputs

- `tools/wasm/build/rag3db/rag3db_wasm.js` — 17MB, contient le WASM inline (single-file)
- Extensions statiquement linkées : json, vector, algo, tantivy_fts

### 6. Tests — TOUS PASSENT

```
=== Test rag3db WASM + tantivy_fts ===
Loading WASM module...
WASM module loaded
Database created
Connection created
Table created: true
Doc 0 inserted: true
Doc 1 inserted: true
Doc 2 inserted: true
Tantivy index created: true

--- Test 1: Contains query ---
Results: 2 (expected 2)
  node_id=0, score=1
  node_id=2, score=1

--- Test 2: Fuzzy query ---
Results: 1 (expected 1)
  node_id=0, score=1

--- Test 3: Phrase query ---
Results: 1 (expected 1)
  node_id=0, score=1.5341556072235107

=== Tests done! ===
```

### 7. Usage depuis Node.js (WASM)

```javascript
const initRag3db = require('./tools/wasm/build/rag3db/rag3db_wasm.js');
const module = await initRag3db();

const config = new module.SystemConfig();
const db = new module.Database(':memory:', config);
const conn = new module.Connection(db);

// tantivy_fts auto-chargé (static link), pas besoin de LOAD EXTENSION
conn.query("CREATE NODE TABLE doc (ID UINT64, title STRING, body STRING, PRIMARY KEY (ID))");
conn.query("CREATE (:doc {ID: 0, title: 'Rust', body: 'Rust is a systems programming language'})");
conn.query("CALL CREATE_TANTIVY_INDEX('doc', ['title', 'body'])");

const result = conn.query(`
  CALL QUERY_TANTIVY_INDEX('doc',
    '{"type":"fuzzy","field":"body","value":"programing","distance":1}', 10)
  RETURN node_id, score
`);
const rows = result.getAsJsArrayOfObjects();
// [{node_id: 0, score: 1}]
```

## Différence WASM vs Node.js natif

| Aspect | WASM | Node.js natif |
|--------|------|---------------|
| tantivy_fts | Static link (auto-chargé) | Dynamic link (`LOAD EXTENSION`) |
| Threads Rust | nightly + build-std + atomics | Natif (full threads) |
| Taille | 17MB .js (inline wasm) | ~50MB .node |
| API | `module.Database()`, `module.Connection()` | `rag3db.Database()`, `rag3db.Connection()` |
| Init | `await initRag3db()` puis sync | `await db.init()` puis async |
| Fichier index | In-memory seulement (pas de fs) | In-memory + fichier |

## Notes techniques

- Le warning `-pthread + ALLOW_MEMORY_GROWTH may run non-wasm code slowly` est normal et documenté
- L'extension `fts` originale a été retirée du build WASM (tantivy_fts la remplace)
- Le mode single-file (WASM inline dans le .js) est le défaut ; pour séparer, ajouter `-sSINGLE_FILE=0`
- Rayon (multi-thread Rust) fonctionne grâce à `-Z build-std` + atomics (nightly requis)
