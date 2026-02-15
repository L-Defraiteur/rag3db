# Rag3Weaver — Single WASM : Phase 2 terminée (15 février 2026)

Date : 15 février 2026
Statut : Phase 2 FAIT, Phase 3 en cours

---

## Résumé

Phase 2 (intégration build + embind) est complète. Le build WASM produit un seul
binaire `rag3db_wasm.js` (17 MB) qui inclut rag3weaver statiquement linké.
La classe JS `Weaver` est exposée via emscripten embind.

## Ce qui a été fait

### Phase 1 : Compilation Rust WASM — FAIT (session précédente)

- `src/wasm_ffi.rs` — ~500 lignes, C API FFI + WasmDbConnection + entry points
- Feature `wasm-emscripten = ["dep:pollster"]`
- 3 targets compilent : natif (271 tests OK), wasm32-unknown-unknown, wasm32-unknown-emscripten

### Phase 2 : Intégration build + embind — FAIT

1. **`extension/rag3weaver/CMakeLists.txt`** — créé
   - Guard `if(NOT EMSCRIPTEN) return() endif()`
   - `cargo +nightly build --release --target wasm32-unknown-emscripten`
   - Flags : `-Z build-std=std,panic_abort`, atomics, bulk-memory, exceptions
   - Produit `librag3weaver.a` en static lib importée (`rag3weaver_lib`)

2. **`Cargo.toml`** — modifié
   - Ajout `crate-type = ["lib", "staticlib"]` dans `[lib]`
   - Nécessaire pour que cargo produise un `.a` (pas juste `.rlib`)

3. **`tools/wasm/src_cpp/weaver_bindings.cpp`** — créé
   - `extern "C"` pour les 5 entry points Rust (version, catalog_new, destroy, create, drain, count)
   - Classe C++ `Weaver` : constructeur, create, drain, count, version (static)
   - `EMSCRIPTEN_BINDINGS(rag3weaver_wasm)` expose la classe à JS

4. **`tools/wasm/CMakeLists.txt`** — modifié
   - `add_subdirectory(${PROJECT_SOURCE_DIR}/extension/rag3weaver ...)`
   - Ajout `src_cpp/weaver_bindings.cpp` aux sources
   - `target_link_libraries(rag3db_wasm PRIVATE rag3db rag3weaver_lib)`

5. **Build WASM** — réussi
   ```
   [100%] Built target rag3db_wasm
   ```
   - `rag3db_wasm.js` : 17 MB
   - Classe `Weaver` confirmée dans le binaire (via `strings | grep Weaver`)

### Bug corrigé

- **`.rlib` au lieu de `.a`** : cargo ne produit pas de `.a` sans `crate-type = ["staticlib"]`
  dans `[lib]`. Le premier build échouait avec "Aucune règle pour fabriquer la cible librag3weaver.a".
  Fix : ajout `crate-type = ["lib", "staticlib"]` dans Cargo.toml.

---

## Phase 3 : Tests Playwright — EN COURS

### Architecture des tests browser existants

Le pattern en place (pour IDBFS/tantivy_fts) :

```
test/browser/
├── serve.js          — HTTP serveur (port 3333), COOP/COEP headers
├── index.html        — page qui crée un Worker, dispatch par ?phase=N
├── worker.js         — Worker qui charge rag3db_wasm.js, exécute les tests
└── idbfs.spec.js     — test Playwright (page.goto → waitForFunction → check results)
```

`playwright.config.js` : lance `serve.js` comme webServer, cherche `*.spec.js` dans `test/browser/`.

### Plan Phase 3

Créer un test dédié pour Weaver, même pattern :

1. **`test/browser/weaver_worker.js`** — nouveau Worker dédié
   - Charge `rag3db_wasm.js`
   - Crée un `new Module.Weaver(configJson, "")` (in-memory)
   - Test 1 : `Weaver.version()` retourne "0.1.0"
   - Test 2 : `weaver.create("Document", ...)` × 3 → chacun retourne `{"uuid":"..."}`
   - Test 3 : `weaver.drain()` → `{"processed":3,"failed":0,...}`
   - Test 4 : `weaver.count("Document")` → `{"count":3}`
   - Test 5 : vérifier via Connection directe que les nœuds existent dans rag3db

2. **`test/browser/weaver.html`** — page dédiée
   - Crée le Worker, collecte les résultats dans `window.testResults`

3. **`test/browser/rag3weaver.spec.js`** — test Playwright
   - `page.goto("/weaver.html")`
   - Attend `window.testResults.done === true`
   - Vérifie version, creates, drain stats, count

4. **`test/browser/serve.js`** — modifier pour servir aussi `weaver.html` et `weaver_worker.js`
   - En fait, `serve.js` sert déjà tous les fichiers de `STATIC_DIR` (= `test/browser/`)
   - Donc aucune modification nécessaire !

### Config JSON pour le test

```json
{
  "name": "test-weaver",
  "entities": {
    "Document": {
      "fields": {
        "title": { "fieldType": "String" },
        "body": { "fieldType": "Text" }
      }
    }
  },
  "embeddingDim": 4
}
```

`embeddingDim: 4` car MockEmbedder (zéro vectors) — on teste le pipeline, pas la qualité des embeddings.

---

## Fichiers créés/modifiés (cette session)

| Fichier | Action |
|---------|--------|
| `tools/wasm/src_cpp/weaver_bindings.cpp` | Créé — embind Weaver class |
| `tools/wasm/CMakeLists.txt` | Modifié — add_subdirectory + source + link |
| `Cargo.toml` | Modifié — crate-type = ["lib", "staticlib"] |
| `docs/.../21-rag3weaver-single-wasm-phase2-done.md` | Créé — ce document |

## API JS exposée

```js
// Construction (config JSON + path, "" = in-memory)
const weaver = new Module.Weaver('{"name":"my-kb","entities":{...}}', "");

// Créer une entité (retourne JSON string)
const result = weaver.create("Document", '{"title":"Hello","body":"World"}');
// → '{"uuid":"abc-123..."}'

// Flush la queue d'opérations
const stats = weaver.drain();
// → '{"processed":1,"failed":0,"pending":0}'

// Compter les entités
const count = weaver.count("Document");
// → '{"count":1}'

// Version statique
const version = Module.Weaver.version();
// → "0.1.0"

// Destructor automatique via C++ destructor (pas de .delete() nécessaire)
// Mais en pratique, appeler weaver.delete() pour libérer immédiatement
```
