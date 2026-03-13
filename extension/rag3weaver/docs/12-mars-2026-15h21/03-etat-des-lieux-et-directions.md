# Doc 03 — État des lieux complet et directions possibles

Date : 12 mars 2026

## 1. Ce qui est fait — vue d'ensemble

### 1.1 Framework dataflow (core)

- DAG typé avec PortType/PortValue, topological sort (Kahn), fan-in/fan-out
- DataflowRuntime : execute(), execute_with_checkpoint()
- 25+ nœuds composables enregistrés dans NodeRegistry
- Parser Mermaid (.mmd) pour définir des graphes déclarativement
- GraphNode : graph-as-node pour composition hiérarchique
- Variable substitution $var dans les templates

Source : docs `6-mars/14-recap-et-direction.md`, `7-mars/07-design-node-registry.md`

### 1.2 Observabilité

- Tap per-edge (TapSpec/TapEvent/TapRegistry), zero cost si inactif
- ExecutionReport sérialisable (NodeReport avec inputs/outputs/logs/metrics)
- DataflowRecorder vers rag3db (Cypher batch) ou JSONL
- PortSnapshot : capture des données transitant entre nœuds
- NodeLog : ctx.debug()/info()/warn()/error() — séparation claire entre CatalogEvent (business) et DataflowEvent (runtime)
- EventBus comme service dans le ServiceRegistry

Source : `11-mars/09-observabilite-dataflow-nodelog.md`

### 1.3 Checkpoint + crash recovery

- Sauvegarde per-node des outputs dans _DataflowNodeState
- Resume : graph hash validation, skip completed nodes, re-inject saved outputs
- Undo context capture : après chaque execute() réussi, le runtime appelle undo_context() et persiste dans undo_json

Source : `7-mars/03-design-checkpoint-complet.md`

### 1.4 Pipeline d'ingestion (KB + simple entities)

**8 nœuds d'ingestion** :
- InsertRecordNode, LinkRecordNode, ChunkRecordNode, EmbedNode
- KBGatherNode, KBUpdateNode, KBChunkNode, KBEmbedNode
- FlushNode (FTS flush configurable)

**3 nœuds CRUD** (ajoutés phase queue/drain) :
- DeleteRecordNode — cascade delete entities + chunks, capture undo
- UpdateRecordNode — batch hash read, change detection, re-chunk si changé, capture undo
- RechunkDeleteNode — supprime old chunks avant re-chunking

**Pipeline KB complet** :
```
DeleteRecordNode → UpdateRecordNode → InsertRecordNode → LinkRecordNode
                                                              ↓
                                                    KBGatherNode (lit pending_aggregates service)
                                                              ↓
                                              KBUpdateNode → KBChunkNode → agg_inserts → agg_links → agg_embeds
                                                    ↓
                                                FlushNode
```

**Pipeline simple entities** :
```
register_entity() crée : table entité + table chunks + relation CHUNKED_FROM + FTS index
ingest_entities() : InsertRecordNode → ChunkRecordNode → InsertRecordNode(chunks) → LinkRecordNode → EmbedNode → FlushNode
```

Source : `11-mars/02-plan-queue-drain-unifie.md`, `8-mars-weaver/01-etat-des-lieux.md`

### 1.5 Queue/drain unifié (Phases 1-5)

Toutes les mutations passent par PendingWork + drain() :
```rust
create()  → enqueue EntityRecord      (sync)
link()    → enqueue RelationRecord     (sync)
update()  → enqueue UpdateRecord       (sync)
delete()  → enqueue DeleteRecord       (sync)
drain()   → build_ingestion_graph() → exécute le DAG (async)
```

- ~1050 lignes dead code supprimées (ancien update/delete inline, batch_update, batch_delete, rechunk_simple_entities)
- FlushResult étendu : processed, failed, update_results, delete_results
- Conflict resolution : delete + update même UUID → delete gagne

Source : `11-mars/02-plan-queue-drain-unifie.md`, `11-mars/08-progression-phase4-en-cours.md`, `11-mars/11-conflict-resolution-plan.md`

