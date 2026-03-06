# Doc 15 — Migration Ingestion vers Dataflow

## Objectif

Remplacer `OperationQueue` + 7 `Processor`s par un pipeline dataflow typé.
Un seul framework pour search **et** ingestion.
Observabilité gratuite (tap, report, record) sur l'ingestion.

---

## Analyse du pipeline actuel

### OperationQueue (queue.rs, ~700L)

Queue par priorité avec `BTreeMap<OrderedPriority, Vec<OperationItem>>`. Flush séquentiel par palier. Après chaque palier, les ops injectées par les processors (via `QueueSender`) sont mergées dans les paliers restants. Assert que les ops injectées ont une priorité **strictement supérieure**.

### Processeurs (catalog.rs, ~1050L)

| Processeur | Priorité | Input | Output | Injection |
|---|---|---|---|---|
| **ChunkProcessor** | 0.0 | `ChunkOp` | — | InsertOp, LinkOp, EmbedOp, SparseEmbedOp via sender |
| **InsertProcessor** | 1.0 | `InsertOp` | Cypher CREATE | Résout EntityRef (UUID) |
| **LinkProcessor** | 2.0 | `LinkOp` | Cypher MATCH+CREATE | Résout RelationRef |
| **AggregateProcessor** | 2.5 | `AggregateOp` | UPDATE _content | InsertOp@2.6, LinkOp@2.7, Embed@3.0 via sender |
| **EmbedProcessor** | 3.0 | `EmbedOp` | UNWIND SET embedding | — |
| **SparseEmbedProcessor** | 3.0 | `SparseEmbedOp` | UNWIND SET sparse | — |
| **DualEmbedProcessor** | 3.0 | `DualEmbedOp` | UNWIND SET dense+sparse | — |

### Ops (ops.rs, ~480L)

`CatalogOp` enum avec 7 variantes. `RefOrUuid` pour la résolution différée. `EntityRef`/`RelationRef` via channels oneshot. `OrderedPriority(f32)` pour le BTreeMap.

### Patterns clés à préserver

1. **Résolution différée** : InsertProcessor résout `EntityRef` → UUID. Les EmbedOps en aval attendent cette résolution (`entity_ref.ready().await`).
2. **Injection dynamique** : ChunkProcessor et AggregateProcessor émettent des ops downstream via `QueueSender.emit()`.
3. **Déduplication** : AggregateProcessor déduplique par `index_entry_uuid` (100 links au même Directory → 1 seul rebuild).
4. **Batching** : Les embed processors groupent par `(entity_name, col)` pour des UNWIND efficaces. DualEmbedProcessor fait des mini-batches GPU de 32 dans des mega-batches de 500.
5. **Priorité override** : Post-aggregate inserts/links ont des priorités 2.6/2.7 pour s'intercaler.

---

## Architecture cible : Ingestion Dataflow

### Principe

Chaque `Processor` actuel devient un `Node` (ou `DynamicNode`) dataflow. Les priorités deviennent des **edges** (dépendances). L'injection via QueueSender devient `GraphEmitter.add_node() + .connect()`.

### Graphe de base (sans aggregate)

```
                        ┌──────────────┐
  CatalogOp[] ─────────►│  ChunkNode   │ (DynamicNode)
                        │  prio: 0.0   │
                        └──────┬───────┘
                               │ émet dynamiquement:
                     ┌─────────┼─────────────┐
                     ▼         ▼             ▼
              ┌──────────┐ ┌──────────┐ ┌──────────────┐
              │InsertNode│ │InsertNode│ │InsertNode    │
              │(entity)  │ │(chunk_1) │ │(chunk_2)     │
              └────┬─────┘ └────┬─────┘ └────┬─────────┘
                   │            │             │
                   └────────────┼─────────────┘
                                ▼
                        ┌──────────────┐
                        │  LinkNode    │
                        └──────┬───────┘
                               ▼
                        ┌──────────────┐
                        │  EmbedNode   │ (batch, UNWIND)
                        └──────────────┘
```

### Graphe complet (avec aggregate)

```
  ┌────────────┐     ┌─────────────┐     ┌────────────┐     ┌───────────────┐
  │ ChunkNode  │────►│ InsertNode  │────►│ LinkNode   │────►│ AggregateNode │
  │ (Dynamic)  │     │             │     │            │     │ (Dynamic)     │
  └────────────┘     └─────────────┘     └────────────┘     └──────┬────────┘
                                                                    │ émet:
                                                     ┌──────────────┼──────────────┐
                                                     ▼              ▼              ▼
                                              ┌────────────┐ ┌────────────┐ ┌────────────┐
                                              │InsertNode  │ │LinkNode    │ │EmbedNode   │
                                              │(chunks@2.6)│ │(@2.7)     │ │(@3.0)      │
                                              └──────┬─────┘ └──────┬─────┘ └────────────┘
                                                     └──────────────┘
                                                             ▼
                                                      ┌────────────┐
                                                      │EmbedNode   │
                                                      │(final@3.0) │
                                                      └────────────┘
```

