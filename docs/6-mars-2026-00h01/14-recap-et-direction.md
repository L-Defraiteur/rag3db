# Doc 14 — Récapitulatif et direction

## État actuel

### rag3weaver — Dataflow Framework

**Phase 1 : Core Framework + Search Migration — FAIT**
- `src/dataflow/` (7 fichiers, ~1475L) remplace `search_queue.rs` + `processors.rs`
- DAG typé : PortType/PortValue, Node/DynamicNode, GraphEmitter, topological sort (Kahn)
- 5 search nodes : QuerySource → PrimarySearch → Expansion (DynamicNode) → FetchRelated(s) + Compose
- Fan-in natif (merge Children de plusieurs FetchRelated), fan-out par clone
- `search_with_strategy()` API publique inchangée, `build_dataflow_graph()` nouvelle API bas niveau
- 24 tests unitaires, 5 E2E

**Phase 2 : Observabilité — FAIT**
- `observe.rs` — Tap per-edge (TapSpec/TapEvent/TapRegistry), zero cost si inactif
- `report.rs` — ExecutionReport sérialisable (NodeReport, EdgeReport, value summaries)
- `record.rs` — DataflowRecorder vers rag3db (Cypher batch) ou JSONL, RecordRetention
- `runtime.rs` — tap()/tap_all()/execute_with_report()
- 18 tests unitaires, 7 E2E
- **Total dataflow : 42 tests unitaires, 394 lib tests, 12 E2E (compilent)**

### ld-lucivy — Python bindings

- Fix SegmentId mismatch (`segment_ord` → `segment_reader().segment_id()`)
- 71 tests pytest (contains, contains_split, fuzzy, regex, highlights, CRUD, persistence, filters, boolean, edge cases)
- README restructuré : guide cross-token vs per-token, anti-patterns documentés
- License corrigée LRSL v1.2

### Commits (locaux, pas poussés)

- **ld-lucivy** `ed9912a` — feat: fix Python SegmentId bug, add 71 pytest tests, rewrite docs
- **rag3db** `ad61707` — feat: dataflow graph framework (Phase 1+2) — replace SearchQueue

---

## Direction : prochaines étapes

### Priorité immédiate : Migration ingestion vers dataflow

L'`OperationQueue` actuelle (insert → embed → link → sparse_embed) souffre du même problème que l'ancienne SearchQueue : shared context, ordonnancement implicite. Migrer vers des nœuds dataflow :

- **InsertNode** — crée les entités en DB (Cypher CREATE)
- **EmbedNode** — appelle l'embedder (dense + sparse)
- **LinkNode** — crée les relations (Cypher MATCH + CREATE)
- **IndexNode** — indexe dans lucivy_fts / vector / sparse

Gains :
- Un seul framework pour tout (search + ingestion)
- Observabilité gratuite (tap, report, record) sur l'ingestion
- Les futures phases (Mermaid, migrations) couvrent automatiquement les deux pipelines
- Base pour les pipelines LLM (normalisation, déduplication, enrichissement)

### Puis Phase 3 : Mermaid + NodeRegistry

- `node_registry.rs` — NodeFactory + NodeSchema, enregistre tous les nœuds built-in (search + ingestion)
- `parser.rs` — Parser Mermaid subset (graph LR/TD, NodeId["Type(params)"], -->|port|)
- `graph_node.rs` — GraphNode (graph-as-node), ports exposés = bords libres
- Templates .mmd built-in (search simple, search+expansion, ingestion)
- `DataflowGraph::to_mermaid()` export
- Variable substitution $var

### Phase 4 : Migrations

- Nœuds migration (QueryNode, BackupNode, ValidateNode, TransformNode, WriteNode)
- MigrationRunner (pending/apply/rollback/status)
- Schema `_DataflowMigration` en rag3db
- Convention `migrations/*.mmd`, dry-run mode

### Phase 5 : Rhai ScriptNode

- ScriptNode (@input/@output annotations)
- ScriptDynamicNode (@dynamic)
- Sandbox (pas d'IO, timeout, mémoire bornée)
- Feature flag rhai optionnel

---

## Architecture cible

```
                    ┌──────────────────────────────────┐
                    │         NodeRegistry             │
                    │  (factories pour tous les nœuds)  │
                    └──────────┬───────────────────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                ▼
        Search nodes     Ingestion nodes    Migration nodes
        (Phase 1)        (à faire)          (Phase 4)
              │                │                │
              └────────────────┼────────────────┘
                               │
                    ┌──────────▼───────────────────────┐
                    │      DataflowRuntime              │
                    │  (execute, tap, report, record)   │
                    └──────────┬───────────────────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                ▼
        Mermaid parser    GraphNode         Rhai ScriptNode
        (Phase 3)         (composable)      (Phase 5)
```

Un seul moteur, toutes les opérations passent par le même DAG typé.
