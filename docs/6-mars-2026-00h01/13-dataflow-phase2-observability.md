# Doc 13 — Phase 2 Complete : Observabilité Dataflow

## Résultat

**Phase 2 terminée à 100%.** Tap per-edge, ExecutionReport sérialisable, recording en rag3db ou JSONL. 18 tests unitaires + 7 E2E compilent, 0 régressions (394 unit tests total).

---

## Fichiers créés : `src/dataflow/`

| Fichier | ~Lignes | Tests | Rôle |
|---------|---------|-------|------|
| `observe.rs` | 190 | 6 | `TapSpec`, `TapEvent`, `TapRegistry` — interception per-edge, zero cost si inactif |
| `report.rs` | 175 | 4 | `ExecutionReport`, `NodeReport`, `EdgeReport`, `summarize_port_value()` — construit depuis DataflowEvent |
| `record.rs` | 260 | 4 | `DataflowRecorder`, `RecordSink` (Database/File/Both/None), `RecordRetention` — persiste en rag3db Cypher ou JSONL |

## Fichier E2E créé

| Fichier | Tests | Couverture |
|---------|-------|-----------|
| `tests/e2e_dataflow_observe.rs` | 7 | execute_with_report simple + expansion, tap_all, tap spécifique, record JSONL, record DB, report JSON structure |

## Fichiers modifiés

| Fichier | Changement |
|---------|-----------|
| `dataflow/runtime.rs` | `Serialize` sur `DataflowEvent`, `DataflowOutput::empty()`, `TapRegistry` intégré, `tap()`/`tap_all()`/`execute_with_report()`, 3 nouveaux tests |
| `dataflow/mod.rs` | Modules `observe`/`report`/`record`, exports publics (`TapEvent`, `TapSpec`, `ExecutionReport`, `DataflowRecorder`, etc.) |
| `catalog.rs` | `conn_arc()` — expose `Arc<dyn DbConnection>` pour le recorder |

---

## Architecture

### Tap system

```
runtime.tap("source", "out", "sink", "in") → Receiver<TapEvent>
runtime.tap_all() → Receiver<TapEvent>
```

- Zero cost si pas de tap posé (`taps.is_active()` court-circuite)
- Clone la valeur uniquement quand un tap matche
- Check au moment de la collecte des inputs (propagation sur les edges)

### ExecutionReport

```rust
ExecutionReport {
    nodes: Vec<NodeReport>,      // name, status, duration_ms, output_ports
    edges: Vec<EdgeReport>,      // from/to + value_summary ("Results(3)", "Query(TreeKB)")
    expanded_nodes: Vec<String>, // nœuds ajoutés dynamiquement
    total_duration_ms: u64,
    status: ExecutionStatus,     // Completed | Failed { error }
}
```

Construit depuis `DataflowEvent[]` + `DataflowGraph` + `DataflowOutput`. Méthode raccourci : `runtime.execute_with_report(&mut graph)`.

### DataflowRecorder

```
RecordSink::Database(conn) → Cypher batch :
  _DataflowExecution { _uuid, pipeline_name, status, duration_ms, node_count, edge_count }
    ← [:PART_OF] ─ _DataflowNodeRun { node_name, status, duration_ms, output_ports }
    ← [:PART_OF] ─ _DataflowEdgeRun { from_node, from_port, to_node, to_port, value_summary }

RecordSink::File(path) → JSONL (1 ligne JSON par exécution)
RecordSink::Both(conn, path) → DB + JSONL
RecordSink::None → noop
```

Rétention : `max_per_pipeline`, `max_age_days`, `keep_errors`.

---

## Tests

### Unitaires (18 nouveaux, 42 total dataflow)

| Module | Tests | Couverture |
|--------|-------|-----------|
| `observe` | 6 | spec matches, registry inactive/active, emit on match, silent no-match, tap_all |
| `report` | 4 | summarize results/empty, build from events completed/failed, serializes |
| `record` | 4 | JSONL roundtrip, sink none noop, mock DB write, retention defaults |
| `runtime` | 3 | tap specific edge, tap all, execute_with_report |

### E2E (7, compilent)

| Test | Vérifie |
|------|---------|
| `observe_execute_with_report_simple` | Report sur search sans expansion : 2 nœuds, edges, status Completed |
| `observe_execute_with_report_expansion` | Report avec expansion : expanded_nodes non vide, ≥4 nœuds, edges vers compose |
| `observe_tap_all` | Capture toutes les edges : Query edge + Results edge |
| `observe_tap_specific_edge` | Tap ciblé query→primary_search : exactement 1 event, valeur Query |
| `observe_record_jsonl` | Écrit JSONL, vérifie pipeline_name + nœuds + JSON valide |
| `observe_record_database` | Écrit en rag3db, query back _DataflowExecution + _DataflowNodeRun + _DataflowEdgeRun |
| `observe_report_json_structure` | Structure JSON : champs requis, value_summary descriptifs |

### Vérification

```
cargo check                                          → OK
cargo check --test e2e_dataflow_observe --features rag3db-native → OK
cargo check --test e2e_search_queue --features rag3db-native     → OK (non-régression)
cargo test --lib                                     → 394 passed, 0 failed
```

---

## Prochaines phases

| Phase | Scope | Statut |
|-------|-------|--------|
| **1** | Core Framework + Search Migration | **FAIT** |
| **2** | Observabilité + rag3db Storage | **FAIT** |
| 3 | Mermaid + GraphNode + NodeRegistry | À faire |
| 4 | Migrations | À faire |
| 5 | Rhai ScriptNode | À faire |