### 1.6 Undo actif (Delete + Update)

- DeleteRecordNode.undo() : restaure les entités via CREATE + SET (données capturées avant DETACH DELETE)
- UpdateRecordNode.undo() : restaure les anciennes valeurs via MATCH SET
- KBUpdateNode.undo() : restaure old _title, _content, _content_hash
- Nœuds read-only marqués can_undo() = true avec undo() no-op
- CypherCheckpointStore : mark_completed() préserve les nodes avec undo_context (fix session 12 mars)
- Filtrage _id/_label dans la capture undo (propriétés système rag3db read-only)

**4 tests e2e validés** :
- undo_delete_simple_entity (BM25 only)
- undo_update_simple_entity (BM25 only)
- undo_delete_simple_entity_bgem3 (BM25 + Vector + Sparse, DualEmbedder, BM25-only sanity check)
- undo_delete_kb_bgem3 (BM25 + Vector + Sparse sur KB, DualEmbedder, BM25-only sanity check)

Source : `11-mars/12-undo-actif-plan.md`, `12-mars/01-undo-e2e-rapport.md`, `12-mars/02-deadlock-fix-rapport.md`

### 1.7 Search

- BM25 multi-field avec highlights + chunk resolution (offsets highlights → overlap avec char ranges des chunks)
- Vector search (dense embeddings, HNSW index via extension vector)
- Sparse search (BGE-M3 ou BM42/Candle, extension sparse_vector)
- Fusion via Reciprocal Rank Fusion (RRF)
- 3 result modes : Aggregated, SourceResolved, Detailed
- BM25Mode : Contains, ContainsSplit, Fuzzy, Phrase, Parse
- Diagnostics optionnels : per-hit BM25 highlight/chunk overlap, per-phase timing
- DualEmbedder : dense + sparse en un seul forward pass (BgeM3Embedder + CandleDualEmbedder)

### 1.8 FTS (tantivy_fts / lucivy_fts)

- Extension C++ complète (CREATE, QUERY, DROP, FLUSH)
- Hooks insert/update/delete
- Lazy commit (dirty_ flag, flush on first query)
- Filter fields natifs
- 15 tests GTest E2E
- 1025 tests ld-lucivy (Rust)

### 1.9 Builds

- Native Rust : rag3dbjs.node + LOAD EXTENSION
- WASM : rag3db_wasm.js 17MB, tantivy_fts statiquement linké
- run_e2e.sh avec --summary (13 suites, 130 tests e2e, 537 unit tests)

### 1.10 Simple entities (registerEntity API)

```rust
pub struct SimpleFieldDef {
    pub field_type: FieldType,  // String, Text, Int64, Double, Boolean, etc.
    pub is_title: bool,
    pub is_content: bool,
}

pub struct EntityConfig {
    pub fields: HashMap<String, SimpleFieldDef>,
    pub signals: SearchSignals,  // BM25 | VECTOR | SPARSE
}
```

- register_entity() crée les tables, chunks, relations, FTS index automatiquement
- ingest_entities() : pipeline dataflow complet
- search() unifié : fonctionne sur KB et simple entities via resolve_search_target()
- update/delete intégrés au drain

Source : `8-mars-rag3db/22-reflexion-noeuds-generiques-sans-kb.md` §8

---

## 2. Ce qui n'est PAS fait

### 2.1 Deserialize sur les types search

**Quoi** : UnifiedResult, ChildSummary, ChunkInfo, SearchMeta, SearchTarget, AttributedChunk, SearchDiagnostics n'ont que Serialize, pas Deserialize.

**Impact** : Bloque le checkpoint des pipelines search (PortValue::Results ne peut pas être désérialisé depuis le checkpoint). Bloque aussi le round-trip ScriptNode (Rhai).

**Effort** : ~0.5 jour. Ajouter derive(Deserialize) + vérifier que CypherValue a Deserialize.

