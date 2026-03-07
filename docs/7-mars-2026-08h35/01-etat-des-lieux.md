# Doc 01 — État des lieux

Date : 7 mars 2026

## rag3weaver — Pipeline d'ingestion et de recherche

### Ce qui est fait

#### Phase 1 — Core Dataflow Framework + Search Migration (doc 10)
- `src/dataflow/` : DAG typé avec PortType/PortValue, Node/DynamicNode, GraphEmitter, topo sort (Kahn)
- 5 search nodes : QuerySource → PrimarySearch → Expansion (DynamicNode) → FetchRelated(s) + Compose
- Fan-in (merge), fan-out (clone), ServiceRegistry

#### Phase 2 — Observabilité (docs 13, 14)
- `observe.rs` — Tap per-edge, zero cost si inactif
- `report.rs` — ExecutionReport sérialisable (NodeReport + metrics, EdgeReport)
- `record.rs` — DataflowRecorder vers rag3db (Cypher batch) ou JSONL, RecordRetention
- `runtime.rs` — tap()/tap_all()/execute_with_report(), NodeEventFilter par nœud

#### Phase A-B — Record-based ingestion nodes (docs 26, 27)
- 8 nœuds record-based remplacent l'ancien pipeline batch :
  - InsertRecordNode, LinkRecordNode, EmbedRecordNode
  - ChunkRecordNode, GatherKBNode, UpdateKBNode, ChunkKBNode, FlushFTSNode
- Graphe d'ingestion **entièrement statique** (pas de DynamicNode côté ingestion)
- Metrics structuré via `ctx.log_metric()` (remplace eprintln)
- Vrais ports data (entities, relations, kb_content) — plus de `done: Empty` seulement

#### Phase C — Wire record nodes into pipeline (doc 29)
- `build_ingestion_graph()` utilise uniquement les record nodes
- `drain()` construit le graphe, exécute via DataflowRuntime, agrège FlushResult
- PendingWork remplace CatalogOp : create()/link() poussent EntityRecord/RelationRecord/AggregateRecord

#### Phase D — Cleanup ancien pipeline (cette session)
- Supprimé ~2500 lignes de code mort : ops.rs, queue.rs, persistence.rs, cypher_persistence.rs, ingestion_nodes.rs
- Supprimé compute_chunk_ops() (~160 lignes) de catalog.rs
- Supprimé 7 PortType morts (Ops, Inserts, Links, Chunks, Embeds, SparseEmbeds, DualEmbeds)
- Relocalisé RefOrUuid, FlushResult, DrainStats dans records.rs
- Renommé queue_stats() → drain_stats()

#### Idempotence (doc 27, 32)
- `_text_hash` : posé à l'insertion du chunk
- `_embed_hash` : posé quand l'embedding est écrit
- Si `_embed_hash IS NULL` → chunk jamais embedded (crash recovery)
- Si `_embed_hash != _text_hash` → texte changé, re-embedding nécessaire
- InsertRecordNode : MERGE sur `_uuid` (pas de doublons)
- LinkRecordNode : MERGE sur endpoints

### Tests actuels
- **359 tests unitaires** (cargo test --lib)
- **86 tests E2E** (7 suites : native, phase0b, search, search_queue, result_mode, batch_observe, dataflow_observe)
- 0 régression

### Architecture du graphe d'ingestion

```
create()/link()/update()/delete()
        │
        ▼
    PendingWork { entities, relations, aggregates }
        │
        ▼  build_ingestion_graph()
┌───────────────────────────────────────────────────────────────┐
│ InsertRecordNode ──inserted──▶ ChunkRecordNode ──chunks──▶ InsertRecordNode("chunk_inserts")
│       │                              │                        │
│       │──inserted──▶ EmbedRecordNode │──chunk_links──▶ LinkRecordNode("chunk_links")
│       │                              │
│       │──done──▶ LinkRecordNode      chunk_inserts ──done──▶ EmbedRecordNode("chunk_embeds")
│                     │
│                     │──done──▶ GatherKBNode ──kb_content──▶ UpdateKBNode ──▶ ChunkKBNode
│                                                                │
│                                                     chunks ──▶ InsertRecordNode("kb_inserts")
│                                                     rels   ──▶ LinkRecordNode("kb_links")
│                                                     kb_inserts ──done──▶ EmbedRecordNode("kb_embeds")
│                                                                          ──done──▶ FlushFTSNode
└───────────────────────────────────────────────────────────────┘
        │
        ▼  DataflowRuntime.execute()
    FlushResult { processed, failed }
```

---

## Ce qui reste à faire

### Prochaine étape : Checkpoint (crash recovery)

Design dans doc 27. Le graphe d'ingestion peut crasher mid-execution (GPU timeout, DB down, OOM). L'idempotence des nœuds permet le replay, mais sans checkpoint on reprend tout depuis le début.

**Objectif** : Sauvegarder l'état après chaque nœud, permettre la reprise à partir du dernier nœud complété.

**Design prévu** :
- `GraphCheckpoint` : `execution_id`, `completed_nodes` (HashSet), `port_data` (sérialisé), `last_checkpoint`
- Stocké dans une table `_DataflowCheckpoint` en rag3db
- Cycle : load → skip completed → save après chaque nœud → delete on success

### Phase 3 : Mermaid + NodeRegistry + GraphNode (doc 10)

**Objectif** : Définir des pipelines en Mermaid, composer des sous-graphes, templates.

- `NodeRegistry` : factories pour tous les nœuds built-in (search + ingestion), `NodeFactory` trait
- Parser Mermaid subset : `graph LR/TD`, `NodeId["Type(param='value')"]`, `-->|port_name|`, `$variable`
- `GraphNode` : graph-as-node (sous-graphes composables), ports exposés = bords libres
- 4 templates `.mmd` built-in
- `DataflowGraph::to_mermaid()` export

### Phase 4 : Migrations (doc 10)

**Objectif** : Migrations de schema graph, basées sur le framework dataflow.

- Nœuds migration : QueryNode, BackupNode, ValidateNode, TransformNode, WriteNode
- `MigrationRunner` : pending/apply/rollback/status
- Schema `_DataflowMigration` en rag3db
- Convention `migrations/*.mmd`

### Phase 5 : Rhai ScriptNode (doc 10)

**Objectif** : Nœuds custom en Rhai pour les power users.

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
        (fait)           (fait)             (Phase 4)
              │                │                │
              └────────────────┼────────────────┘
                               │
                    ┌──────────▼───────────────────────┐
                    │      DataflowRuntime              │
                    │  (execute, tap, report, record)   │
                    │  + Checkpoint (à faire)           │
                    └──────────┬───────────────────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                ▼
        Mermaid parser    GraphNode         Rhai ScriptNode
        (Phase 3)         (composable)      (Phase 5)
```
