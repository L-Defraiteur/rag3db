# Doc 17 — Rapport de session : Phase 4 Migrations (WIP)

Date : 8 mars 2026

## Résumé

Phase 4 en cours. Étape 1 complète (trait Node + checkpoint undo). Étape 2 en cours (CypherNode + ValidateNode créés, pas encore compilé).

## Travail réalisé

### Étape 1 — Trait Node + checkpoint undo (FAIT, compilé, 436 tests green)

**Modifié : `src/dataflow/node.rs`**
- Ajout de 3 méthodes au trait `Node` (avec defaults) :
  - `can_undo(&self) -> bool` (default: false)
  - `undo_context(&self) -> Option<serde_json::Value>` (default: None)
  - `async fn undo(&mut self, ctx: &mut NodeContext, undo_ctx: serde_json::Value) -> Result<(), String>` (default: Err)

**Modifié : `src/dataflow/checkpoint.rs`**
- Ajout champ `undo_context: Option<serde_json::Value>` à `NodeCheckpoint` (avec `#[serde(default, skip_serializing_if)]`)
- Signature `CheckpointStore::save_node_completed` changée : ajout paramètre `undo_context: Option<&serde_json::Value>`

**Modifié : `src/dataflow/checkpoint_store.rs`**
- Schema `_DataflowNodeState` : ajout colonne `undo_json STRING`
- `CypherCheckpointStore::save_node_completed` : écrit `undo_json` dans le MERGE
- `CypherCheckpointStore::load_execution` : lit `n.undo_json` (7e colonne), parse en `Option<serde_json::Value>`
- `MockCheckpointStore::save_node_completed` : stocke `undo_context` dans `NodeCheckpoint`
- Tests mis à jour : 2 appels `save_node_completed` reçoivent `None` comme undo_context
- Toutes les constructions `NodeCheckpoint { ... }` reçoivent `undo_context: None`

**Modifié : `src/dataflow/runtime.rs`**
- Dans `execute_inner_with_checkpoint`, après `execute()` réussi :
  - `let undo_ctx = graph.nodes[node_idx].undo_context();`
  - Passé à `store.save_node_completed(..., undo_ctx.as_ref(), ...)`
- Construction `NodeCheckpoint` pour nouveau checkpoint : ajout `undo_context: None`

### Étape 2 — CypherNode + ValidateNode (EN COURS, fichier créé, pas encore compilé)

**Nouveau fichier : `src/dataflow/migration_nodes.rs`** (~500 lignes, ~30 tests)

#### CypherNode
- Exécute une query Cypher, optionnellement capture undo context
- Config : `query` (requis), `capture` (optionnel — query SELECT exécutée AVANT la mutation)
- Ports : input `trigger` (Empty, optionnel), outputs `result` (Map) + `done` (Empty)
- `can_undo() = true` si `capture_query` est défini
- `undo_context()` retourne les rows capturées en JSON
- `undo()` reconstruit des `SET` à partir des valeurs capturées (par `_uuid`)
- Accède à la DB via `ctx.service::<Arc<dyn DbConnection>>("conn")`
- `CypherNodeFactory` avec config params `query` + `capture`

#### ValidateNode
- Assertion sur résultat d'une query Cypher, fail le graphe si violée
- Config : `query` (requis), `assert` (requis), `message` (optionnel)
- Ports : input `trigger` (Empty, optionnel), output `done` (Empty)
- `can_undo() = false` (lecture seule)
- `ValidateNodeFactory`

#### Assertion enum
- `IsEmpty`, `IsNotEmpty`, `CountEquals(i64)`, `CountGt(i64)`, `CountLt(i64)`
- `Expression { column, op, value }` avec `AssertOp` (Eq, Gt, Lt, Gte, Lte)
- `Assertion::parse(str)` : parse depuis config string ("empty", "count > 0", "cnt == 5")
- `Assertion::check(row_count, first_row_value)` : vérifie l'assertion

#### Helper
- `cypher_value_to_json(CypherValue) -> serde_json::Value`

**Modifié : `src/dataflow/mod.rs`**
- Ajout `pub mod migration_nodes;`
- Ajout exports : `CypherNode, CypherNodeFactory, ValidateNode, ValidateNodeFactory, Assertion`

### NON FAIT (reste pour étape 2)

- **Enregistrer les factories dans `node_factories.rs`** : ajouter `CypherNodeFactory` et `ValidateNodeFactory` dans `register_builtins()`
- **Compiler et tester** : `cargo check --lib` + `cargo test --lib`
- Le fichier `migration_nodes.rs` est écrit mais pas encore compilé — il peut y avoir des erreurs mineures (imports, variantes CypherValue manquantes)

### Étapes restantes (Phase 4)

3. **MigrationRunner** — `migrations.rs` : scan dirs, pending, apply, rollback, status, verrouillage TTL, dry-run
4. **Undo sur record nodes existants** — InsertRecordNode, LinkRecordNode, EmbedRecordNode, UpdateKBNode
5. **Templates de migration internes** — `migrations/internal/001_create_dataflow_tables.mmd`
6. **E2E tests**

## Commits

- `b2f32400a` — docs: design Phase 4 — migrations + undo intégré au trait Node
- Pas de commit code encore (code non compilé)

## Tests

| Suite | Count |
|-------|-------|
| Unit tests (après étape 1) | 436 pass, 0 fail |
| E2E | 89 pass, 0 fail |

## Fichiers touchés (non commités)

```
M  src/dataflow/node.rs           (3 méthodes undo au trait)
M  src/dataflow/checkpoint.rs     (undo_context field + save_node_completed signature)
M  src/dataflow/checkpoint_store.rs (undo_json column + read/write + mock + tests)
M  src/dataflow/runtime.rs        (capture undo_context après execute)
M  src/dataflow/mod.rs            (wire migration_nodes + exports)
A  src/dataflow/migration_nodes.rs (CypherNode + ValidateNode + tests)
```
