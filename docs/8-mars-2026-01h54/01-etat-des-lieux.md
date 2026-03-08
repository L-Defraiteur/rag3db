# Doc 01 — État des lieux

Date : 8 mars 2026 (~02h00)

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

#### Phase D — Cleanup ancien pipeline
- Supprimé ~2500 lignes de code mort : ops.rs, queue.rs, persistence.rs, cypher_persistence.rs, ingestion_nodes.rs
- Supprimé compute_chunk_ops() (~160 lignes) de catalog.rs
- Supprimé 7 PortType morts (Ops, Inserts, Links, Chunks, Embeds, SparseEmbeds, DualEmbeds)
- Relocalisé RefOrUuid, FlushResult, DrainStats dans records.rs
- Renommé queue_stats() → drain_stats()

#### Phase 3 — Mermaid + NodeRegistry + GraphNode
- **NodeRegistry** : 12 factories pour tous les nœuds built-in, `add_from_registry()`, `NodeFactory` trait
- **Mermaid parser** : `parse_mermaid()` / `parse_mermaid_template()` — hand-rolled, pas de dépendance externe
  - Syntaxe : `graph LR`, `id["Type(key='value')"]`, `-->|port|`, `-->|from:to|`, `$variable`, `%% commentaire`
- **GraphNode** : graph-as-node (sous-graphes composables), ports exposés = bords libres, transparent au runtime
- **to_mermaid()** : export lisible de tout DataflowGraph
- **4 templates .mmd built-in** : search, search_expansion, ingestion, migration interne

#### Phase 4 — Migrations
- **MigrationRunner** : `pending()`, `apply()`, `rollback()`, `status()`
- **CypherNode** : nœud migration avec undo (capture → restore), mode DDL
- **ValidateNode** : assertions post-migration (not_empty, count_eq, schema_exists, custom)
- **Undo complet** : rollback par nœud via `undo_context()`, `can_undo()`, `undo()`
- **Checkpoint** : sauvegarde d'état par nœud dans `_DataflowNodeState`
- **5 templates internes** : 001_create_dataflow_tables.mmd (DDL des tables checkpoint/migration/lock)

#### Idempotence (doc 27, 32)
- `_text_hash` : posé à l'insertion du chunk
- `_embed_hash` : posé quand l'embedding est écrit
- Si `_embed_hash IS NULL` → chunk jamais embedded (crash recovery)
- Si `_embed_hash != _text_hash` → texte changé, re-embedding nécessaire
- InsertRecordNode : MERGE sur `_uuid` (pas de doublons)
- LinkRecordNode : MERGE sur endpoints

### Tests actuels
- **~500 tests unitaires** (cargo test --lib) — incluant checkpoint, migrations, mermaid, registry
- **~89 tests E2E** (7 suites)
- 0 régression

---

## Ce qui reste à faire

### Pré-requis transversal : Deserialize sur types search

Les types search (`UnifiedResult`, `ChildSummary`, `ChunkInfo`, `SearchMeta`, etc.) n'ont que `Serialize`, pas `Deserialize`. Ça bloque :
- Le round-trip des données à travers ScriptNode (Rhai ↔ Rust)
- Les ports typés configurables (recevoir/émettre des Results)
- Le checkpoint complet pour les pipelines search (`deserialize_non_batch_port_value` = stub)

**Impact** : ~50 loc, mécanique (tous les sous-types sont déjà deserializables).

### Phase "nœuds génériques" — Pipeline sans concept KB

**Problème** : actuellement, toute recherche/ingestion passe par l'abstraction KB (createKB, title_entity, content_fields, relations). Trop lourd pour les cas simples.

**Nouveaux nœuds prévus** (Doc 22) :
- **EmbedNode** — embedding direct, config explicite (entity, text_field, embedding_col, signals)
- **VectorSearchNode** — recherche vectorielle directe sur une table
- **BM25SearchNode** — recherche BM25 directe via Tantivy
- **ResolveSourceNode** — résolution chunk → entité source (via relation CHUNKED_FROM)
- **FuseResultsNode** — fusion multi-source (RRF ou weighted)
- **SearchSourceNode** — source de query sans concept KB
- **FlushNode** — flush FTS configurable (table explicite)

