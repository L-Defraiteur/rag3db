# Doc 26 — Session : Phase B — Record-Based Ingestion Nodes

Date : 7 mars 2026

## Contexte

Le doc 25 a implémenté la Phase A du doc 23 (élimination des ops) : `records.rs` avec `EntityRecord`, `RelationRecord`, `AggregateRecord`, `PendingWork`, plus les PortType variants `Entities`/`Relations` et le shadow recording dans `catalog.rs`. Cette session implémente la Phase B : les 5 nouveaux nœuds record-based.

## Travail effectué

### 1. Création de `src/dataflow/record_nodes.rs` (1554 lignes)

5 nœuds qui prennent des records typés au lieu d'ops. Même logique UNWIND que les batch nodes existants, mais avec `EntityRecord`/`RelationRecord`/`AggregateRecord` en input.

#### 1.1 `InsertRecordNode` (Node)

- **Input** : `entities` — `BatchPayload<EntityRecord>` (PortType::Entities)
- **Output** : `done` — Empty
- **Services** : `conn`, `node_id_cache`
- Même pattern que `InsertBatchNode` : groupe par `(entity_name, column_set)`, UNWIND CREATE, résout les EntityRefs, cache les node IDs
- Différence : prend `Vec<EntityRecord>` au lieu de `Vec<InsertOp>`, utilise `rec.take_resolver()` au lieu de `insert.take_resolver()`

#### 1.2 `LinkRecordNode` (Node)

- **Input** : `relations` — `BatchPayload<RelationRecord>` (PortType::Relations)
- **Output** : `done` — Empty
- **Services** : `conn`
- Même pattern que `LinkBatchNode` : résout from/to refs, groupe par `(rel_name, property_keys)`, UNWIND MATCH+CREATE
- Différence : prend `Vec<RelationRecord>` au lieu de `Vec<LinkOp>`

#### 1.3 `EmbedRecordNode` (Node) — fusion de 3 nœuds

- **Input** : `entities` — `BatchPayload<EntityRecord>` (PortType::Entities)
- **Output** : `done` — Empty
- **Services** : `conn`, `embedder`, `embedding_dim`, `config`, `kb_metadata`, optionnel `sparse_embedder`, `dual_embedder`
- **Changement majeur** : fusionne `EmbedBatchNode` + `SparseEmbedBatchNode` + `DualEmbedBatchNode` en un seul nœud
- Ne reçoit plus d'instructions explicites (pas d'`EmbedOp`). Reçoit des `EntityRecord` et décide lui-même :
  1. Pour chaque entity, trouve les KBs qui la référencent via `kb_metadata`
  2. Extrait les textes des content fields de l'entity
  3. Décide dense/sparse/dual via les signals KB et la présence des embedders
- Produit 3 listes de travail : `dense_works`, `sparse_works`, `dual_works`
- GPU mini-batching pour dual (paramètre `gpu_batch_size`)
- UNWIND SET par groupe `(entity_name, embedding_col)` pour dense, `(entity_name, kb_name)` pour sparse

#### 1.4 `ChunkRecordNode` (DynamicNode)

- **Input** : `entities` — `BatchPayload<EntityRecord>` (PortType::Entities)
- **Output** : `done` — Empty
- **Services** : `config`, `kb_metadata`, `chunker_cache`
- Remplace `ChunkBatchNode` : même chunking parallèle via rayon
- **Nouvelle méthode** `compute_chunks()` remplace `compute_chunk_ops()` : retourne `(Vec<EntityRecord>, Vec<RelationRecord>)` au lieu de `Vec<CatalogOp>`
- Émet en downstream : `InsertRecordNode("chunk_inserts")` + `LinkRecordNode("chunk_links")`
- Les chunks ont leur `entity_ref` pré-résolu (UUID déterministe via `chunk_uuid()`)

#### 1.5 `AggregateRecordNode` (DynamicNode)

- **Input** : `aggregates` — `BatchPayload<AggregateRecord>` (PortType::Aggregates)
- **Output** : `done` — Empty
- **Services** : `conn`, `config`, `kb_metadata`, `chunker_cache`
- Remplace `AggregateBatchNode` : même logique UNWIND 7 étapes (read titles, read linked content, read hashes, compute changed, UPDATE, DELETE, re-chunk)
- **Nouvelle méthode** `generate_chunk_records()` remplace `generate_chunk_ops()` : retourne `(Vec<EntityRecord>, Vec<RelationRecord>)` avec `_content_offset`, `_source_field`, `_source_entity`, `_source_uuid`
- Émet en downstream : `InsertRecordNode("agg_inserts")` + `LinkRecordNode("agg_links")`
- Structs internes renommés `RecordSourceContent`/`RecordAggState` pour éviter conflit avec ceux de `ingestion_nodes.rs`

### 2. Mise à jour de `src/dataflow/mod.rs`

- Ajout `pub mod record_nodes;`
- Export des 5 nœuds : `InsertRecordNode`, `LinkRecordNode`, `EmbedRecordNode`, `ChunkRecordNode`, `AggregateRecordNode`

## Fichiers modifiés

