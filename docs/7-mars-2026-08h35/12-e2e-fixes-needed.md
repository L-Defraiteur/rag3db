# Doc 12 — Problèmes E2E à corriger

Date : 7 mars 2026

## Contexte

Après complétion du plan doc 07 (6/6 tâches, 403 unit tests OK), les tests E2E révèlent 2 catégories de problèmes.

## 1. Tests E2E corrigés (FAIT)

`build_dataflow_graph()` retourne maintenant `(DataflowGraph, ServiceRegistry)` au lieu de `DataflowGraph` (changement #168).

**Fichiers corrigés :**
- `tests/e2e_search_queue.rs` — 1 occurrence
- `tests/e2e_dataflow_observe.rs` — 8 occurrences

Pattern : `let mut graph = ...` → `let (mut graph, services) = ...` et `DataflowRuntime::new(10)` → `DataflowRuntime::with_services(10, services)`.

## 2. Tests E2E encore en échec (À FAIRE)

### 2a. `observe_execute_with_report_expansion` — FAIL

**Fichier :** `tests/e2e_dataflow_observe.rs:297`

**Cause :** Le test vérifie `report.expanded_nodes.is_empty()` — concept de DynamicNode supprimé en #167. L'expansion est maintenant statique (FetchRelatedNode pré-construit dans le graphe par `build_dataflow_graph()`).

**Fix :** Adapter le test :
- Supprimer l'assertion `expanded_nodes` (toujours vide maintenant)
- Garder les assertions sur `nodes.len() >= 4` (query_source, primary_search, fetch_related_0, compose)
- Garder les assertions sur les edges vers compose

### 2b. `observe_record_database` — FAIL

**Fichier :** `tests/e2e_dataflow_observe.rs:504`

**Cause :** `Binder exception: Cannot find property pipeline_name for e.` — la table `_DataflowExecution` a été créée avec un ancien schéma (sans `pipeline_name`). `CREATE NODE TABLE IF NOT EXISTS` ne met pas à jour les colonnes manquantes. Problème pré-existant (pas lié à nos changements).

**Fix :** Le test utilise une DB in-memory neuve → le schéma devrait être correct. Investiguer si la DB est partagée entre tests ou si le schéma DDL a un bug.

## 3. Extension lucivy rebuild (FAIT)

`cmake --build . --target rag3db_lucivy_fts_extension` OK — `lucivy-core` compilé correctement après l'extraction doc 10.

## Résultat E2E actuel

- `e2e_batch_observe` : 2/2 OK
- `e2e_checkpoint` : 3/3 OK
- `e2e_dataflow_observe` : **5/7 OK, 2 FAIL**
- `e2e_native` : OK
- `e2e_phase0b` : OK
- `e2e_result_mode` : OK
- `e2e_search_queue` : OK
- `e2e_search` : OK