### Nouveau : PortValue variants pour l'ingestion

Il faut étendre `PortValue` avec des variants pour transporter les données d'ingestion entre nœuds :

```rust
// Ajouts à PortValue
Operations(Vec<CatalogOp>),      // batch d'ops à traiter
Entities(Vec<EntityData>),        // entités créées (uuid résolus)
References(Vec<ResolvedRef>),     // refs résolues (entity_ref → uuid)
EmbedWork(Vec<EmbedWorkItem>),    // textes + UUIDs prêts pour embedding
```

**Alternative** : Passer les données via `ServiceRegistry` (comme on fait pour `Catalog` dans les search nodes) et utiliser les ports existants (`Uuids`, `Map`, `Any`) pour la signalisation. Plus simple, moins de variants.

**Recommandation** : L'alternative `ServiceRegistry + ports légers`. Les ops d'ingestion sont side-effectful (DB writes) — les PortValues servent de signal/sync, pas de transport lourd.

---

## Mapping détaillé : Processor → Node

### 1. ChunkNode (DynamicNode)

**Remplace** : ChunkProcessor
**Inputs** : `ops` (PortType::Any — Vec<ChunkOp>)
**Outputs** : `done` (PortType::Empty — signal de complétion)
**Dynamic** : Émet InsertNode(s) + LinkNode(s) + EmbedNode(s) via GraphEmitter

```rust
impl DynamicNode for ChunkNode {
    fn execute_dynamic(&self, ctx: &mut NodeContext, emitter: &mut GraphEmitter) {
        // Récupère les ChunkOps depuis le ServiceRegistry (ou PortValue::Any)
        // Rayon parallel chunking
        // Pour chaque chunk résultant :
        //   emitter.add_node("insert_chunk_N", InsertNode { ... })
        //   emitter.connect("insert_chunk_N", "done", "link_chunk_N", "trigger")
        //   emitter.add_node("link_chunk_N", LinkNode { ... })
        //   emitter.add_node("embed_chunk_N", EmbedNode { ... })
    }
}
```

**Question ouverte** : Le pattern "données baked-in" (comme FetchRelatedNode dans search) est plus naturel ici que des ports. Chaque InsertNode émis dynamiquement porte ses propres données dans ses champs, pas via un port.

### 2. InsertNode (Node)

**Remplace** : InsertProcessor
**Champs constructeur** : `entity_name`, `data: BTreeMap<String, CypherValue>`, `resolver: EntityRefResolver`
**Inputs** : `trigger` (PortType::Empty — attend la complétion du nœud précédent)
**Outputs** : `done` (PortType::Empty — signal pour les nœuds en aval)
**Services** : `DbConnection` via ServiceRegistry, `NodeIdCache` via ServiceRegistry

```rust
impl Node for InsertNode {
    async fn execute(&self, ctx: &mut NodeContext) {
        let conn = ctx.service::<dyn DbConnection>();
        let cache = ctx.service::<NodeIdCache>();
        // Cypher CREATE, résout EntityRef, cache node ID
    }
}
```

### 3. LinkNode (Node)

**Remplace** : LinkProcessor
**Champs** : `rel_name`, `from: RefOrUuid`, `to: RefOrUuid`, `properties`, `resolver`
**Inputs** : `trigger` (Empty)
**Outputs** : `done` (Empty)

### 4. AggregateNode (DynamicNode)

**Remplace** : AggregateProcessor
**Champs** : `index_entry_uuid`, `kb_name`, `title_entity`, `source_uuid`
**Inputs** : `trigger` (Empty)
**Outputs** : `done` (Empty)
**Dynamic** : Émet InsertNode(s) + LinkNode(s) + EmbedNode(s) pour les chunks reconstruits

La déduplication est gérée en amont : le code appelant (Catalog.create/link) ne crée qu'un AggregateNode par `index_entry_uuid` unique, pas un par linkage.

### 5. EmbedNode (Node)

**Remplace** : EmbedProcessor
**Champs** : `works: Vec<EmbedWorkItem>` (uuid + text + entity_name + kb_name)
**Inputs** : `trigger` (Empty — attend que tous les inserts soient faits)
**Outputs** : `done` (Empty)

Le batching (UNWIND par group entity+col) reste interne au nœud.

### 6. SparseEmbedNode / DualEmbedNode (Nodes)

Idem EmbedNode mais pour sparse / dual. DualEmbedNode conserve son mini-batching GPU interne.

---

## Changements dans PortType / PortValue

### Option A : Variants dédiés (riche mais lourd)

