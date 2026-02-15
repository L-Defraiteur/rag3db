# Rag3Weaver — Persistence, E2E, WASM (15 février 2026)

Date : 15 février 2026
Statut : plan / prochaines étapes

---

## Ce qui est fait

### Session précédente (doc 12-13)
- BM25 NgramContains (`contains:`/`regex:` JSON QueryConfig, BM25Mode enum)
- CallbackEmbedder (EmbedFn type alias, trait object compatible)
- CandleEmbedder intégré (feature `candle-embedder`, DefaultModel::BgeBase par défaut)
- 3 exemples fonctionnels (tei_reqwest, tei_openai, candle_local)

### Session actuelle (doc 14 → maintenant)
- **Rag3dbConnection** (feature `rag3db-native`) — bridge vers le moteur rag3db en process
  - Self-referential struct (`Box<Database>` + `Connection<'static>`, unsafe transmute)
  - Mapping complet `rag3db::Value` ↔ `CypherValue` (int/float/string/bool/null/list/node→map/rel→map/struct→map)
  - Prepared statements via `execute_with_params`
  - Fonctionne comme `Box<dyn DbConnection>`
  - Build : `RAG3DB_SHARED=1 RAG3DB_LIBRARY_DIR=.../build/release/src RAG3DB_INCLUDE_DIR=.../build/release/src`
  - **267 tests réguliers + 6 intégration rag3db** — tout vert

### État actuel
- 22 modules, 267 tests (+ 11 ignorés : 5 candle, 6 rag3db)
- Features : `candle-embedder` (default), `rag3db-native` (opt-in)
- Trait `OperationPersistence` défini (6 méthodes) mais **pas d'implémentation concrète**
- rag3db WASM utilise **IDBFS** (IndexedDB) pour la persistance — déjà fonctionnel et testé (idbfs.spec.js)

---

## Objectif global

rag3weaver doit fonctionner dans 3 environnements :

| Cible | DbConnection | Embedder | Persistance |
|-------|-------------|----------|-------------|
| **Natif Rust** | `Rag3dbConnection` (feature `rag3db-native`) | `CandleEmbedder` ou callback | Table `_Operation` via Cypher |
| **Node.js** | Même `Rag3dbConnection` via napi-rs | Callback vers TEI/OpenAI | Table `_Operation` via Cypher |
| **WASM browser** | `CallbackConnection` (JS → rag3db_wasm.js) | Callback vers API | Table `_Operation` via Cypher |

Le code métier (catalog, search, queue, chunker, fusion...) est **identique** dans les 3 cas. Seules les implémentations de `DbConnection` et `Embedder` changent.

---

## Étapes immédiates

### Étape 1 : Implémentation concrète `CypherPersistence`

**But** : Implémenter `OperationPersistence` avec du Cypher via `DbConnection`. Crée une table `_Operation` et persiste les items de la queue.

**Fichier** : `src/cypher_persistence.rs`

**Schema de la table `_Operation`** :
```cypher
CREATE NODE TABLE _Operation(
    uuid STRING,
    op_type STRING,
    priority INT64,
    state STRING,
    temp_uuid STRING,
    entity_name STRING,
    payload STRING,
    depends_on STRING[],
    created_at INT64,
    error STRING,
    PRIMARY KEY(uuid)
);
```

**Méthodes à implémenter** :
1. `persist(item)` — INSERT de l'item, retourne uuid
2. `update_state(uuid, state, error)` — SET state + error
3. `mark_completed(uuid)` — SET state='completed'
4. `cleanup_old_completed(retention_ms)` — DELETE WHERE state='completed' AND created_at < now - retention
5. `load_for_recovery()` — MATCH WHERE state IN ['persisted','failed'] RETURN *
6. `reset_processing_items()` — SET state='persisted' WHERE state='processing'

**Constructor** : `CypherPersistence::new(conn: Arc<dyn DbConnection>)` + `ensure_table()` pour créer la table si absente.

**Tests** : Avec `Rag3dbConnection::in_memory()` — vrais tests E2E, pas mock.

### Étape 2 : Test E2E Catalog + Rag3dbConnection

**But** : Valider le pipeline complet avec une vraie DB :
1. Charger un `CatalogConfig` (JSON)
2. Créer le schema via `generate_full_schema()` + `execute()`
3. Insérer des documents (entités + relations)
4. Rechercher (BM25 + vector hybrid)
5. Vérifier les résultats

**Fichier** : `tests/e2e_native.rs` (test d'intégration cargo, `#[ignore]`)

**Dépendances** : `Rag3dbConnection` + `CandleEmbedder` (ou `CallbackEmbedder` avec vecteurs fixes pour la reproductibilité)

### Étape 3 : Vérifier compilation WASM

**But** : S'assurer que rag3weaver compile pour `wasm32-unknown-unknown` sans les features natives.

```bash
cargo check --target wasm32-unknown-unknown --no-default-features
```

**Problèmes potentiels** :
- `tokio::sync` (Mutex/RwLock) — devrait compiler car ce sont des impls sans runtime
- `async-broadcast` — à vérifier
- `blake3` — a un mode WASM/fallback
- `text-splitter` — pure Rust, devrait passer

Si ça ne compile pas, identifier les deps problématiques et les mettre derrière des feature flags.

### Étape 4 : Playwright / WASM browser

**But** : Test bout-en-bout dans un vrai browser :
1. rag3weaver compilé en WASM (via wasm-pack ou wasm-bindgen)
2. rag3db_wasm.js chargé comme d'habitude
3. `CallbackConnection` qui bridgue JS → rag3db_wasm.js
4. Pipeline complet : config → create schema → insert → search

**Pattern IDBFS** (déjà existant dans rag3db) :
```js
await rag3db.FS.mkdir("/database");
await rag3db.FS.mountIdbfs("/database");
await rag3db.FS.syncfs(true);  // charger depuis IndexedDB
const db = new rag3db.Database("/database");
// ... utiliser rag3weaver ...
await db.close();
await rag3db.FS.syncfs(false); // sauver vers IndexedDB
```

**Fichier** : `tools/wasm/test/browser/rag3weaver.spec.js` (Playwright)

---

## Décisions prises

- **Architecture 3 targets** : même code métier, impls DbConnection/Embedder différentes
- **Feature flags** : `rag3db-native` pour natif/Node.js, rien pour WASM (CallbackConnection)
- **DefaultModel** : BgeBase (768 dims) par défaut
- **Persistance** : Via Cypher (table `_Operation`), fonctionne sur les 3 targets puisqu'elle passe par `DbConnection`
- **IDBFS** : rag3db WASM le gère déjà, rag3weaver n'a rien à faire côté stockage

## Décisions ouvertes

- **Node.js binding** : napi-rs (compile rag3weaver + rag3db natif en .node) vs JS wrapper pur ?
- **WASM packaging** : wasm-pack + wasm-bindgen vs emscripten (comme rag3db) ?
- **Queue flush timing** : flush auto dans le browser (avant `syncfs(false)`) ou explicite ?
- **Extension tantivy_fts** : tester le chargement via `LOAD EXTENSION` depuis Rag3dbConnection (nécessite `-rdynamic` dans le build)
