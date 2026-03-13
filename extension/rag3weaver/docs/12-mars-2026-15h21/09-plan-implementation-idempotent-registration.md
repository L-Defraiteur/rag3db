# Doc 09 — Plan d'implémentation : registration idempotente

Date : 12 mars 2026

Réf : doc 07 (design), doc 08 (migrations destructives — plus tard)

## Phase 1 : register_entity() idempotent + persistance

**Fichiers** : `src/catalog.rs`, `src/schema.rs`, `src/config.rs`

### 1a. Étendre SimpleFieldDef (config.rs)

Ajouter `title_for: Option<String>` et `content_for: Option<Vec<String>>` à `SimpleFieldDef`.
Garder `is_title` et `is_content` comme raccourcis pour le pipeline simple ("self").
Validation : mutuellement exclusifs (`is_title` vs `title_for`, `is_content` vs `content_for`).

### 1b. Helper schema.rs

Ajouter `kuzu_default_value(field_type) -> &'static str` pour les defaults ALTER TABLE ADD.

### 1c. Persistance dans _catalog_meta (catalog.rs)

Deux méthodes internes :
- `persist_entity_config(name, config)` : MERGE dans `_catalog_meta` clé `entity_config:{name}`, valeur = JSON
- `load_entity_configs()` : MATCH WHERE _key STARTS WITH 'entity_config:', désérialise, restore entity_configs + config.entities

Appeler `load_entity_configs()` à la fin de `initialize()` (nouvelle étape 8).

### 1d. Refactor register_entity() (catalog.rs)

Extraire `entity_config_to_entity_def(config) -> EntityDef` en helper.

Nouvelle logique :
```
if entity déjà registered:
  diff fields (added / removed / type changed)
  removed → erreur (pour l'instant, doc 08)
  type changed → erreur (pour l'instant, doc 08)
  added → ALTER TABLE ADD {field} {type} DEFAULT {default}
  content/title fields changés → rebuild FTS + flag needs_reindex
  new signals → create missing indexes
else:
  code actuel (CREATE tables etc.)

persist config → _catalog_meta
update in-memory
```

### 1e. Flag needs_reindex (catalog.rs)

Si content/title fields ajoutés/modifiés :
- Persist `needs_reindex:{entity}` = `true` dans `_catalog_meta`
- Log warning

### 1f. Tests

- `register_entity_idempotent_same_config` — no-op
- `register_entity_add_field` — ALTER TABLE ADD
- `register_entity_add_content_field_flags_reindex` — needs_reindex
- `register_entity_remove_field_errors` — erreur
- `register_entity_change_type_errors` — erreur
- `register_entity_persists_and_reloads` — persist → load

### Vérification

```bash
cargo test --lib --features "rag3db-native,candle-embedder"
```

---

## Phase 2 : register_relation()

**Fichiers** : `src/catalog.rs`

Nouvelle méthode publique :
```rust
pub async fn register_relation(
    &mut self, rel_name: &str, from: &str, to: &str
) -> Result<(), CatalogError>
```

- Génère `CREATE REL TABLE IF NOT EXISTS {rel}(FROM {from} TO {to})` (DDL existe déjà dans schema.rs)
- Idempotent : IF NOT EXISTS
- Persist dans `_catalog_meta` clé `relation:{name}`
- Charger au `initialize()` dans `load_entity_configs()` (ou `load_relations()` séparé)

---

## Phase 3 : register_kb()

**Fichiers** : `src/catalog.rs`, `src/schema.rs`

Nouvelle méthode publique :
```rust
pub async fn register_kb(
    &mut self, kb_name: &str, kb_config: KBConfig
) -> Result<(), CatalogError>
```

- Détecte les entités qui ont des champs `title_for`/`content_for` pointant vers cette KB
- Crée les tables KB : `{KB}_Index`, `{KB}_Index_Chunk`, rels, FTS, indexes
- Si KB existe déjà : diff et migration additive (ALTER TABLE ADD, rebuild FTS)
- Persist dans `_catalog_meta` clé `kb_config:{name}`
- Build `KBMetadata` et l'ajouter à `self.kb_metadata`

---

## Phase 4 : reindex()

**Fichiers** : `src/catalog.rs`

```rust
pub async fn reindex(&mut self, entity_name: &str) -> Result<(), CatalogError>
```

- Query tous les UUIDs de l'entité
- Enqueue comme updates dans PendingWork
- Drain → re-chunk, re-embed, re-index FTS
- Clear flag `needs_reindex:{entity}` dans `_catalog_meta`

---

## Ordre recommandé

Phase 1 → 2 → 3 → 4. Chaque phase est utile indépendamment. Phase 1 couvre le cas d'usage principal (simple entities idempotentes).
