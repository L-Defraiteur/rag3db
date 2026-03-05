# Rag3Weaver — Plan Étape 4 : Playwright / WASM browser (15 février 2026)

Date : 15 février 2026
Statut : plan / interrogations avant implémentation

---

## Ce qui a été fait dans cette session (docs 16-17)

1. **Rag3dbConnection** (doc 16) — bridge natif vers rag3db, self-referential struct, 7+6 tests
2. **CypherPersistence** (doc 16) — persistance queue via Cypher, 7+9 tests
3. **Tests E2E natif** (doc 17) — `tests/e2e_native.rs`, 11 tests avec vraie DB in-memory
4. **Compilation WASM** (doc 17) — `cargo check --target wasm32-unknown-unknown` OK (fix getrandom wasm_js)
5. **CallbackConnection** (cette session) — implémentation générique DbConnection par closures, 4 tests, re-exporté dans lib.rs

## État actuel

- 271 tests réguliers (default features), 275 avec rag3db-native
- 3 configs validées : natif default, natif rag3db, WASM check
- `CallbackConnection` prêt — même pattern que `CallbackEmbedder`
- `.cargo/config.toml` avec rustflags getrandom pour wasm32

---

## Étape 4 : Playwright / WASM browser

### But

Prouver que rag3weaver fonctionne dans un browser via rag3db_wasm.js.

### Architecture existante (tools/wasm/)

L'infrastructure Playwright est déjà en place :
- `test/browser/serve.js` — serveur HTTP port 3333, headers COOP/COEP pour SharedArrayBuffer
- `test/browser/index.html` — page qui spawn un Web Worker
- `test/browser/worker.js` — charge `rag3db_wasm.js` (emscripten), exécute les opérations
- `test/browser/idbfs.spec.js` — test Playwright (Phase 1: create+persist, Phase 2: reload+verify)
- `playwright.config.js` — headless chromium, timeout 120s

Le WASM build existe : `package/nodejs/rag3db/rag3db_wasm.js` + `.wasm` (17MB, lucivy_fts linké statiquement).

API JS synchrone (dans le worker) :
```js
const db = new Module.Database(path, config);
const conn = new Module.Connection(db);
const result = conn.query(cypher);        // sync
result.getAsJsArrayOfObjects();           // [{col1: val, ...}]
conn.delete(); db.delete();
```

### Deux approches possibles

#### Approche A : Test Playwright pur JS (sans compiler rag3weaver en WASM)

Écrire un worker JS qui reproduit le pipeline rag3weaver :
1. Crée le schema (même DDL que `generate_full_schema`)
2. Insère des documents (même Cypher que `InsertProcessor`)
3. Crée des relations (même Cypher que `LinkProcessor`)
4. Requête les données (MATCH, count, exists)
5. Optionnel : test FTS via QUERY_LUCIVY_INDEX + vector via QUERY_VECTOR_INDEX

**Avantage** : simple, valide que le Cypher généré par rag3weaver est compatible WASM
**Inconvénient** : ne teste pas rag3weaver Rust lui-même en WASM

#### Approche B : Compiler rag3weaver en WASM (wasm-pack + wasm-bindgen)

Créer `src/wasm_bindings.rs` avec des exports `#[wasm_bindgen]` :
- `JsCatalog` wrappant `Catalog`
- Callbacks JS → Rust via `js_sys::Function` + `wasm_bindgen_futures::JsFuture`
- Bridge : rag3weaver WASM ↔ JS glue ↔ rag3db_wasm.js

**Problèmes identifiés** :
1. **Deux modules WASM** : rag3weaver (wasm-bindgen) + rag3db (emscripten) = deux runtimes séparés
2. **Async cross-boundary** : `DbConnection` est async, les callbacks JS doivent retourner des Promises, converties via `JsFuture`
3. **Send + Sync** : `JsValue` est `!Send`, mais le trait `DbConnection: Send + Sync`. Solution : `unsafe impl Send/Sync` (sound car WASM est single-threaded)
4. **Sérialisation** : `CypherValue` ↔ `JsValue` mapping nécessaire
5. **Build pipeline** : wasm-pack + intégration avec le serveur de test existant

### Interrogations ouvertes

1. **Approche A ou B ?** — A est faisable maintenant, B est plus ambitieux mais teste vraiment rag3weaver en WASM
2. **Si B : wasm-pack ou wasm-bindgen CLI ?** — wasm-pack est plus simple (gère le bundling)
3. **Si B : comment bridger deux modules WASM ?** — Le worker charge rag3db_wasm.js ET rag3weaver.wasm, puis connecte les deux via JS
4. **CallbackConnection suffit-il pour B ?** — Oui, mais il faudra un adapter `JsCallbackConnection` qui convertit `JsValue` ↔ `QueryResult`
5. **Faut-il un feature flag `wasm` ?** — Probablement, pour gater wasm-bindgen et les bindings JS
6. **IDBFS** : rag3weaver n'a rien à gérer côté storage (c'est rag3db_wasm.js qui le fait). Mais faut-il tester la persistance cross-session ?

### Plan recommandé (approche hybride)

**Phase 1 (immédiat) — Approche A** : Test Playwright JS qui valide le Cypher
- Nouveau fichier : `test/browser/rag3weaver_worker.js`
- Nouveau fichier : `test/browser/rag3weaver.spec.js`
- Génère le même DDL que rag3weaver, insère des données, vérifie
- Prouve : schema + insert + link + query fonctionnent en WASM browser

**Phase 2 (futur) — Approche B** : wasm-bindgen bindings
- Ajout `wasm-bindgen`, `js-sys`, `wasm-bindgen-futures` en deps optionnelles
- Feature `wasm-bindings` gating le module
- `src/wasm_bindings.rs` : JsCatalog, JsCallbackConnection
- Build avec wasm-pack, intégration Playwright

---

## Fichiers créés/modifiés dans cette session

| Fichier | Action |
|---------|--------|
| `src/connection.rs` | Modifié — ajout `CallbackConnection` + `DbExecuteFn` + 4 tests |
| `src/lib.rs` | Modifié — re-export `CallbackConnection` |
| `tests/e2e_native.rs` | Créé (session précédente) — 11 tests E2E |
| `Cargo.toml` | Modifié (session précédente) — getrandom WASM |
| `.cargo/config.toml` | Créé (session précédente) — rustflags WASM |
