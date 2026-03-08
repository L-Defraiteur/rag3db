# Doc 09 — Plan : CRUD simple entities + drain pour pipelines génériques

Date : 8 mars 2026
Réf : Doc 08 (Phase 3 nœuds génériques), Doc 07 (E2E tests)

## Résumé

Deux problèmes identifiés + une amélioration souhaitée :

1. **CRUD cassé pour simple entities** (registered via `register_entity()`)
2. **Pas de mécanisme drain pour les pipelines génériques** (dataflow)

---

## 1. Problème : CRUD simple entities

### Contexte

Les simple entities (Product, Article, etc.) sont enregistrées via `catalog.register_entity()` et ingérées via `catalog.ingest_entities()`. Elles n'ont PAS de KB mappings (`resolve_entity_kbs()` retourne vide).

Le CRUD (`update()`, `delete()`) dans `catalog.rs` ne gère que le chemin KB (via AggregateRecords). Pour les simple entities, les chunks deviennent stales ou orphelins.

### État actuel

| Op | KB entities | Simple entities |
|----|------------|-----------------|
| Create/Ingest | `create()` + `drain()` | `ingest_entities()` ✅ |
| Search | `search()` ✅ | `search()` ✅ |
| Update | Re-aggregate + re-chunk + re-embed ✅ | Champs mis à jour, **chunks stales** ❌ |
| Delete | Delete index + chunks + entity ✅ | Entity supprimée, **chunks orphelins** ❌ |

### Naming conventions simple entities

- Table entité : `{EntityName}` (ex: `Product`)
- Table chunks : `{EntityName}_Chunk` (ex: `Product_Chunk`)
- Relation : `{EntityName}_CHUNKED_FROM` (chunk → parent)
- Champ de liaison : `_parent_uuid` sur les chunks

### Fix delete()

Dans `catalog.rs`, méthode `delete()`, après `let entity_kbs = resolve_entity_kbs(...)` et AVANT le loop KB :

```rust
if self.entity_configs.contains_key(entity_name) && entity_kbs.is_empty() {
    let chunk_table = format!("{entity_name}_Chunk");
    // DETACH DELETE les chunks par _parent_uuid
    // Flush FTS index
}
```

### Fix update()

Dans `catalog.rs`, méthode `update()`, après le loop KB, dans le bloc `if content_changed` :

1. Rendre `chunks_deleted` et `chunks_created` mutables
2. Si simple entity + content changed : appeler `rechunk_simple_entity(entity_name, uuid)`

### Helper rechunk_simple_entity()

Nouvelle méthode privée sur `Catalog`. Étapes :

1. Delete old chunks : `MATCH (c:{Entity}_Chunk {_parent_uuid: $uuid}) DETACH DELETE c`
2. Lire les données mises à jour via `self.get(entity_name, uuid)`
3. Construire mini pipeline dataflow (même pattern que `ingest_entities()` sans le InsertRecordNode initial) :
   - `ChunkRecordNode` ← données entity avec `EntityRef::pre_resolved()`
   - `InsertRecordNode` (chunks) ← output chunks
   - `LinkRecordNode` ← output chunk_links
   - `EmbedNode` ← output inserted
   - `FlushNode` ← trigger
4. Mêmes services que `ingest_entities()` (conn, embedder, entity_configs, chunker_cache, etc.)
5. Retourne `(chunks_deleted, chunks_created)`

### Tests E2E prévus (3)

Dans `tests/e2e_simple_entity.rs` :

1. **`simple_delete_removes_chunks`** — Ingest 3 produits → delete 1 → chunks supprimés, BM25 ne trouve plus, les autres restent
2. **`simple_update_refreshes_chunks`** — Ingest "Rust programming" → update vers "Python cookbook" → BM25 "programming" = 0, "cookbook" = trouvé
3. **`simple_update_unchanged_no_rechunk`** — Update seulement `price` (non-content) → `UpdateStatus::Unchanged`, pas de re-chunk

---

## 2. Amélioration souhaitée : drain pour pipelines génériques

### Contexte

Actuellement les pipelines dataflow génériques (construits avec les 22 nœuds composables) s'exécutent de manière **synchrone et immédiate** via `DataflowRuntime::execute()`. Pas de mécanisme de queue/drain comme pour le Catalog.

Pour les KB entities, le flux est :
```
catalog.create() → enqueue ops → catalog.drain() → process all
```

Pour les pipelines génériques, tout est exécuté d'un coup. Il serait mieux d'avoir un mécanisme similaire au drain : pouvoir enqueuer des exécutions de pipeline et les drainer en batch, avec les mêmes garanties de :
- **Ordering** : respecter les dépendances entre pipelines
- **Batching** : grouper les exécutions pour efficacité GPU (embedding)
- **Observabilité** : events de progression
- **Retry** : re-exécution en cas d'échec (via checkpoint)

### Idée

Un `PipelineQueue` ou extension du `DataflowRuntime` qui permettrait :

```rust
// Enqueue pipeline execution
runtime.enqueue(graph, services)?;
runtime.enqueue(graph2, services2)?;

// Drain all queued pipelines
let results = runtime.drain().await?;
```

Ou intégré au Catalog :

```rust
catalog.enqueue_pipeline(graph)?;
catalog.drain_pipelines().await?;
```

### À discuter

- Est-ce que ça devrait être au niveau du runtime ou du catalog ?
- Faut-il un scheduling intelligent (ex: grouper les EmbedNode de plusieurs pipelines) ?
- Ou juste une simple queue FIFO avec exécution séquentielle ?

---

## Fichiers clés

| Fichier | Rôle |
|---------|------|
| `src/catalog.rs` | `delete()` ligne ~1113, `update()` ligne ~965, `ingest_entities()` ligne ~505 |
| `src/schema.rs` | `resolve_entity_kbs()` ligne ~81 — retourne vide pour simple entities |
| `src/dataflow/runtime.rs` | `DataflowRuntime::execute()` — exécution synchrone |
| `src/dataflow/record_nodes.rs` | `ChunkRecordNode`, `EmbedNode`, `FlushNode` |
| `src/refs.rs` | `EntityRef::pre_resolved()` — pour re-chunk sans re-INSERT |
| `tests/e2e_simple_entity.rs` | Tests existants + 3 nouveaux CRUD tests |
| `tests/e2e_phase0b.rs` | Pattern de référence pour KB CRUD tests |

---

## Ordre d'exécution

1. Fix `delete()` (le plus simple)
2. Ajouter `rechunk_simple_entity()`
3. Fix `update()` (dépend du helper)
4. Écrire les 3 E2E tests
5. Non-régression complète
6. (Plus tard) Concevoir le mécanisme drain pour pipelines génériques

---

## Tasks

```
#192 ⬜ Fix delete() for simple entities — cascade-delete chunks
#193 ⬜ Add rechunk_simple_entity() helper + fix update()
#194 ⬜ Add 3 CRUD E2E tests for simple entities
```
