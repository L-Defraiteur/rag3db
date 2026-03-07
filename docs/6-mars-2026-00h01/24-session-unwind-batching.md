# Doc 24 — Session : UNWIND Batching + Observabilité

Date : 7 mars 2026

## Contexte

Le doc 23 a identifié la dette historique : INSERT et LINK n'ont **jamais** été batchés (1 Cypher par entité depuis le premier commit `67a87123f`). Seuls les embed nodes utilisaient UNWIND.

Cette session implémente le batching systématique sur les nœuds existants (Phase 2) et ajoute l'observabilité pour prouver que ça fonctionne.

## Pré-requis validé

- Phase 2 (doc 22) complète : 385 unit tests, 84 E2E, 0 fail
- Doc 23 écrit et validé

## Travail effectué

### 1. UNWIND batching — InsertBatchNode

Réécriture complète de `InsertBatchNode::execute()` :

1. **Groupement** : `HashMap<(entity_name, sorted_columns), Vec<usize>>` — les entités avec le même nom et les mêmes colonnes vont dans le même UNWIND
2. **Cypher** : `UNWIND $items AS item CREATE (n:{entity} {{col: item.col, ...}}) RETURN ID(n), item._uuid`
3. **Résolution safe** : matcher par UUID dans les résultats (pas par position de row) — `HashMap<uuid, node_id>`

### 2. UNWIND batching — LinkBatchNode

Réécriture complète de `LinkBatchNode::execute()` :

1. **Resolve all refs** en amont (devrait être instantané — InsertBatchNode déjà completed)
2. **Groupement** : `HashMap<(rel_name, sorted_property_keys), Vec<usize>>`
3. **Cypher** : `UNWIND $items AS item MATCH (a {_uuid: item.from_uuid}), (b {_uuid: item.to_uuid}) CREATE (a)-[:REL {props}]->(b)`

### 3. Observabilité — eprintln! sur tous les nœuds

Ajout de logs batch sur **7 nœuds** :

| Nœud | Log |
|---|---|
| `SplitOpsNode` | `routed: inserts=N, links=N, chunks=N, aggregates=N, embeds=N, sparse=N, dual=N` |
| `InsertBatchNode` | `N items → K UNWIND groups: [Entity×M, ...]` |
| `LinkBatchNode` | `N links → K UNWIND groups: [Rel×M, ...]` |
| `EmbedBatchNode` | `N texts embedded → K UNWIND groups: [Entity.col×M, ...]` |
| `SparseEmbedBatchNode` | `N texts → K UNWIND groups: [Entity.kb×M, ...]` |
| `DualEmbedBatchNode` | `N texts, gpu_batch_size=B` |
| `ChunkBatchNode` | `N chunk_ops → downstream: inserts=X, links=Y, embeds=Z, ...` |
| `AggregateBatchNode` | `N ops (U unique) → downstream: inserts=X, links=Y, embeds=Z, ...` |

### 4. E2E test — e2e_batch_observe.rs

Nouveau fichier de test avec 2 scénarios :

**Test 1 — Multi-entity (5 File + 5 Document, HYBRID KB)** :

Config : File (titleFor FileKB) + Document (contentFor FileKB) + HAS_DOCUMENT relation. KB = dense + BM25. C'est le pire cas pour le batching (multi-entity, multi-relation, agrégation cross-entity).

Résultat observé :

```
[SplitOpsNode] routed: inserts=15, links=10, chunks=0, aggregates=5, embeds=0, sparse=0, dual=0
[InsertBatchNode:inserts] 15 items → 3 UNWIND groups: [File×5, FileKB_Index×5, Document×5]
[LinkBatchNode:links] 10 links → 2 UNWIND groups: [File_IN_FileKB×5, HAS_DOCUMENT×5]
[AggregateBatchNode] 5 ops (5 unique) → downstream: inserts=5, links=10, embeds=5
[InsertBatchNode:agg_inserts] 5 items → 1 UNWIND groups: [FileKB_Index_Chunk×5]
[LinkBatchNode:agg_links] 10 links → 2 UNWIND groups: [Document_SOURCED_FileKB×5, FileKB_Index_HAS_CHUNK×5]
[EmbedBatchNode:agg_embeds] 5 texts embedded → 1 UNWIND groups: [FileKB_Index_Chunk.FileKB_embedding×5]
```

