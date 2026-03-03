# 07 — Rapport de progression Phase 0 + Phase 1

## Phase 0a : Float priority (u8 → f32) — FAIT ✓

Migration complète de `priority: u8` vers `priority: OrderedPriority(f32)` dans rag3weaver.

### Fichiers modifiés

1. **`extension/rag3weaver/src/ops.rs`**
   - Ajout `OrderedPriority(f32)` newtype avec `Ord` via `f32::total_cmp()`
   - `OperationConfig.priority: u8` → `OrderedPriority`
   - 6 constantes migrées : `OP_CHUNK(0.0)`, `OP_INSERT(1.0)`, `OP_LINK(2.0)`, `OP_EMBED/SPARSE/DUAL(3.0)`
   - `CatalogOp::priority() -> OrderedPriority`
   - +2 tests : `ordered_priority_ordering`, `ordered_priority_btreemap`
   - Tests existants mis à jour (`assert_eq!(op.priority(), OrderedPriority(1.0))`)

2. **`extension/rag3weaver/src/queue.rs`**
   - `QueueEvent` : 5 variants `priority: u8` → `OrderedPriority`
   - `FlushOptions.up_to_priority: Option<u8>` → `Option<OrderedPriority>`
   - `flush()` : `BTreeMap<u8, Vec<OperationItem>>` → `BTreeMap<OrderedPriority, Vec<OperationItem>>`
   - `max_priority = u8::MAX` → `OrderedPriority(f32::MAX)`
   - `flush_insertions()` → `Some(OrderedPriority(1.0))`
   - `flush_links()` → `Some(OrderedPriority(2.0))`
   - `ops_info: Vec<(&str, u8)>` → `Vec<(&str, OrderedPriority)>`

3. **`extension/rag3weaver/src/persistence.rs`**
   - `PersistedOp.priority: u8` → `f32`

4. **`extension/rag3weaver/src/cypher_persistence.rs`**
   - Schema `_Operation` : `priority INT64` → `DOUBLE`
   - `persist()` : `priority as i64` + `CypherValue::Int` → `priority.0 as f64` + `CypherValue::Float`
   - `row_to_persisted_op()` : `.as_i64()...as u8` → `.as_f64()...as f32`
   - Test `row_to_persisted_op_parses` : `CypherValue::Int(1)` → `CypherValue::Float(1.0)`, assert `== 1.0`

### Résultat : 352 tests passent, 0 échecs

---

## Phase 0b : Cross-entity KB à l'ingestion — EN COURS

### Concept

Quand une KB a `entities.len() > 1` (ex: TreeKB avec Directory + File), créer une **table d'index partagée** `{KB}_Index` au lieu d'indexes FTS/HNSW séparés par entité. Avantage : BM25 avec IDF partagé (scores comparables), pas de fusion au search.

### Table cible

```sql
CREATE NODE TABLE IF NOT EXISTS TreeKB_Index(
    _uuid STRING,
    _source_entity STRING,  -- "File" | "Directory"
    _source_uuid STRING,
    _title STRING,
    _content STRING,
    TreeKB_embedding FLOAT[384],
    PRIMARY KEY(_uuid)
)
```

### Plan d'implémentation (3 fichiers)

#### 1. `schema.rs` — DDL generation (EN COURS)

- [ ] Ajouter fn `generate_index_table_ddl(kb_name, embedding_dim, kb_config)` → DDL de `{KB}_Index`
- [ ] Ajouter fn `generate_index_rel_ddl(entity_name, kb_name)` → `{Entity}_IN_{KB}` rel
- [ ] Dans `generate_full_schema()` :
  - Détecter KBs multi-entity (agréger `resolve_entity_kbs()` cross-entities)
  - Pour chaque KB multi-entity : générer index table + rels + FTS/HNSW sur la table d'index
  - Skip les index FTS/HNSW per-entity pour les KBs multi-entity
- [ ] Tests unitaires DDL

#### 2. `catalog.rs` — Ingestion + Search

- [ ] Ajouter `is_multi_entity: bool` dans `KBMetadata`
- [ ] Ajouter fn `maybe_enqueue_index_entry_ops(entity_name, uuid, data) -> Vec<CatalogOp>`
  - Pour chaque KB multi-entity contenant l'entité : créer InsertOp + LinkOp + EmbedOp dans `{KB}_Index`
  - Appeler depuis `create()` et `update()`
- [ ] Modifier `search()` pour les KBs multi-entity :
  - `entity` = `"{KB}_Index"` au lieu de `kb.title.entity`
  - BM25 fields = `["_title", "_content"]`
  - Enrichir résultats avec `_source_entity` + `_source_uuid`
- [ ] Propagation delete/update vers les entries d'index

#### 3. `validator.rs` — Pas de changement nécessaire

`KBValidation.entities: HashSet<String>` existe déjà. On détecte multi-entity via `entities.len() > 1`.

### État : lecture de schema.rs terminée, implémentation pas encore commencée

Le code de `generate_full_schema()` (lignes 347-450) est bien compris. Le point d'insertion est après la boucle per-entity (ligne 439), et il faut ajouter un guard dans la boucle d'index per-entity (ligne 406) pour skipper les KBs multi-entity.

---

## Phase 1 : Schema Code Domain + CRUD E2E — PAS COMMENCÉ

Dépend de Phase 0b. Config YAML et test E2E définis dans le plan (`/home/luciedefraiteur/.claude/plans/federated-dancing-firefly.md`).

4 entités (File, Directory, Scope, Library), 7 relations, 4 KBs (FileContentKB, TreeKB, ScopeKB, LibraryKB).

---

## Fichier plan

Le plan détaillé est dans : `/home/luciedefraiteur/.claude/plans/federated-dancing-firefly.md`