**Cascade** : nécessaire pour search port checkpoint (doc 13) et ScriptNode (doc 22 §7).

Source : `11-mars/13-search-port-checkpoint-plan.md`

### 2.2 Search port checkpoint

**Quoi** : `deserialize_non_batch_port_value()` dans checkpoint.rs retourne toujours une erreur. Les pipelines search ne passent pas par execute_with_checkpoint().

**Impact** : Pas de crash recovery ni resume pour les pipelines search. Pas critique tant qu'on n'a pas de pipelines mixtes (ingestion + search dans le même graph).

**Effort** : ~1 jour (après Deserialize).

Source : `11-mars/13-search-port-checkpoint-plan.md`

### 2.3 CypherNode + ValidateNode (migrations)

**Quoi** : Nœuds génériques pour les migrations de schéma.
- CypherNode : exécute une query Cypher, avec capture optionnelle pour undo automatique
- ValidateNode : assertions sur le résultat d'une query (count, empty, expression)

**Impact** : Briques nécessaires pour le MigrationRunner.

**Effort** : ~1 jour.

Source : `7-mars/16-design-migrations-undo.md`

### 2.4 MigrationRunner

**Quoi** : Orchestrateur qui scanne les fichiers .mmd, les applique en ordre, supporte le rollback via undo.
- Schema : _DataflowMigration + _DataflowMigrationLock (TTL)
- API : status(), pending(), apply(), rollback(), check_reversible()
- Dry-run : parse + validate + affiche le plan sans exécuter
- Verrouillage : empêche les apply concurrents

**Impact** : Permet des migrations de schéma réversibles, crash-safe, déclaratives. Le undo actif qu'on vient d'implémenter est un prérequis.

**Effort** : ~1.5 jours.

Source : `7-mars/16-design-migrations-undo.md`

### 2.5 Auto-drain après rollback

**Quoi** : Quand un undo de DeleteRecordNode restaure des entités, il faut re-ingérer (chunks, embeddings, FTS). Le plan prévoit que le rollback dans migrations.rs détecte le pending work et lance un auto-drain.

