# Doc 18 — Rapport de session : Phase 4 Migrations (étapes 1-4 complètes)

Date : 8 mars 2026

## Résumé

Phase 4 — étapes 1 à 4 complètes. 485 tests pass. Il reste l'étape 5 (templates de migration internes + E2E).

## Travail réalisé

### Étape 1 — Trait Node + checkpoint undo (FAIT)

- 3 méthodes undo ajoutées au trait `Node` (defaults)
- `undo_context: Option<serde_json::Value>` ajouté à `NodeCheckpoint`
- `undo_json STRING` dans `_DataflowNodeState`
- Runtime capture `undo_context()` après execute réussi

### Étape 2 — CypherNode + ValidateNode (FAIT)

- `migration_nodes.rs` (~900 lignes avec tests)
- `CypherNode` : exécute Cypher, capture optionnelle pour undo, `CypherNodeFactory`
- `ValidateNode` : assertion sur résultats query, `ValidateNodeFactory`
- `Assertion` enum avec `parse()` + `check()`
- `cypher_value_to_json()` helper (tous variants CypherValue couverts dont Map)
- 30 tests unitaires
- Factories enregistrées dans `register_builtins()` (14 types total)

### Étape 3 — MigrationRunner (FAIT)

- `migrations.rs` (~550 lignes)
- `MigrationRunner` : `initialize()`, `status()`, `pending()`, `apply()`, `rollback()`, `check_reversible()`
- `MigrationFile`, `MigrationStatus`, `MigrationState`, `MigrationResult`, `MigrationDirection`
- `DryRunPlan` / `DryRunNode` pour dry-run
- `MigrationError` avec 10 variantes
- Locking TTL 10min via `_DataflowMigrationLock`
- Scan de répertoires, validation format `{version}_{name}.mmd`
- Schema `_DataflowMigration` + `_DataflowMigrationLock` (CREATE NODE TABLE IF NOT EXISTS)
- 19 tests unitaires (parsing filename, scan dir, display, erreurs)

### Étape 4 — Undo sur record nodes (FAIT)

Ajout de `undo_data: Option<serde_json::Value>` + implémentation `can_undo()`, `undo_context()`, `undo()` sur :

| Nœud | can_undo | undo() fait |
|------|----------|-------------|
| InsertRecordNode | true | DETACH DELETE par entity_name + _uuid |
| LinkRecordNode | true | DELETE relations par from/to/rel_name |
| EmbedRecordNode | true | SET _embed_hash = NULL (force re-embed) |
| UpdateKBNode | true | Restaure anciennes _title, _content, _content_hash |
| FlushFTSNode | true | Re-flush (CALL FLUSH_LUCIVY_INDEX) |

Nœuds inchangés (can_undo = false) : ChunkRecordNode, ChunkKBNode, GatherKBNode, QuerySourceNode, PrimarySearchNode, ComposeNode, FetchRelatedNode.

### Étape 5 — Templates internes (RESTE À FAIRE)

- `migrations/internal/001_create_dataflow_tables.mmd`
- Tests E2E migration apply/rollback
- Vérification : `./run_e2e.sh` toujours green

## Fichiers touchés

```
M  src/dataflow/node.rs               (3 méthodes undo au trait)
M  src/dataflow/checkpoint.rs         (undo_context field)
M  src/dataflow/checkpoint_store.rs   (undo_json column)
M  src/dataflow/runtime.rs            (capture undo_context)
M  src/dataflow/mod.rs                (wire migration_nodes + migrations)
M  src/dataflow/node_factories.rs     (14 factories, CypherNode + ValidateNode)
M  src/dataflow/record_nodes.rs       (undo sur 5 nœuds)
A  src/dataflow/migration_nodes.rs    (CypherNode + ValidateNode + Assertion)
A  src/dataflow/migrations.rs         (MigrationRunner)
M  Cargo.toml                         (tempfile dev-dep)
```

## Tests

| Suite | Count |
|-------|-------|
| Unit tests | 485 pass, 0 fail |
| Ignored (GPU) | 13 |