Ajouter `Operations`, `Entities`, `References`, `EmbedWork` à PortValue.

**Pour** : Typé, tap/report montrent le contenu.
**Contre** : Gonfle l'enum, les ops d'ingestion ne sont pas sérialisables facilement (contiennent des `Arc`, `EntityRefResolver`).

### Option B : Signalisation légère (recommandé)

Les nœuds d'ingestion portent leurs données dans leurs **champs constructeur** (comme `FetchRelatedNode` avec `parents` baked-in). Les ports servent uniquement de sync :

- `trigger` (PortType::Empty) : attend la complétion
- `done` (PortType::Empty) : signale la complétion

Les données transitent via les champs du nœud + ServiceRegistry (pour DbConnection, Embedder, etc.).

**Pour** : Simple, pas de changement à PortValue, cohérent avec les search nodes.
**Contre** : Tap/report ne montrent que "Empty" sur les edges, pas le contenu.

**Mitigation** : Le `summarize_port_value()` de report.rs voit "Empty" mais les DataflowEvents (NodeStarted/NodeCompleted) portent le nom du nœud qui encode le contexte (ex: `"insert_TreeKB_Index_Chunk_abc123"`).

### Décision : **Option B**

---

## Migration de Catalog.drain()

### Avant (queue.rs)

```rust
pub async fn drain(&mut self) -> FlushResult {
    self.queue.drain().await  // priority-ordered flush
}
```

### Après (dataflow)

```rust
pub async fn drain(&mut self) -> FlushResult {
    let graph = self.build_ingestion_graph();
    let runtime = DataflowRuntime::new(10);
    let output = runtime.execute(&mut graph).await?;
    // Convertir DataflowOutput en FlushResult
}
```

### build_ingestion_graph()

Méthode sur Catalog qui construit le graphe d'ingestion à partir des ops pendantes :

```rust
fn build_ingestion_graph(&mut self) -> DataflowGraph {
    let mut graph = DataflowGraph::new();
    let ops = std::mem::take(&mut self.pending_ops);

    // 1. ChunkNode(s) — un par batch de ChunkOps (DynamicNode)
    // 2. InsertNode(s) — un par InsertOp (ou batch si on groupe par entity)
    // 3. LinkNode(s) — un par LinkOp, attend la résolution des from/to
    // 4. AggregateNode(s) — un par index_entry_uuid unique (DynamicNode)
    // 5. EmbedNode(s) — un par batch de même KB (attend inserts)

    // Les edges encodent les dépendances :
    // chunk_N.done → insert_M.trigger
    // insert_M.done → link_K.trigger
    // link_K.done → aggregate_J.trigger
    // insert_M.done → embed_L.trigger (via fan-in)

    graph
}
```

### Batching dans le DAG

**Problème** : L'OperationQueue actuelle batch par `batch_size` (50 inserts, 32 embeds). Dans le DAG, chaque InsertNode représente un item unique. Trop de nœuds ?

**Solution** : Les nœuds d'ingestion sont **batch-aware**. Un `BatchInsertNode` prend `Vec<InsertOp>` et fait la boucle interne. On réduit le nombre de nœuds à O(types) au lieu de O(items).

```
ChunkBatchNode → InsertBatchNode → LinkBatchNode → AggregateBatchNode → EmbedBatchNode
```

Chaque batch node fait la même boucle que le processeur actuel. L'avantage du dataflow :
- Les edges encode les dépendances (pas de priorités implicites)
- L'observabilité est gratuite
- Les DynamicNodes (Chunk, Aggregate) peuvent émettre des sous-graphes

---

## Sous-tâches d'implémentation

### Phase I.1 : Ingestion Nodes (fichier `dataflow/ingestion_nodes.rs`)

| Nœud | Type | ~Lignes | Tests |
|---|---|---|---|
| `InsertBatchNode` | Node | 80 | 2 |
| `LinkBatchNode` | Node | 70 | 2 |
| `EmbedBatchNode` | Node | 90 | 2 |
| `SparseEmbedBatchNode` | Node | 80 | 2 |
| `DualEmbedBatchNode` | Node | 100 | 2 |
| `ChunkBatchNode` | DynamicNode | 60 | 2 |
| `AggregateBatchNode` | DynamicNode | 200 | 3 |
| **Total** | | ~680 | 15 |

### Phase I.2 : build_ingestion_graph() dans catalog.rs

| Changement | ~Lignes |
|---|---|
| `build_ingestion_graph()` méthode | 120 |
| `drain()` réécrit via DataflowRuntime | 30 |
| `drain_parallel()` supprimé (le runtime gère) | -50 |
| **Net** | ~100 |

### Phase I.3 : Cleanup

