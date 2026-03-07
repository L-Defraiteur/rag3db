# Doc 22 — Session : Pure Dataflow Ingestion — Phase 2

Date : 6 mars 2026

## Objectif

Implémenter la Phase 2 du doc 20 (Nœuds data-on-ports) : les données d'ingestion circulent sur les ports (pas dans les constructeurs), les services sont récupérés via `ServiceRegistry`, et le pipeline est routé par `SplitOpsNode`.

## Pré-requis validé

Avant d'attaquer Phase 2, tous les tests E2E ont été exécutés pour vérifier que Phase 1 n'avait rien cassé :

- e2e_search: 37 pass
- e2e_native: 11 pass
- e2e_phase0b: 14 pass
- e2e_result_mode: 10 pass
- e2e_search_queue: 5 pass
- e2e_dataflow_observe: 7 pass

**84 E2E, 0 failure.**

## Travail effectué

### 2.1 — SplitOpsNode + GraphEmitter::set_initial_input()

**`SplitOpsNode`** — nouveau nœud routeur qui prend un `BatchPayload<CatalogOp>` en input `ops` et distribue sur 7 ports typés (`inserts`, `links`, `chunks`, `aggregates`, `embeds`, `sparse_embeds`, `dual_embeds`). Chaque port non-vide émet un `BatchPayload` du type correspondant.

**`GraphEmitter::set_initial_input()`** — permet aux DynamicNodes de pré-charger un port d'input sur un nœud émis :

```rust
emitter.add_node(Box::new(InsertBatchNode::new("chunk_inserts")));
emitter.set_initial_input("chunk_inserts", "ops",
    PortValue::Batch(BatchPayload::new(PortType::Inserts, inserts)));
```

Stocké dans `GraphEmitter.initial_inputs: HashMap<String, HashMap<String, PortValue>>`. Le `drain()` retourne maintenant un 4-tuple (ajout du 4e élément `initial_inputs`).

**Runtime** : après `emitter.drain()`, les initial_inputs sont injectés dans le `NodeContext` avant l'exécution du nœud cible.

+3 tests unitaires SplitOpsNode (routes_by_type, empty_input, missing_input_errors).

### 2.2 — Refactor 5 nœuds statiques

Chaque nœud statique passe de "data baked in constructor" à "data-on-ports + services" :

| Nœud | Avant | Après |
|---|---|---|
| `InsertBatchNode` | `new(name, items, conn, cache)` | `new(name)` — input `ops: BatchPayload<InsertOp>`, services `conn`, `node_id_cache` |
| `LinkBatchNode` | `new(name, items, conn)` | `new(name)` — input `ops: BatchPayload<LinkOp>`, service `conn` |
| `EmbedBatchNode` | `new(name, items, conn, embedder, dim)` | `new(name)` — input `ops: BatchPayload<EmbedOp>`, services `conn`, `embedder`, `embedding_dim` |
| `SparseEmbedBatchNode` | `new(name, items, conn, sparse)` | `new(name)` — input `ops: BatchPayload<SparseEmbedOp>`, services `conn`, `sparse_embedder` |
| `DualEmbedBatchNode` | `new(name, items, conn, dual, dim, batch)` | `new(name, gpu_batch_size)` — input `ops: BatchPayload<DualEmbedOp>`, services `conn`, `dual_embedder`, `embedding_dim` |

**Pattern service pour trait objects** : `dyn DbConnection` n'est pas `Sized`, donc `ctx.service::<dyn DbConnection>()` échoue. Solution : enregistrer `Arc<dyn DbConnection>` comme valeur (qui EST Sized), récupérer via `ctx.service::<Arc<dyn DbConnection>>("conn")`.

### 2.3 — Refactor 2 DynamicNodes

**ChunkBatchNode** : struct unitaire (était 12 champs). Input `ops: BatchPayload<ChunkOp>`. Services : `config`, `kb_metadata`, `chunker_cache`, `has_sparse`, `has_dual`. Les nœuds émis utilisent `set_initial_input()`.

**AggregateBatchNode** : struct unitaire (était 11 champs). Input `ops: BatchPayload<AggregateOp>`. Même pattern de services. `process_one()` refactoré en méthode statique prenant tous les services en paramètres.

**Trigger port** : tous les nœuds gardent un input `trigger` (Empty, optional) pour l'ordonnancement des dépendances en plus du port `ops` pour les données.

### 2.4 — Réécriture build_ingestion_graph() + drain()

**`DataflowGraph`** enrichi :
- Nouveau champ `initial_inputs: HashMap<String, HashMap<String, PortValue>>`
- `set_initial_input(node, port, value)` — pour pré-charger les ports d'entrée
- `validate()` considère les initial_inputs comme "connectés" (pas de faux-positif sur les required inputs)

**Runtime** :
- Consomme `graph.initial_inputs` au démarrage de `execute()`
- Le readiness check considère les initial_inputs en plus des edge-delivered data

