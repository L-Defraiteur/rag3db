# Doc 11 — Implémentation terminée : registration idempotente

Date : 12 mars 2026

Réf : doc 07 (design), doc 09 (plan), doc 10 (design reindex)

## Résumé

Les 4 phases du plan (doc 09) sont implémentées et testées E2E.

## Phase 1 : `register_entity()` idempotent + persistance

**Fichiers modifiés** : `src/config.rs`, `src/schema.rs`, `src/catalog.rs`

### config.rs

- `SimpleFieldDef` : ajout de `title_for: Option<String>` et `content_for: Option<Vec<String>>` (pour les KBs)
- Dérivation de `Default` ajoutée
- `title_field()` / `content_fields()` : prennent en compte `title_for="self"` / `content_for=["self"]`
- `validate()` : vérifie exclusivité mutuelle `is_title`/`title_for` et `is_content`/`content_for`

### schema.rs

- `kuzu_default_value(FieldType) -> &'static str` : retourne le default Kuzu pour ALTER TABLE ADD (`''`, `0`, `0.0`, `false`, `'1970-01-01 00:00:00'`)

### catalog.rs

- `register_entity()` refactoré : si l'entité existe déjà → `migrate_entity()`, sinon → `create_entity_tables()`
- `migrate_entity()` : diff les champs, ALTER TABLE ADD pour les nouveaux, erreur pour suppression/changement de type, rebuild FTS si content fields changent, flag `needs_reindex`
- `entity_config_to_entity_def()` : helper extrait
- `persist_meta_key()` / `persist_entity_config()` : MERGE dans `_catalog_meta`
- `load_entity_configs()` : charge depuis `_catalog_meta` au `initialize()`

## Phase 2 : `register_relation()`

**Fichier modifié** : `src/catalog.rs`

- Méthode publique `register_relation(rel_name, from, to)` — `CREATE REL TABLE IF NOT EXISTS`
- Vérifie que les deux entités existent dans `config.entities`
- Si déjà enregistrée avec mêmes endpoints → no-op. Endpoints différents → erreur.
- `persist_relation()` / `load_relations()` dans `_catalog_meta`

## Phase 3 : `register_kb()`

**Fichier modifié** : `src/catalog.rs`

- Méthode publique `register_kb(kb_name, kb_config)`
- Scanne `config.entities` pour trouver les entités avec `title_for`/`content_for` pointant vers cette KB
- `create_kb_tables()` : crée `{KB}_Index`, `{KB}_Index_Chunk`, rels (`HAS_CHUNK`, `IN_{KB}`, `SOURCED_{KB}`), FTS, vector, sparse indexes
- Construit `KBMetadata` et l'insère dans `self.kb_metadata`
- `persist_kb_config()` / `load_kb_configs()` dans `_catalog_meta`

## Phase 4 : `reindex()`

**Fichier modifié** : `src/catalog.rs`

- Méthode publique `reindex(entity_name) -> ReindexStats`
- Query tous les `_uuid` + données de l'entité
- Enqueue comme `UpdateRecord` dans `pending.updates`
- `drain()` — UpdateRecordNode gère le reste :
  - Simple entities : détecte hash mismatch → rechunk pipeline
  - KB entities : enqueue AggregateRecords → KB pipeline (KBGatherNode re-gather)
- Clear flag `needs_reindex:{entity}` dans `_catalog_meta`

## Persistance `_catalog_meta`

Table key-value existante (jamais utilisée avant). Clés :

| Préfixe | Contenu |
|---------|---------|
| `entity_config:{name}` | JSON de EntityConfig |
| `relation:{name}` | JSON de RelationDef `{ from, to }` |
| `kb_config:{name}` | JSON de KBConfig |
| `needs_reindex:{name}` | `"true"` / `"false"` |

Chargé à `initialize()` étape 8 : `load_entity_configs()` → `load_relations()` → `load_kb_configs()`.

## Tests E2E

**Fichier** : `tests/e2e_idempotent_registration.rs` — 10 tests, tous verts.

| Test | Scénario |
|------|----------|
| `register_entity_idempotent_same_config` | Re-appel = no-op, données intactes |
| `register_entity_add_non_content_field` | ALTER TABLE ADD, default `''`, ingest avec nouveau champ OK |
| `register_entity_add_content_field_and_reindex` | FTS rebuild + reindex → search retrouve les résultats |
| `register_entity_remove_field_errors` | Erreur explicite "cannot remove field" |
| `register_entity_change_type_errors` | Erreur explicite "cannot change type" |
| `register_entity_persists_and_reloads` | Close DB → reopen → entity configs restaurés, ingest + migration OK |
| `register_relation_idempotent_and_conflict` | No-op si même, erreur si endpoints différents, erreur si entité inconnue |
| `progressive_schema_evolution` | V1 → V3 → V4, données intactes, search OK |
| `ingest_after_migration_works` | Ingest avec nouveau schéma, anciens records intacts |
| `entity_config_persisted_in_catalog_meta` | JSON roundtrip OK |

## Ce qui n'est pas fait

- Migration destructive (drop champ, change type) — voir doc 08
- Migration additive sur KB existante (`register_kb()` = no-op si KB existe déjà)
- `reindex_all()` (toutes les entités d'un coup)
- Batching intermédiaire dans `reindex()` (tout en un drain)