| Action |
|---|
| Supprimer les 7 struct Processor de catalog.rs (~1050L) |
| Supprimer queue.rs (~700L) sauf QueueEvent (gardé pour compat?) |
| Supprimer ops.rs (~480L) ou le garder (les structs sont toujours utiles) |
| Mettre à jour lib.rs |
| Vérifier E2E (e2e_search_queue.rs, e2e_dataflow_observe.rs) |

### Phase I.4 : E2E Tests ingestion dataflow

| Test | Vérifie |
|---|---|
| `ingestion_simple_insert_link` | Insert + Link via dataflow, entité créée en DB |
| `ingestion_with_chunks` | ChunkBatchNode émet InsertBatchNodes dynamiquement |
| `ingestion_with_aggregate` | AggregateNode rebuild _content, re-chunk, embed |
| `ingestion_with_embed` | EmbedBatchNode UNWIND, vecteurs en DB |
| `ingestion_report` | ExecutionReport montre les nœuds d'ingestion |
| `ingestion_tap` | Tap sur edge chunk→insert |

---

## Questions ouvertes

### 1. EntityRef résolution dans le DAG

L'OperationQueue actuelle utilise `EntityRef.ready().await` pour attendre qu'un InsertOp ait résolu le UUID. Dans le DAG, les InsertNodes s'exécutent **avant** les LinkNodes/EmbedNodes par construction (edges). Donc la résolution est implicite — l'InsertNode écrit le UUID dans le `NodeContext` et les nœuds en aval le lisent.

**Mais** : `EntityRef` utilise des channels oneshot (`watch`). Faut-il garder ce mécanisme ou passer à des ports ?

**Recommandation** : Garder `EntityRef`/`RefOrUuid` tel quel. L'InsertNode résout l'EntityRef comme avant (via `resolver.resolve(uuid)`). Les EmbedNodes en aval font toujours `entity_ref.ready().await` mais c'est instantané car l'InsertNode a déjà tourné (garanti par le DAG). Pas de changement nécessaire.

### 2. Déduplication AggregateOp

Actuellement fait dans `AggregateProcessor.process()` avec un HashSet. Dans le DAG, la déduplication se fait **en amont** dans `build_ingestion_graph()` : on ne crée qu'un seul `AggregateBatchNode` par `index_entry_uuid`.

### 3. Retries

L'OperationQueue supporte les retries (`max_retries`, item remis en Pending sur échec). Le DataflowRuntime actuel ne supporte pas les retries — un nœud qui fail arrête tout.

**Option** : Ajouter un try/retry dans le nœud lui-même (boucle interne). Plus simple qu'un mécanisme de retry au niveau runtime.

### 4. Persistance (OperationPersistence)

L'OperationQueue persiste les ops dans rag3db avant traitement (recoverability). Le dataflow ne fait pas ça nativement.

**Recommandation** : Différer. La persistance est rarement utilisée en pratique. Le DataflowRecorder (Phase 2) enregistre déjà les exécutions complètes. Si besoin, ajouter un `PersistenceNode` plus tard.

### 5. `drain_parallel()` (WASM rayon)

Utilisé uniquement en WASM pour le drainage parallèle via rayon. Dans le dataflow, on peut marquer certains nœuds comme parallélisables. Pas prioritaire — on s'en occupe quand on fait le build WASM.

---

## Estimation

| Phase | Effort | Fichiers |
|---|---|---|
| I.1 Ingestion Nodes | ~680L nouveau | `dataflow/ingestion_nodes.rs` |
| I.2 build_ingestion_graph + drain | ~150L modifié | `catalog.rs` |
| I.3 Cleanup | ~-2000L supprimé | `queue.rs`, `catalog.rs` |
| I.4 E2E tests | ~400L nouveau | `tests/e2e_ingestion_dataflow.rs` |
| **Net** | ~-770L (simplification) | |

---

## Risques

1. **AggregateProcessor** est complexe (~500L, 8 étapes, queries DB, re-chunking). Le porter vers un DynamicNode est le plus risqué.
2. **DualEmbedProcessor** avec ses mini-batches GPU et timing events est délicat mais encapsulé.
3. **Tests E2E** dépendent de la DB réelle (rag3db-native feature flag). Si les tests dataflow Phase 1+2 passent déjà, ça devrait aller.
4. **L'API publique de Catalog** (`drain()`, `flush_insertions()`, `has_pending()`, etc.) doit rester identique. Les tests existants ne doivent pas casser.

---

## Ordre d'exécution recommandé

1. **I.1** : Écrire `ingestion_nodes.rs` avec les 7 nœuds + 15 tests unitaires
2. **I.2** : `build_ingestion_graph()` + réécrire `drain()` dans catalog.rs
3. **I.4** : E2E tests (avant cleanup pour avoir les deux systèmes en parallèle)
4. **I.3** : Cleanup (supprimer les processeurs, queue.rs partiel)
5. **Commit** : feat: migrate ingestion pipeline to dataflow framework
