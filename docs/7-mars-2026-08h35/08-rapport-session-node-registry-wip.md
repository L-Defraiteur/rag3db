# Doc 08 — Rapport de session : NodeRegistry WIP (DynamicNode suppression)

Date : 7 mars 2026

## Contexte

Suite au checkpoint (doc 06), on attaque la Phase 3 du roadmap (doc 01) : **NodeRegistry + Mermaid + GraphNode**. On commence par le NodeRegistry (doc 07 = design).

Décision prise en session : **supprimer DynamicNode entièrement**. Le seul DynamicNode en production (ExpansionNode) peut être remplacé par un pattern statique (DispatchExpansionNode + FetchRelatedNode avec input port pour parents).

## Plan en 6 étapes (tâches #167-#172)

| # | Tâche | Statut |
|---|---|---|
| #167 | Supprimer DynamicNode + simplifier runtime | **EN COURS** |
| #168 | Créer DispatchExpansionNode + refactorer search nodes | pending |
| #169 | Implémenter NodeSchema + NodeFactory + NodeRegistry | pending |
| #170 | Implémenter factories pour les 13 node types | pending |
| #171 | Intégrer NodeRegistry dans checkpoint + catalog | pending |
| #172 | Ajouter helper DataflowGraph::add_from_registry() | pending |

## Travail effectué — Étape #167 (en cours, non compilé)

### Fichiers modifiés

| Fichier | Changement |
|---|---|
| `src/dataflow/node.rs` | Supprimé `DynamicNode` trait (~25 lignes), `GraphEmitter` struct (~80 lignes), test `graph_emitter_drain` |
| `src/dataflow/graph.rs` | Supprimé `NodeSlot` enum, `add_dynamic_node()`, `merge_dynamic()`. `nodes` est maintenant `Vec<Box<dyn Node>>` directement. `find_node()` retourne `Option<&dyn Node>` |
| `src/dataflow/runtime.rs` | Supprimé import `NodeSlot`/`GraphEmitter`, supprimé `GraphExpanded` event variant, supprimé branche DynamicNode dans `execute()` (remplacée par appel direct `node.execute()`), supprimé branche DynamicNode dans `execute_inner_with_checkpoint()`, supprimé `GraphExpanded` dans `NodeEventFilter::matches()`, supprimé 2 tests (`runtime_dynamic_node`, `runtime_max_iterations`) |
| `src/dataflow/checkpoint.rs` | Simplifié `to_definition()` — plus de match `NodeSlot::Static`/`Dynamic`, itère directement sur `Vec<Box<dyn Node>>` |
| `src/dataflow/search_nodes.rs` | Supprimé `ExpansionNode` struct + impl DynamicNode (~100 lignes), supprimé 3 tests (`expansion_emits_fetch_and_compose`, `expansion_no_match_passthrough`, `expansion_dedup_sources`), supprimé imports `DynamicNode`/`GraphEmitter` |
| `src/dataflow/report.rs` | Supprimé handling `DataflowEvent::GraphExpanded` dans `ExecutionReport::build()` |
| `src/dataflow/mod.rs` | Supprimé exports `DynamicNode`, `GraphEmitter`, `ExpansionNode` |
| `src/catalog.rs` | Supprimé `add_dynamic_node(ExpansionNode)` dans `build_dataflow_graph()`, remplacé par TODO pour DispatchExpansionNode (étape #168), supprimé variable `conn` devenue unused |

### Ce qui reste à faire pour #167

- **`cargo check --lib`** — pas encore lancé, potentiellement des erreurs de compilation restantes (imports unused, types qui ne matchent plus)
- Vérifier que `search_nodes.rs` n'a pas d'imports unused (`HashSet`, `source_info`, `ExpansionDirection`, `ExpansionRule` — potentiellement encore utilisés par FetchRelatedNode ou les tests)
- **`cargo test --lib`** — valider que les tests restants passent

### Suppression vérifiée (grep = 0 match)

```
DynamicNode     → 0
GraphEmitter    → 0
NodeSlot        → 0
add_dynamic_node → 0
merge_dynamic   → 0
GraphExpanded   → 0
ExpansionNode   → 0 (hors commentaires TODO)
```

## Documents créés cette session

- **Doc 07** : `07-design-node-registry.md` — Design complet NodeRegistry + suppression DynamicNode, 6 étapes, refactoring search nodes, intégration checkpoint
- **Doc 08** : ce fichier

## Prochaine reprise

1. Finir #167 : `cargo check --lib`, fixer les erreurs, `cargo test --lib`
2. Étape #168 : DispatchExpansionNode + refactoring FetchRelatedNode/PrimarySearchNode