**Renommage KB** — tous les nœuds KB-spécifiques seront préfixés :
| Actuel | Nouveau |
|--------|---------|
| GatherKBNode | KBGatherNode |
| UpdateKBNode | KBUpdateNode |
| ChunkKBNode | KBChunkNode |
| FlushFTSNode | KBFlushNode |
| QuerySourceNode | KBQuerySourceNode |
| PrimarySearchNode | KBSearchNode |

**API SimpleCatalog** — `registerEntity()` + `ingestEntities()` + `searchEntities()` sur le Catalog existant. Config type EntityDef avec `isTitle`/`isContent` + `FieldType`. Pas de concept KB, self-contained.

**Templates simples** : `simple_ingestion.mmd`, `simple_search.mmd`, `hybrid_search.mmd`

### Phase 5 : Extensibilité (Docs 19, 20, 21)

**3 niveaux d'extensibilité** :

1. **ScriptNode (Rhai)** — transformations sandboxées (filtre, reranking, Cypher multi-step)
   - Ports fixes MVP (trigger/result/done), extensibles vers ports configurables
   - Builtins MVP : `query_cypher`, `log`, `set_output`, `set_undo_context`
   - Undo optionnel via `fn undo()` dans le script
   - Feature flag `rhai-script` (~2MB)
   - Source inline ou fichier

2. **HttpNode** — appels HTTP déclaratifs (REST, LLM, APIs)
   - URL, method, headers avec variables template
   - Ports : trigger(in), body(in), response(out), status(out), done(out)
   - Feature flag `http-node` (reqwest)
   - Pattern typique : ScriptNode → HttpNode → ScriptNode

3. **ProcessNode** (futur) — subprocess externe (Python, Node.js)
   - Protocole stdin/stdout JSON
   - Hors scope immédiat

**Principes constants** : sandbox additif, `block_in_place` pour async, limites d'exécution (100K ops, 32 call levels), query_cypher comme builtin core.

### Ordre de priorisation

```
A — Deserialize types search
├→ B — EmbedNode
├→ C — VectorSearchNode
├→ D — BM25SearchNode
├→ E — ResolveSourceNode
├→ F — FuseResultsNode
├→ G — Templates simples
└→ H — Catalog API (registerEntity, ingestEntities, searchEntities)
     └→ I — ScriptNode (Rhai) → J — HttpNode
```

---

## Architecture actuelle

```
                    ┌──────────────────────────────────┐
                    │         NodeRegistry             │
                    │  12 factories (search+ingestion)  │
                    └──────────┬───────────────────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                ▼
        Search nodes     Ingestion nodes    Migration nodes
        (5 nœuds)        (8 nœuds)          (2 nœuds)
              │                │                │
              └────────────────┼────────────────┘
                               │
                    ┌──────────▼───────────────────────┐
                    │      DataflowRuntime              │
                    │  execute, tap, report, record     │
                    │  + Checkpoint (fait)              │
                    └──────────┬───────────────────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                ▼
        Mermaid parser    GraphNode         Templates .mmd
        (fait)            (composable)      (4 built-in)
```

## Documents de design dans ce dossier

| Doc | Titre | Résumé |
|-----|-------|--------|
| **19** | Design : Phase 5 — Rhai ScriptNode | Design concret : ports fixes, inline/file, undo, feature flag, builtins MVP |
| **20** | Réflexion : Extensibilité — Nœuds custom et limites de Rhai | 3 niveaux (Rhai/Http/Process), Deserialize, 6 questions ouvertes |
| **21** | Synthèse : Vision Rhai & extensibilité à travers les docs | Évolution de la vision depuis le 3 mars, ce qui a survécu au pivot Dataflow |
| **22** | Réflexion : Nœuds génériques sans concept KB | Nœuds simples, templates, API SimpleCatalog, 6 questions résolues |
