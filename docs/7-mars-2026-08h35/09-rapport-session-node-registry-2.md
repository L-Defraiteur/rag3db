# Doc 09 — Rapport de session : NodeRegistry Phase 3 (suite)

Date : 7 mars 2026

## Contexte

Suite au rapport doc 08 (DynamicNode suppression WIP), cette session finalise #167 et complète #168 et #169 du plan en 6 étapes (doc 07).

## Tâches complétées

### #167 — Supprimer DynamicNode + simplifier runtime (TERMINÉ)

Reprise du WIP : `cargo check --lib` révèle 6 warnings, tous corrigés :

| Fichier | Fix |
|---|---|
| `runtime.rs` | Supprimé import unused `GraphDefinition`, renommé `iteration` → `_iteration` |
| `search_nodes.rs` | Supprimé imports unused `SearchMeta`, `ExpansionRule`, `Edge` |
| `report.rs` | `let mut expanded_nodes` → `let expanded_nodes` |

Résultat : 0 warnings, 381 tests pass.

### #168 — Refactoring search nodes + expansion statique (TERMINÉ)

**Déviation du design doc 07** : pas de DispatchExpansionNode séparé. FetchRelatedNode prend directement `results` en input et filtre par `source_entity` en interne. Plus simple, même résultat, évite le problème de ports dynamiques avec `PortDef.name: &'static str`.

#### Fichiers modifiés

| Fichier | Changement |
|---|---|
| `services.rs` | Ajout `ConnService(pub Arc<dyn DbConnection>)` — wrapper pour ServiceRegistry (contourne `dyn Trait` non-Sized) |
| `search_nodes.rs` | **PrimarySearchNode** : supprimé champ `catalog`, résolu via `ctx.service::<Mutex<Catalog>>("catalog")`. **FetchRelatedNode** : supprimé `conn`/`parents`, ajouté `source_entity: Option<String>`, input port `results` (type Results), conn via `ctx.service::<ConnService>("conn")`. +2 tests ports |
| `catalog.rs` | `build_dataflow_graph()` retourne `(DataflowGraph, ServiceRegistry)`. Enregistre services `catalog` + `conn`. Construit graphe d'expansion statique : 1 FetchRelatedNode par rule + ComposeNode. Fan-out results, fan-in children. `search_with_strategy()` utilise `DataflowRuntime::with_services()` |
| `mod.rs` | Export `ConnService` |

#### Architecture du graphe d'expansion (statique)

```
query_source → primary_search ──results──→ fetch_related_0 ──children──→ compose
                               ──results──→ fetch_related_1 ──children──┘
                               ──results──────────────────────results──┘
```

- Fan-out : le runtime clone `results` pour chaque edge sortant
- Fan-in : `merge_port_values` fusionne les `Children` HashMaps sur le port `compose.children`

Résultat : 0 warnings, 383 tests pass.

### #169 — NodeSchema + NodeFactory + NodeRegistry (TERMINÉ)

Nouveau fichier `src/dataflow/node_registry.rs` :

| Type | Description |
|---|---|
| `ConfigParamType` | enum: String, Int, Float, Bool, Json |
| `ConfigParam` | struct: name, param_type, required, default, description |
| `NodeSchema` | struct: node_type, description, inputs, outputs, config_params |
| `NodeFactory` | trait: `create(name, config) → Result<Box<dyn Node>>`, `node_type()`, `schema()` |
| `NodeRegistry` | struct: `register()`, `create()`, `schema()`, `types()`, `has()` |
| `simple_factory!` | macro: génère factory pour nodes sans config (`Type::new()`) |
| `named_factory!` | macro: génère factory pour nodes avec nom (`Type::new(name)`) |

6 tests : create, unknown type, schema, types/has, macro.

Résultat : 0 warnings, 389 tests pass.

## État du plan

| # | Tâche | Statut |
|---|---|---|
| #167 | Supprimer DynamicNode + simplifier runtime | **FAIT** |
| #168 | Refactoring search nodes + expansion statique | **FAIT** |
| #169 | NodeSchema + NodeFactory + NodeRegistry | **FAIT** |
| #170 | Implémenter factories pour les 13 node types | pending |
| #171 | Intégrer NodeRegistry dans checkpoint + catalog | pending |
| #172 | Ajouter helper DataflowGraph::add_from_registry() | pending |

## Compteurs

- `cargo check --lib` : 0 warnings
- `cargo test --lib` : 389 pass, 0 fail, 13 ignored
- Fichiers créés : 1 (`node_registry.rs`)
- Fichiers modifiés : 5 (`services.rs`, `search_nodes.rs`, `catalog.rs`, `mod.rs`, `runtime.rs`, `report.rs`)

## Prochaine reprise

1. **#170** : Implémenter factories pour les 13 node types (macro `simple_factory!`/`named_factory!` pour ~10, manuel pour QuerySourceNode, FetchRelatedNode, EmbedRecordNode)
2. **#171** : Remplacer `create_node_from_checkpoint()` par `registry.create()`
3. **#172** : Helper `DataflowGraph::add_from_registry()`

### Constructeurs actuels des nodes (référence pour #170)

| Node | Constructeur | Factory type |
|---|---|---|
| ComposeNode | `ComposeNode` (unit struct) | `simple_factory!` |
| PrimarySearchNode | `PrimarySearchNode::new()` | `simple_factory!` |
| QuerySourceNode | `new(kb_name, query, options)` | manuel (3 config params) |
| FetchRelatedNode | `new(name, relation, direction, limit, source_entity)` | manuel (4 config params) |
| InsertRecordNode | `new(name)` | `named_factory!` |
| LinkRecordNode | `new(name)` | `named_factory!` |
| EmbedRecordNode | `new(name)` ou `new_with_config(name, batch_size)` | manuel (1 config param) |
| ChunkRecordNode | `new(name)` | `named_factory!` |
| GatherKBNode | `new(name)` | `named_factory!` |
| UpdateKBNode | `new(name)` | `named_factory!` |
| ChunkKBNode | `new(name)` | `named_factory!` |
| FlushFTSNode | `new(name)` | `named_factory!` |
