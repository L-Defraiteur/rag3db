# Rag3Weaver — Résumé session 15 février 2026

Date : 15 février 2026
Statut : résumé post-session

---

## Ce qui a été fait dans cette session

### 1. Rag3dbConnection (feature `rag3db-native`)

**Fichier** : `src/rag3db_connection.rs`

Bridge natif vers le moteur rag3db en process. Implémente `DbConnection`.

- **Self-referential struct** : `Box<Database>` + `Connection<'static>` (unsafe transmute du lifetime). `conn` déclaré avant `db` dans le struct pour garantir l'ordre de drop.
- **Constructeurs** : `new(path)`, `in_memory()`, `with_config(path, config)`
- **Mapping rag3db::Value → CypherValue** :
  - Int64/32/16/8, UInt64/32/16/8, Int128 → `CypherValue::Int(i64)`
  - Double, Float → `CypherValue::Float(f64)`
  - String → String, Bool → Bool, Null → Null
  - List/Array → List (récursif)
  - Node → Map avec `_label`, `_id`, + properties
  - Rel → Map avec `_label`, `_src`, `_dst`, + properties
  - Struct → Map
  - Map → Map (clé string)
  - Fallback (Date, Timestamp, etc.) → `String(format!("{}"))`
- **Mapping CypherValue → rag3db::Value** (pour prepared statements) :
  - Int→Int64, Float→Double, String→String, Bool→Bool, Null→Null(String)
  - List → List avec LogicalType inféré du premier élément
  - Map → Struct
- **Build pre-built** : `RAG3DB_SHARED=1 RAG3DB_LIBRARY_DIR=.../build/release/src RAG3DB_INCLUDE_DIR=.../build/release/src LD_LIBRARY_PATH=.../build/release/src`
- **7 tests unitaires** (mapping types) + **6 tests intégration** (#[ignore]) avec DB in-memory

**Cargo.toml** :
```toml
rag3db = { path = "../../tools/rust_api", optional = true }
[features]
rag3db-native = ["dep:rag3db"]
```

### 2. CypherPersistence

**Fichier** : `src/cypher_persistence.rs`

Implémentation concrète du trait `OperationPersistence`. Stocke les opérations de la queue dans une table `_Operation` via Cypher, compatible avec tout `DbConnection` (natif, Node.js, WASM).

- **Table `_Operation`** : uuid (PK), op_type, priority, state, temp_uuid, entity_name, payload (JSON), depends_on (STRING[]), created_at, error
- **`ensure_table()`** : vérifie via `CALL show_tables()`, crée si absente, flag `table_ready`
- **`persist(item)`** : génère UUID déterministe via `content_hash("op-{type}-{id}-{created_at}")`, sérialise le payload JSON, INSERT dans _Operation, retourne UUID
- **`update_state(uuid, state, error)`** : MATCH + SET state/error
- **`mark_completed(uuid)`** : raccourci → state='completed'
- **`cleanup_old_completed(retention_ms)`** : COUNT puis DELETE WHERE completed AND old
- **`load_for_recovery()`** : MATCH WHERE state IN (persisted, failed) ORDER BY created_at
- **`reset_processing_items()`** : SET state='persisted' WHERE state='processing'
- **Sérialisation payload** :
  - InsertOp → `serde_json::to_string(&data)` (HashMap<String, CypherValue>)
  - LinkOp → JSON avec rel_name, from, to, properties
  - EmbedOp → JSON avec kb_name, texts
- **7 tests unitaires** + **9 tests intégration** (#[ignore]) avec Rag3dbConnection::in_memory()

### 3. Documentation

- **Doc 15** (`15-rag3weaver-persistence-e2e-wasm-plan.md`) : plan des 4 prochaines étapes (CypherPersistence ✓, E2E Catalog, WASM check, Playwright)
- **Doc 16** (ce fichier) : résumé session

---

## État actuel du crate

- **23 modules** : +2 (rag3db_connection, cypher_persistence)
- **274 tests réguliers** (sans ignore)
- **20 tests intégration** (#[ignore]) : 5 candle + 6 rag3db_connection + 9 cypher_persistence
- **Features** : `candle-embedder` (default), `rag3db-native` (opt-in)
- **Tout vert** sur les deux configs (avec et sans rag3db-native)

## Architecture 3 targets

| Cible | DbConnection | Embedder | Status |
|-------|-------------|----------|--------|
| Natif Rust | `Rag3dbConnection` ✓ | `CandleEmbedder` ✓ | Fonctionnel |
| Node.js | Même via napi-rs | Callback | À faire |
| WASM browser | `CallbackConnection` | Callback | À faire |

`CypherPersistence` fonctionne sur les 3 targets (passe par `DbConnection`).

## Prochaines étapes (doc 15)

1. ~~CypherPersistence~~ ✓ FAIT
2. **Test E2E Catalog + Rag3dbConnection** — pipeline complet (config → schema → insert → search)
3. **Vérification compilation WASM** — `cargo check --target wasm32-unknown-unknown --no-default-features`
4. **Playwright / WASM browser** — test bout-en-bout avec rag3db_wasm.js

## Points techniques à retenir

- **Pre-built rag3db** : utiliser `RAG3DB_SHARED=1` + `LD_LIBRARY_PATH` pour éviter la recompilation CMake (~5-10 min). Les headers sont dans `build/release/src/` (pas `src/include/main/` — le single-file header `rag3db.hpp` est généré dans le build dir).
- **IDBFS** : rag3db WASM utilise déjà IDBFS pour la persistance. Pattern : `mountIdbfs` → `syncfs(true)` pour charger, `syncfs(false)` pour sauver. Tests Playwright existants dans `tools/wasm/test/browser/idbfs.spec.js`.
- **Self-referential struct** : transmute du lifetime `'a` → `'static` sur Connection. Sound car Box<Database> est heap-allocated et conn est drop avant db.
