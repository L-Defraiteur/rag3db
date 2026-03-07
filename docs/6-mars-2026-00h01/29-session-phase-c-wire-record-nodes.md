# Doc 29 — Session : Phase C — Câbler les Record Nodes dans le pipeline

Date : 7 mars 2026

## Contexte

Les docs 26-28 ont implémenté les 5 record nodes (Insert, Link, Embed, Chunk, Aggregate), les ont convertis en nœuds statiques avec metrics, et ajouté l'idempotence (MERGE + `_embed_hash`). Mais `build_ingestion_graph()` consommait toujours `pending_ops` (Vec<CatalogOp>) via les anciens batch nodes. Phase C remplace ce câblage.

## Décisions de design

### 1. Pas d'EmbedRecordNode sur les raw entities

L'exploration du code a confirmé que **les raw entities (File, DocPage, etc.) ne sont jamais embeddées ni recherchées**. Le pipeline d'ingestion ancien :
1. Raw entities → insérées, **PAS embeddées**
2. AggregateOp → crée `{KB}_Index` + `{KB}_Index_Chunk`
3. Seuls les `{KB}_Index_Chunk` sont embeddés (vector/sparse/dual)

La recherche cible uniquement `{KB}_Index_Chunk` (vector/sparse) et `{KB}_Index` (BM25). Donc pas d'`EmbedRecordNode("entity_embeds")` dans la topologie.

### 2. Pas de ChunkRecordNode

`AggregateRecordNode` fait le chunking en interne via `generate_chunk_records()` — il n'a pas besoin de `ChunkRecordNode` en aval. `ChunkRecordNode` sert uniquement au futur template Mermaid "simple" (Insert → Chunk → Embed, sans KB Index).

Analyse de décomposabilité : AggregateRecordNode n'est pas décomposable en nœuds existants car il gère :
- Lecture cross-entity (N queries DB pour titre + contenu lié)
- Hash change detection (`_content_hash`)
- Suppression des anciens chunks (DETACH DELETE)
- Mise à jour du KB_Index (SET _title, _content, _content_hash)
- Chunking multi-source avec `_content_offset` accumulé
- Relations SOURCED (source_entity → chunk)

Aucune de ces opérations n'est couverte par ChunkRecordNode ou InsertRecordNode.

### 3. Topologie cible

```
PendingWork.entities → InsertRecordNode("inserts")
                            └── done → LinkRecordNode("links") ← PendingWork.relations
                                          └── done → AggregateRecordNode("aggregate") ← PendingWork.aggregates
                                                        ├── entities → InsertRecordNode("agg_inserts")
                                                        ├── relations → LinkRecordNode("agg_links")
                                                        │                    ↑ trigger (agg_inserts.done)
                                                        └── agg_inserts ── inserted → EmbedRecordNode("agg_embeds")
                                                                                        ↑ trigger (agg_links.done)
```

## Travail effectué

### C.1 — `create()` : PendingWork avec resolvers

**Avant** : `create()` poussait des `CatalogOp` dans `pending_ops` (avec EntityRefResolver) + des shadow records dans `pending` (sans resolver).

**Après** : `create()` pousse uniquement dans `self.pending` avec les vrais resolvers :
- `EntityRecord::new(...)` avec resolver via `EntityRef::new()`
- `RelationRecord::new(...)` avec resolver via `RelationRef::new()`
- `AggregateRecord` dans `self.pending.aggregates`

Plus de duplication ops/shadow.

### C.2 — `link()` : même migration

Supprimé les `CatalogOp::Link` + shadow records. Pousse directement `RelationRecord::new(...)` avec resolver + `AggregateRecord` pour le cas incrémental.

### C.3 — `update()` et `delete()` : aggregates directs

`update()` et `delete()` font des opérations DB directes puis poussent des `AggregateRecord` directement dans `self.pending.aggregates` au lieu de créer des `CatalogOp::Aggregate`.

### C.4 — `build_ingestion_graph()` : nouveau graphe record-based

Réécrit pour consommer `self.pending` (PendingWork) :
1. `std::mem::take(&mut self.pending)` pour récupérer le PendingWork
2. Graphe conditionnel :
   - Si entities non vide → `InsertRecordNode("inserts")`
   - Si relations non vide → `LinkRecordNode("links")`, triggered after inserts
   - Si aggregates non vide → `AggregateRecordNode("aggregate")` + downstream (agg_inserts, agg_links, agg_embeds)
3. Services enregistrés (conn, node_id_cache, embedder, embedding_dim, config, kb_metadata, chunker_cache, sparse_embedder, dual_embedder, has_sparse, has_dual)
4. Initial inputs via `graph.set_initial_input()` sur chaque nœud racine

### C.5 — `flush_insertions()` : utilise PendingWork

Extrait `self.pending.entities` (laisse relations/aggregates en place), construit un mini-graphe avec `InsertRecordNode`.

### C.6 — `has_pending()` et `queue_stats()`

- `has_pending()` → `!self.pending.is_empty()`
- `queue_stats().pending` → `self.pending.total_count()`

### C.7 — Suppression de `pending_ops`

- Champ `pending_ops: Vec<CatalogOp>` supprimé de `struct Catalog`
- Import `CatalogOp` gardé pour `compute_chunk_ops()` (utilisé par les anciens batch nodes, supprimé en Phase D)

### C.8 — Borrow checker fix

`check_entity()` retourne `&EntityDef` qui emprunte `self`. Comme `create()` doit ensuite muter `self.pending`, on clone `entity_def` pour libérer l'emprunt : `let entity_def = self.check_entity(entity_name)?.clone();`

### C.9 — AggregateRecordNode : always set output ports

**Bug** : Deadlock `nodes ["agg_inserts", "agg_links", "agg_embeds"] cannot execute`.

**Cause** : `AggregateRecordNode` ne posait `ctx.set_output("entities", ...)` et `ctx.set_output("relations", ...)` que si les vecs étaient non-vides. Avec `MockConnection` (résultats vides pour toutes les queries DB), `process_batch()` retourne des vecs vides → les output ports ne sont jamais posés → les downstream nodes (agg_inserts, agg_links, agg_embeds) ont `required: true` sur leur input data → attendent indéfiniment → deadlock.

**Fix** : Toujours poser les output ports, même avec des vecs vides. Les downstream nodes traitent 0 éléments → no-op.

```rust
// Avant (conditionnel)
if !all_entities.is_empty() {
    ctx.set_output("entities", ...);
}
// Après (toujours)
ctx.set_output("entities", PortValue::Batch(
    BatchPayload::new(PortType::Entities, all_entities),
));
```

## Validation

- `cargo check --lib` : compile clean
- `cargo test --lib` : **392 pass, 0 fail** (0 régression)

## Fichiers modifiés

| Fichier | Changement |
|---|---|
| `src/catalog.rs` | create(), link(), update(), delete() → PendingWork seul. build_ingestion_graph() → record nodes. flush_insertions() → entities only. Suppression pending_ops. |
| `src/dataflow/record_nodes.rs` | AggregateRecordNode : toujours poser entities/relations output ports (même vides) |

## Prochaine étape

1. `./run_e2e.sh` — validation E2E
2. Phase D : supprimer les anciens batch nodes, CatalogOp, SplitOpsNode, compute_chunk_ops
