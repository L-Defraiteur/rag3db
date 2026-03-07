# Doc 14 — Rapport de session : Mermaid + GraphNode + Templates

Date : 7 mars 2026

## Résumé

Phase 3 complète : parser Mermaid, `from_definition()`, GraphNode composable, templates `.mmd` built-in, refactor noms paramétrables des search nodes.

## Travail réalisé

### Phase 3b — Mermaid parser (`22ed1dee7`)

**Nouveau fichier** : `src/dataflow/mermaid.rs` (~300 lignes, 17 tests)

- `parse_mermaid(input)` → `GraphDefinition` : parser hand-rolled, zero dépendance externe
- `parse_mermaid_template(input, vars)` : substitution `$variable` dans les valeurs de config
- `to_mermaid(def)` : export Mermaid avec round-trip support
- `MermaidError` : 5 variantes (MissingHeader, InvalidNodeDecl, InvalidEdge, UnknownVariable, UnparsableLine)

Syntaxe supportée :
- `graph LR|TD` header
- `id["NodeType(key='value', key=N, key=true)"]` — déclaration de nœud
- `from -->|port| to` — edge shorthand (même port)
- `from -->|from_port:to_port| to` — edge explicite
- `$var` dans les valeurs string
- `%% commentaire` (pleine ligne et inline)

**Modifié** : `src/dataflow/graph.rs` — ajout `DataflowGraph::from_definition(&GraphDefinition, &NodeRegistry)`

### Phase 3c — GraphNode (`13866da77`)

**Nouveau fichier** : `src/dataflow/graph_node.rs` (~280 lignes, 11 tests)

- `GraphNode` : nœud qui wraps un sous-graphe, implémente `Node`
- Ports libres = ports d'entrée/sortie non-connectés dans le sous-graphe interne
- Nommage : `inner_node.port_name` (ex: `ps.results`)
- `alias_input()/alias_output()` pour renommer les ports exposés
- Exécution : matérialise le sous-graphe via `from_definition()`, injecte les inputs, exécute via `DataflowRuntime` interne, collecte les outputs
- Services partagés via `ctx.services()` → `DataflowRuntime::with_services_arc()`

**`GraphNodeFactory`** : factory enregistrable dans le `NodeRegistry` pour templates réutilisables

**Modifié** :
- `node.rs` — ajout `NodeContext::services()` accessor
- `runtime.rs` — ajout `DataflowRuntime::with_services_arc(Arc<ServiceRegistry>)`
- `mod.rs` — wire module + exports

### Templates `.mmd` built-in (`3c6ae2a36`)

**Nouveau répertoire** : `templates/`

| Template | Nœuds | Variables | Usage |
|----------|-------|-----------|-------|
| `search.mmd` | 2 | `$kb_name`, `$query` | Recherche simple |
| `search_expansion.mmd` | 4 | +`$relation`, `$direction`, `$limit` | Recherche + expansion graph |
| `ingestion.mmd` | 10 | `$gpu_batch_size` | Pipeline d'ingestion complet |
| `kb_pipeline.mmd` | 7 | `$gpu_batch_size` | Sous-graphe KB (composable via GraphNode) |

**Refactor search nodes** : QuerySourceNode, PrimarySearchNode, ComposeNode acceptent maintenant un nom d'instance paramétrable (comme les record nodes). Les factories passent le nom du Mermaid.

5 tests template : parse + build graph + validate + topo sort + GraphNode wrapping.

## Tests

| Suite | Avant | Après |
|-------|-------|-------|
| Unit tests (`cargo test --lib`) | 420 | 436 (+16) |
| E2E (`run_e2e.sh`) | 89 | 89 |
| Failures | 0 | 0 |

## Commits

1. `22ed1dee7` — feat: Mermaid parser + from_definition() + GraphNode design doc
2. `13866da77` — feat: GraphNode — composable sub-graph node
3. `3c6ae2a36` — feat: built-in Mermaid templates + paramétrable search node names

## Phase 3 — Status final

| Élément | Status |
|---------|--------|
| NodeRegistry + 12 factories | ✅ (session précédente) |
| Mermaid parser (parse + template + export) | ✅ |
| `from_definition()` | ✅ |
| GraphNode (sous-graphe composable) | ✅ |
| GraphNodeFactory | ✅ |
| 4 templates `.mmd` built-in | ✅ |
| Search nodes noms paramétrables | ✅ |

## Prochaine étape

**Phase 4 — Migrations** : nœuds migration (QueryNode, BackupNode, ValidateNode, TransformNode, WriteNode), `MigrationRunner`, schema `_DataflowMigration`, convention `migrations/*.mmd`.