**État** : Le undo fonctionne (testé e2e), mais l'auto-drain n'est pas câblé dans le MigrationRunner (qui n'existe pas encore). Les tests e2e font le re-ingest manuellement.

**Effort** : ~0.5 jour (intégré au MigrationRunner).

Source : `11-mars/12-undo-actif-plan.md` étape 3

### 2.6 ScriptNode (Rhai)

**Quoi** : Nœud sandboxé qui exécute du Rhai pour des transformations custom dans le pipeline dataflow.
- @input/@output annotations
- @dynamic pour ScriptDynamicNode
- Sandbox : pas d'IO, timeout, mémoire bornée
- Feature flag optionnel

**Impact** : Permet aux utilisateurs d'ajouter de la logique custom (filtre, reranking, transformation) sans écrire de Rust.

**Prérequis** : Deserialize sur types search (§2.1).

**Effort** : ~2 jours.

Source : `7-mars/19-design-rhai-scriptnode.md`, `7-mars/20-reflexion-extensibilite-noeuds-custom.md`

### 2.7 HttpNode

**Quoi** : Nœud pour appels HTTP déclaratifs (REST, LLM APIs) dans le pipeline dataflow.

**Effort** : ~1 jour.

Source : `7-mars/20-reflexion-extensibilite-noeuds-custom.md`

### 2.8 Phase C : wrapper Node.js pour rag3weaver

**Quoi** : Exposer les fonctions Catalog (register_entity, ingest_entities, search, update, delete, drain) dans les bindings Node.js (rag3dbjs).

**État** : Le WASM n'expose pas update/delete. Node.js n'expose pas encore les nouvelles APIs.

**Effort** : ~2-3 jours.

Source : `11-mars/01-etat-des-lieux-et-roadmap.md` §4

### 2.9 Sparse index V2 mmap

**Quoi** : Remplacer la persistance bincode (full load/save) par un format mmap + LRU cache + WAL. Élimine le coût O(N) à chaque open/commit.

**Impact** : Nécessaire si les volumes dépassent quelques milliers de documents.

**Effort** : ~3-5 jours.

Source : `11-mars/01-etat-des-lieux-et-roadmap.md` §6

### 2.10 Nœuds search génériques (sans KB)

**Quoi** : VectorSearchNode, BM25SearchNode, ResolveSourceNode, FuseResultsNode — nœuds de recherche composables qui fonctionnent sans l'abstraction KB.

**État** : La réflexion est documentée en détail. Les templates Mermaid sont designés. Mais les nœuds ne sont pas implémentés car registerEntity + search() unifié couvre déjà le cas d'usage principal. Ces nœuds deviendraient utiles pour des pipelines search custom (via Mermaid).

**Effort** : ~3-4 jours pour les 4 nœuds + templates.

Source : `8-mars-rag3db/22-reflexion-noeuds-generiques-sans-kb.md`

---

## 3. Directions possibles

### Direction A — MigrationRunner (continuer l'undo)

On a le trait undo, les tests e2e prouvent que ça marche (delete + update, KB + simple entities, tous signaux). La suite logique :

1. CypherNode + ValidateNode (~1j)
2. MigrationRunner avec apply/rollback/dry-run (~1.5j)
3. Auto-drain post-rollback (~0.5j)
4. Templates de migration internes (~0.5j)

**Pour qui** : rag3weaver lui-même (migrations internes de schéma) + utilisateurs qui veulent des migrations réversibles sur leur graph.

**Prérequis** : rien, on a tout ce qu'il faut.

### Direction B — Deserialize + search port checkpoint

1. Ajouter Deserialize sur les types search (~0.5j)
2. Implémenter deserialize_non_batch_port_value() (~0.5j)
3. Tests E2E pipeline mixte checkpointé (~0.5j)

**Pour qui** : débloque ScriptNode, pipelines mixtes, crash recovery search.

**Prérequis** : rien.

### Direction C — Wrapper Node.js

Exposer tout ce qui est construit dans les bindings Node.js :
- register_entity, ingest_entities, search, update, delete, drain
- Events (subscribe)
- Possiblement : DualEmbedder via NAPI

**Pour qui** : rendre le tout utilisable côté applicatif. Valeur immédiate pour les consommateurs.

**Prérequis** : les APIs Rust sont stables (c'est le cas).

### Direction D — ScriptNode (Rhai)

Nœuds custom sandboxés dans les pipelines dataflow. Puissant mais nécessite Deserialize d'abord (Direction B).

**Pour qui** : power users qui veulent de la logique custom sans écrire de Rust.

### Direction E — Nœuds search génériques + templates Mermaid

VectorSearchNode, BM25SearchNode, ResolveSourceNode, FuseResultsNode — rendre les pipelines search composables via Mermaid, sans passer par le Catalog.

**Pour qui** : cas avancés où l'utilisateur veut contrôler le pipeline search étape par étape.

---

## 4. Dépendances entre directions

```
B (Deserialize) ─────→ D (ScriptNode)
                  └──→ search port checkpoint

A (MigrationRunner) ←── standalone, pas de dépendance

C (Node.js wrapper) ←── standalone, APIs Rust stables

E (Nœuds search)   ←── B souhaitable mais pas bloquant
```

## 5. Tests — état actuel

| Suite | Tests |
|-------|------:|
| Unit tests (cargo test --lib) | 537 |
| e2e_batch_observe | 2 |
| e2e_checkpoint | 3 |
| e2e_dataflow_observe | 7 |
| e2e_drain_unified | 6 |
| e2e_generic_search | 8 |
| e2e_highlight_long_text | 8 |
| e2e_native | 11 |
| e2e_phase0b | 14 |
| e2e_result_mode | 10 |
| e2e_search | 37 |
| e2e_search_queue | 5 |
| e2e_simple_entity | 15 |
| e2e_undo | 4 |
| **Total** | **667** |

Tout vert, 0 régressions.
