# Rag3Weaver — E2E natif + vérification WASM (15 février 2026)

Date : 15 février 2026
Statut : résumé post-session (étapes 2 et 3 du plan doc 15)

---

## Ce qui a été fait

### Étape 2 : Tests E2E Catalog + Rag3dbConnection

**Fichier** : `tests/e2e_native.rs`

11 tests d'intégration avec une vraie DB rag3db in-memory. Config sans knowledge bases (pas besoin des extensions vector/lucivy_fts). Teste le pipeline CRUD complet.

**Config de test** :
- Entités : `Document` (title STRING, body TEXT, page_count INT64, hashsafe sur title), `Author` (name STRING, hashsafe sur name)
- Relation : `WRITTEN_BY` (Document → Author)
- Pas de KBs → pas d'embedding columns, pas d'indexes vector/FTS

**Tests** :
1. `e2e_initialize_creates_schema` — DDL exécuté, tables créées, count == 0
2. `e2e_create_drain_count` — 3 inserts, drain, count == 3
3. `e2e_create_drain_get` — insert, drain, get par UUID, vérification title/body/page_count
4. `e2e_create_drain_exists` — exists true sur UUID réel, false sur UUID bidon
5. `e2e_hashsafe_deterministic` — même titre → même UUID entre deux catalogs séparés
6. `e2e_create_link_drain` — Document + Author + WRITTEN_BY, drain 3 ops, refs résolues
7. `e2e_update_document` — create, drain, update body+page_count, vérification via get
8. `e2e_delete_document` — create, drain, delete, exists false, count == 0
9. `e2e_get_many` — 3 docs, get_many par liste d'UUIDs
10. `e2e_full_pipeline` — 5 entités + 4 relations + update + delete en séquence
11. `e2e_update_not_found` — erreur NotFound sur UUID inexistant

**Résultat** : 11/11 verts.

**Commande** :
```bash
RAG3DB_SHARED=1 \
  RAG3DB_LIBRARY_DIR=.../build/release/src \
  RAG3DB_INCLUDE_DIR=.../build/release/src \
  LD_LIBRARY_PATH=.../build/release/src \
  cargo test --features rag3db-native --test e2e_native -- --ignored
```

**Point technique** : `catalog.get()` retourne `{"n": Map({_label, _id, _uuid, _content_hash, ...properties})}`. Le nœud rag3db est mappé en `CypherValue::Map` avec `_label` et `_id` en plus des propriétés utilisateur.

### Étape 3 : Vérification compilation WASM

**But** : `cargo check --target wasm32-unknown-unknown --no-default-features` doit compiler.

**Problème** : `getrandom` 0.3.4 (tiré par `text-splitter` → `ahash` → `getrandom`) ne supporte pas `wasm32-unknown-unknown` par défaut. Erreur : *"The wasm32-unknown-unknown targets are not supported by default"*.

**Fix (2 fichiers)** :

1. **`Cargo.toml`** — dépendance conditionnelle WASM :
```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
getrandom = { version = "0.3", features = ["wasm_js"] }
```

2. **`.cargo/config.toml`** — cfg flag pour le backend getrandom :
```toml
[target.wasm32-unknown-unknown]
rustflags = ["--cfg", "getrandom_backend=\"wasm_js\""]
```

**Résultat** : compilation WASM OK, tests natifs intacts.

---

## État actuel du crate

- **23 modules** (inchangé)
- **274 tests réguliers** (avec rag3db-native), **267 sans**
- **31 tests ignorés** : 5 candle + 6 rag3db_connection + 9 cypher_persistence + 11 e2e_native
- **3 configs validées** :

| Cible | Commande | Status |
|-------|----------|--------|
| Natif (default) | `cargo test` | 267 tests ✓ |
| Natif (rag3db) | `cargo test --features rag3db-native` | 274 tests ✓ |
| WASM | `cargo check --target wasm32-unknown-unknown --no-default-features` | Compile ✓ |

## Fichiers créés/modifiés

| Fichier | Action |
|---------|--------|
| `tests/e2e_native.rs` | Créé — 11 tests E2E |
| `Cargo.toml` | Modifié — ajout getrandom WASM |
| `.cargo/config.toml` | Créé — rustflags WASM |

## Prochaines étapes (doc 15)

1. ~~CypherPersistence~~ ✓ (doc 16)
2. ~~Test E2E Catalog + Rag3dbConnection~~ ✓ (ce doc)
3. ~~Vérification compilation WASM~~ ✓ (ce doc)
4. **Playwright / WASM browser** — test bout-en-bout avec rag3db_wasm.js + CallbackConnection

## Décisions ouvertes (inchangées)

- Node.js binding : napi-rs vs JS wrapper pur ?
- WASM packaging : wasm-pack + wasm-bindgen vs emscripten ?
- Queue flush timing dans le browser ?
- Extension lucivy_fts via LOAD EXTENSION depuis Rag3dbConnection ?