Bilan : **20 entités + 20 relations = 40 queries sans batching → 9 UNWIND queries avec batching** (×4.4 gain).

**Test 2 — Single entity (10 Files)** :

```
[InsertBatchNode:inserts] 20 items → 2 UNWIND groups: [File×10, FileKB_Index×10]
[LinkBatchNode:links] 10 links → 1 UNWIND groups: [File_IN_FileKB×10]
```

Bilan : **30 entités + 10 relations = 40 queries sans batching → 3 UNWIND queries** (×13 gain).

### 5. UNWIND batching — AggregateBatchNode

Réécriture complète de `AggregateBatchNode`. Avant : `process_one()` séquentiel (5 queries par aggregate op). Après : `process_batch()` par groupe `(title_entity, kb_name)`.

**Architecture batch :**

1. **Groupement** : `HashMap<(title_entity, kb_name), Vec<&AggregateOp>>` — même structure de champs = même UNWIND
2. **UNWIND read titres** : 1 query par groupe → récupère titre + content fields sur l'entité titre
3. **UNWIND read contenu lié** : 1 query par type de content entity lié (via relation)
4. **UNWIND read hashes** : 1 query par groupe → compare hash courant avec nouveau
5. **Filtrage Rust** : hash identique = skip (pas de re-chunk)
6. **UNWIND UPDATE** : 1 query pour tous les index modifiés du groupe
7. **UNWIND DELETE** : 1 query pour supprimer les vieux chunks du groupe
8. **Re-chunk** (CPU) : `generate_chunk_ops()` extrait en méthode pure (pas de DB)

**Gains mesurés :**

| Cas | Avant (séquentiel) | Après (UNWIND) | Gain |
|---|---|---|---|
| 5 File@FileKB | 25 queries | 5 queries | ×5 |
| 10 File@FileKB | 50 queries | 5 queries | ×10 |
| 7 ops (2 groups) | 35 queries | 8 queries | ×4.4 |
| 4 Document@main | 16 queries | 4 queries | ×4 |

**Observabilité enrichie :**

```
[AggregateBatchNode] 5 ops (5 unique) → 1 UNWIND groups: [File@FileKB×5], 5 queries, 0 skipped (unchanged),
 downstream: inserts=5, links=10, embeds=5, sparse=0, dual=0
```

Le log montre maintenant : nombre de groupes UNWIND, nombre réel de queries DB, et nombre d'aggregates skippés (hash unchanged).

### 6. run_e2e.sh — toutes les suites par défaut

Modifié pour itérer tous les `tests/e2e_*.rs` quand aucun `--test` n'est spécifié (avant : hardcodé `e2e_search` uniquement).

## Fichiers modifiés

| Fichier | Changement |
|---|---|
| `src/dataflow/ingestion_nodes.rs` | UNWIND batching sur InsertBatchNode + LinkBatchNode + AggregateBatchNode, eprintln! sur 7 nœuds |
| `tests/e2e_batch_observe.rs` | **Nouveau** — 2 tests multi-entity batching observability |
| `run_e2e.sh` | Itère tous les `e2e_*.rs` par défaut |

## Validation

- `cargo test --lib` : **385 pass**, 0 fail
- Tous les E2E (7 suites, 86 tests) : **86 pass**, 0 fail

## Prochaine étape

Le doc 23 décrit l'élimination des ops en 4 phases (A-D). Le batching est maintenant complet sur **tous** les nœuds existants. La prochaine étape est l'implémentation du vrai comportement graphe pour l'ingestion :

- **Phase A** : `EntityRecord`, `RelationRecord`, `AggregateRecord` + `PendingWork`
- **Phase B** : Nouveaux nœuds (`InsertNode`, `LinkNode`, `EmbedNode` unifié, `ChunkNode`, `AggregateNode`)
- **Phase C** : Nouveau `build_ingestion_graph()` sans `SplitOpsNode`
- **Phase D** : Suppression des ops, des anciens nœuds, des 8 PortType variants

Le batching implémenté ici sera conservé tel quel dans les nouveaux nœuds — seul le type d'input change (`EntityRecord` au lieu de `InsertOp`).
