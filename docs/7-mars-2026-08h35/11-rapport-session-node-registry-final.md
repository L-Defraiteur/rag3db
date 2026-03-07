# Doc 11 — Rapport de session : NodeRegistry Phase 3 (final)

Date : 7 mars 2026

## Contexte

Suite au rapport doc 09 (#167-#169 terminés), cette session finalise le plan en 6 étapes (doc 07) : tâches #170, #171, #172.

## Tâches complétées

### #170 — Implémenter factories pour les 12 node types (TERMINÉ)

#### Pré-requis : Serialize/Deserialize sur SearchOptions

Ajout de `Serialize + Deserialize` en cascade sur 6 types pour permettre `serde_json::from_value(SearchOptions)` dans la factory QuerySourceNode :

| Fichier | Type | Ajout |
|---|---|---|
| `filter.rs` | `FilterOp` | `Serialize, Deserialize` + `#[serde(tag = "op", content = "value")]` |
| `filter.rs` | `FilterValue` | `Serialize, Deserialize` + `#[serde(untagged)]` |
| `filter.rs` | `FilterCondition` | `Serialize, Deserialize` + `#[serde(rename_all)]` |
| `search.rs` | `Consistency` | `Deserialize` (avait déjà Serialize) |
| `search.rs` | `BM25Mode` | `Serialize, Deserialize` + `#[serde(rename_all)]` |
| `search.rs` | `FusionConfig` | `Serialize, Deserialize` |
| `search.rs` | `SearchOptions` | `Serialize, Deserialize` + `#[serde(default)]` |

#### Pré-requis : node_type() sur search nodes

Ajouté `node_type()` aux 4 search nodes (QuerySourceNode, PrimarySearchNode, FetchRelatedNode, ComposeNode) — retournaient "Unknown" par défaut. Ajouté `ComposeNode::new()` et `FetchRelatedNode::node_config()`.

#### Nouveau fichier `src/dataflow/node_factories.rs`

| Factory | Macro/Manuel | Node |
|---|---|---|
| `ComposeNodeFactory` | `simple_factory!` | ComposeNode |
| `PrimarySearchNodeFactory` | `simple_factory!` | PrimarySearchNode |
| `InsertRecordNodeFactory` | `named_factory!` | InsertRecordNode |
| `LinkRecordNodeFactory` | `named_factory!` | LinkRecordNode |
| `ChunkRecordNodeFactory` | `named_factory!` | ChunkRecordNode |
| `GatherKBNodeFactory` | `named_factory!` | GatherKBNode |
| `UpdateKBNodeFactory` | `named_factory!` | UpdateKBNode |
| `ChunkKBNodeFactory` | `named_factory!` | ChunkKBNode |
| `FlushFTSNodeFactory` | `named_factory!` | FlushFTSNode |
| `QuerySourceNodeFactory` | manuel (3 params) | QuerySourceNode |
| `FetchRelatedNodeFactory` | manuel (4 params) | FetchRelatedNode |
| `EmbedRecordNodeFactory` | manuel (1 param) | EmbedRecordNode |

Fonction `register_builtins(&mut NodeRegistry)` enregistre les 12. 11 tests unitaires.

Résultat : 0 warnings, 400 tests pass.

### #171 — Intégrer NodeRegistry dans checkpoint + catalog (TERMINÉ)

| Fichier | Changement |
|---|---|
| `catalog.rs` | Remplacé `create_node_from_checkpoint()` par `registry.create()` dans `drain_resume()` |
| `checkpoint.rs` | Supprimé `create_node_from_checkpoint()` + imports `record_nodes` devenus inutiles |
| `checkpoint.rs` (tests) | Tests migrés vers `builtin_registry()` helper |
| `mod.rs` | Retiré export `create_node_from_checkpoint` |

Résultat : 0 warnings, 400 tests pass.

### #172 — Helper DataflowGraph::add_from_registry() (TERMINÉ)

Ajouté `DataflowGraph::add_from_registry(&registry, node_type, name, config)` dans `graph.rs`. 3 tests : création + connexion, type inconnu, nom dupliqué.

Résultat : 0 warnings, 403 tests pass.

## État du plan (doc 07) — COMPLET

| # | Tâche | Statut |
|---|---|---|
| #167 | Supprimer DynamicNode + simplifier runtime | **FAIT** (doc 09) |
| #168 | Refactoring search nodes + expansion statique | **FAIT** (doc 09) |
| #169 | NodeSchema + NodeFactory + NodeRegistry | **FAIT** (doc 09) |
| #170 | Implémenter factories pour les 12 node types | **FAIT** |
| #171 | Intégrer NodeRegistry dans checkpoint + catalog | **FAIT** |
| #172 | Helper DataflowGraph::add_from_registry() | **FAIT** |

## Compteurs finaux

- `cargo check --lib` : 0 warnings
- `cargo test --lib` : 403 pass, 0 fail, 13 ignored
- Fichiers créés : 1 (`node_factories.rs`)
- Fichiers modifiés : 7 (`search_nodes.rs`, `filter.rs`, `search.rs`, `checkpoint.rs`, `catalog.rs`, `graph.rs`, `mod.rs`)

## Bonus : SearchOptions désormais Serialize + Deserialize

La cascade Serialize/Deserialize sur `FilterOp`, `FilterValue`, `FilterCondition`, `Consistency`, `BM25Mode`, `FusionConfig` et `SearchOptions` est utile au-delà des factories : API REST, config JSON, checkpoint, debug.

## Prochaines étapes possibles

1. **Mermaid templates** : parser Mermaid → `GraphDefinition` → `DataflowGraph` via `add_from_registry()`
2. **GraphNode** : nœud composite qui encapsule un sous-graphe (graph-in-graph)
3. **Regarder doc 10** (rapport d'une autre instance) pour aligner les travaux