| Fichier | Changement |
|---|---|
| `src/dataflow/record_nodes.rs` | **Nouveau** — 5 nœuds record-based (1554 lignes) |
| `src/dataflow/mod.rs` | +1 module, +5 exports |

## Validation

- `cargo check --lib` : compile clean (warnings pré-existants uniquement)
- `cargo test --lib` : **392 pass**, 0 fail (0 régression)

## Architecture — Mapping ancien → nouveau

| Ancien (ops) | Nouveau (records) | Changement |
|---|---|---|
| `SplitOpsNode` | *(supprimé en Phase C)* | Plus de routeur — données typées dès la source |
| `InsertBatchNode` | `InsertRecordNode` | `Vec<InsertOp>` → `Vec<EntityRecord>` |
| `LinkBatchNode` | `LinkRecordNode` | `Vec<LinkOp>` → `Vec<RelationRecord>` |
| `EmbedBatchNode` | `EmbedRecordNode` | Fusionné (dense+sparse+dual) |
| `SparseEmbedBatchNode` | *(fusionné)* | → `EmbedRecordNode` |
| `DualEmbedBatchNode` | *(fusionné)* | → `EmbedRecordNode` |
| `ChunkBatchNode` | `ChunkRecordNode` | `Vec<ChunkOp>` → `Vec<EntityRecord>` |
| `AggregateBatchNode` | `AggregateRecordNode` | `Vec<AggregateOp>` → `Vec<AggregateRecord>` |

## Limitation actuelle — outputs `done` uniquement + DynamicNode inutile

Les 5 nœuds n'ont que `done: Empty` en output, copié du pattern des anciens `BatchNode`. C'est insuffisant : le doc 23 spécifie que `InsertRecordNode` devrait avoir **deux** outputs :

- `done: Empty` — signal trigger pour `LinkRecordNode`
- `inserted: Vec<EntityRecord>` — les entités avec refs résolues, pour `ChunkRecordNode` et `EmbedRecordNode`

Actuellement les données downstream passent par `set_initial_input()` (baked-in data dans les DynamicNodes). Le vrai design doc 23, c'est que les données circulent via les edges du graphe — c'est tout le principe "le graphe EST le plan".

**Conséquence** : `ChunkRecordNode` et `AggregateRecordNode` n'ont plus besoin d'être des `DynamicNode`. L'ancien pattern DynamicNode existait parce que les données étaient baked-in via `set_initial_input()` — le nœud devait émettre ses nœuds downstream et leur pré-charger les données. Si les données circulent sur les ports output, le graphe est entièrement câblé à l'avance dans `build_ingestion_graph()`, et tous les nœuds deviennent des `Node` statiques :

```
build_ingestion_graph() câble :

InsertRecordNode --inserted--> ChunkRecordNode --chunks-------> InsertRecordNode("chunk_inserts")
                 --inserted--> EmbedRecordNode                  --chunk_links--> LinkRecordNode("chunk_links")
                 --done------> LinkRecordNode                   chunk_inserts --done--> EmbedRecordNode("chunk_embeds")
                               --done--> AggregateRecordNode --entities--> InsertRecordNode("agg_inserts")
                                                              --relations-> LinkRecordNode("agg_links")
                                                              agg_inserts --done--> EmbedRecordNode("agg_embeds")
```

Tout est statique, connu au moment de `build_ingestion_graph()`. Les DynamicNodes ne sont nécessaires que quand le nombre de nœuds downstream dépend des données runtime (ex: `ExpansionNode` dans search qui émet N `FetchRelatedNode` selon les résultats). L'ingestion record-based n'a pas ce cas.

## Prochaine étape

**Étape immédiate** — Convertir les 5 nœuds en `Node` statiques avec vrais ports output :

1. `InsertRecordNode` : ajouter output `inserted: Vec<EntityRecord>` (PortType::Entities)
2. `ChunkRecordNode` : convertir de `DynamicNode` → `Node`, outputs `chunks: Vec<EntityRecord>` + `chunk_links: Vec<RelationRecord>`
3. `AggregateRecordNode` : convertir de `DynamicNode` → `Node`, outputs `entities: Vec<EntityRecord>` + `relations: Vec<RelationRecord>`
4. Supprimer tout le code `emitter.add_node()` / `emitter.set_initial_input()` / `emitter.connect()` dans ces nœuds

**Puis Phase C** (doc 23) — Switch `build_ingestion_graph()` + `drain()` vers `PendingWork` :

1. Réécrire `build_ingestion_graph()` pour consommer `self.pending` (PendingWork) au lieu de `self.pending_ops` (Vec<CatalogOp>)
2. Câbler le graphe complet statiquement : tous les nœuds et edges connus à l'avance
3. Plus de `SplitOpsNode` — les données sont typées dès la source
4. Plus de `DynamicNode` côté ingestion — tout est statique
5. Réécrire `flush_insertions()` pour utiliser `InsertRecordNode`
6. Supprimer le shadow recording dans `create()`/`link()` (les records deviennent la seule source)
7. Supprimer `pending_ops` du struct `Catalog`

Puis **Phase D** — suppression du code mort (ops, anciens nœuds, anciens PortType variants).
