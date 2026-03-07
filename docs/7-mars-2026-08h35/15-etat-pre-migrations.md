# Doc 15 — État pré-migrations

Date : 7 mars 2026

## Résumé

Vérification complète avant Phase 4. Les 2 E2E précédemment signalés comme cassés (doc 12) passent désormais. Suite complète verte : 436 unit tests, 89 E2E, 0 failures.

## E2E corrigés (doc 12 → résolu)

### `observe_execute_with_report_expansion`
- **Cause originale** : assertion sur `expanded_nodes` (concept DynamicNode supprimé)
- **Résolution** : `expanded_nodes` est toujours `Vec::new()` depuis la suppression de DynamicNode. L'assertion `is_empty()` passe correctement. Le test avait été écrit avec le commentaire expliquant ce comportement attendu.

### `observe_record_database`
- **Cause originale** : `Cannot find property pipeline_name for e`
- **Résolution** : le schéma `_DataflowExecution` dans `checkpoint_store.rs` inclut `pipeline_name` depuis l'implémentation du checkpoint (doc 06). La table est créée correctement avant que `record.rs` ne tente d'écrire. `CREATE NODE TABLE IF NOT EXISTS` est un no-op cohérent car les deux schémas sont identiques.

## Tests — état complet

| Suite | Total | Pass | Fail |
|-------|-------|------|------|
| Unit tests (`cargo test --lib`) | 436 | 436 | 0 |
| E2E observe | 7 | 7 | 0 |
| E2E search | 5 | 5 | 0 |
| E2E checkpoint | 14 | 14 | 0 |
| E2E batch | 37 | 37 | 0 |
| E2E ingestion | 11 | 11 | 0 |
| E2E dataflow | 2 | 2 | 0 |
| E2E record nodes | 3 | 3 | 0 |
| E2E search queue | 10 | 10 | 0 |
| **Total E2E** | **89** | **89** | **0** |

## Phases complétées

| Phase | Contenu |
|-------|---------|
| Phase 1 | Core dataflow framework + search migration |
| Phase 2 | Observability (taps, reports, recorder) |
| Ingestion A-D | Record nodes, drain(), cleanup ~2500 lignes |
| Checkpoint | Crash recovery, resume, `_DataflowExecution` |
| Phase 3a | NodeRegistry + 12 factories |
| Phase 3b | Mermaid parser + `from_definition()` |
| Phase 3c | GraphNode + GraphNodeFactory |
| Templates | 4 templates `.mmd` built-in |

## Prochaine étape — Phase 4 : Migrations

Objectif : migrations de schéma graph à la Supabase, basées sur le framework dataflow.

### Nouveaux nœuds
- `QueryNode` — exécute une requête Cypher, expose les résultats
- `BackupNode` — snapshot d'une table avant transformation
- `ValidateNode` — vérifie des invariants (count, schema, contraintes)
- `TransformNode` — applique une transformation Cypher (SET, CREATE, DELETE)
- `WriteNode` — écrit le résultat d'une migration (log, rapport)

### Infrastructure
- `MigrationRunner` — `pending()`, `apply()`, `rollback()`, `status()`
- Schema tracking : `_DataflowMigration { version, name, status, applied_at, execution_uuid, checksum }`
- Convention fichiers : `migrations/001_name.mmd`
- Dry-run mode
- Rollback via `execution_uuid`

### Fichiers à créer
- `src/dataflow/migration_nodes.rs` — nœuds migration
- `src/dataflow/migrations.rs` — MigrationRunner + schema