**`build_ingestion_graph()`** réécrit :

```
Avant : partition ops par type → constructeur(ops, conn, ...) → connect trigger→done
Après : SplitOpsNode → connect ports typés → nœuds unitaires + services via ServiceRegistry
```

Retourne maintenant `(DataflowGraph, ServiceRegistry, usize)` au lieu de `(DataflowGraph, usize)`.

**`drain()`** : utilise `DataflowRuntime::with_services(n, services)` pour injecter les services.

**`flush_insertions()`** : également migré au nouveau pattern.

**Services enregistrés** :

| Clé | Type | Source |
|---|---|---|
| `conn` | `Arc<dyn DbConnection>` | `self.conn` |
| `node_id_cache` | `RwLock<NodeIdCache>` | `self.node_id_cache` (même Arc partagé) |
| `embedder` | `Arc<dyn Embedder>` | `self.embedder` |
| `embedding_dim` | `usize` | `self.config.embedding_dim` |
| `config` | `CatalogConfig` | `self.config` |
| `kb_metadata` | `HashMap<String, KBMetadata>` | `self.kb_metadata` |
| `has_sparse` | `bool` | computed |
| `has_dual` | `bool` | computed |
| `chunker_cache` | `HashMap<ChunkerConfig, Chunker>` | `self.chunker_cache` (moved) |
| `sparse_embedder` | `Arc<dyn SparseEmbedder>` | optionnel |
| `dual_embedder` | `Arc<dyn DualEmbedder>` | optionnel |

### 2.5 — Tests unitaires migrés

Les 4 tests d'intégration dans `ingestion_nodes.rs` (insert_batch_node_resolves_refs, link_batch_node_resolves_refs, embed_batch_node_calls_embedder, insert_then_link_pipeline) réécrits pour utiliser le nouveau pattern :
- `ServiceRegistry` + `DataflowRuntime::with_services()`
- `graph.set_initial_input()` au lieu de data dans les constructeurs

## Fichiers modifiés

| Fichier | Changement |
|---|---|
| `src/dataflow/node.rs` | +`initial_inputs` field sur GraphEmitter, +`set_initial_input()`, drain() → 4-tuple |
| `src/dataflow/graph.rs` | +`initial_inputs` field, +`set_initial_input()`, validate() considère initial_inputs |
| `src/dataflow/runtime.rs` | Consomme `graph.initial_inputs`, readiness check étendu |
| `src/dataflow/ingestion_nodes.rs` | 7 nœuds refactorés data-on-ports, +SplitOpsNode, +trigger ports, 4 tests réécrits, +3 tests SplitOps |
| `src/dataflow/search_nodes.rs` | Fix destructuring drain() 3-tuple → 4-tuple |
| `src/dataflow/mod.rs` | Export SplitOpsNode |
| `src/catalog.rs` | Réécriture build_ingestion_graph() + drain() + flush_insertions() |

## Bugs rencontrés et corrigés

1. **drain() 4-tuple mismatch** : après ajout de `initial_inputs` comme 4e élément du retour de `drain()`, tous les appelants (node.rs test, search_nodes.rs tests, runtime.rs) cassent avec E0308. Fix : mettre à jour les destructurations.

2. **dyn Trait unsized (E0277)** : `ctx.service::<dyn DbConnection>()` échoue car `dyn DbConnection` n'est pas Sized. Fix : enregistrer `Arc<dyn Trait>` (qui est Sized), récupérer via `ctx.service::<Arc<dyn Trait>>()`.

3. **trigger port manquant** : les nœuds refactorés n'avaient plus de port `trigger` (seulement `ops`), mais `build_ingestion_graph()` connecte `done → trigger` pour l'ordonnancement. Fix : ajouter `trigger: Empty, optional` sur tous les batch nodes.

4. **node_id_cache copie vs partage** : première tentative créait une copie du cache (read → clone → new Arc). Corrigé pour partager le même `Arc<RwLock<NodeIdCache>>`.

## Validation

- `cargo test --lib` : **385 pass**, 0 fail, 13 ignored (+3 vs Phase 1)
- E2E (6 suites, 84 tests) : **84 pass**, 0 fail

## Résultat net

Les 5 problèmes identifiés dans le doc 20 sont résolus :

| Problème | Statut |
|---|---|
| P1. Data baked dans constructeurs | Résolu — data-on-ports via BatchPayload |
| P2. Pas de PortValue ingestion | Résolu — 8 variants + BatchPayload (Phase 1) |
| P3. Services dans constructeurs | Résolu — ServiceRegistry string keys |
| P4. build_ingestion_graph() monolithique | Résolu — SplitOpsNode + routage typé |
| P5. unsafe pour muter &self | Résolu — execute(&mut self) (Phase 1) |

## Prochaine étape

**Phase 3** (doc 20) : `NodeRegistry` + sérialisation JSON du graphe — rendre le pipeline descriptif et instanciable par nom, condition pour l'éditeur visuel et les pipelines par KB.
