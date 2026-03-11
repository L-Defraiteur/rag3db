# Doc 02 — Plan : Queue/drain unifié pour update/delete

Date : 11 mars 2026
Réf : Doc 11 (analyse batch), Doc 15 (discussion queue approach), Doc 01 (roadmap)

## Problème

L'architecture CRUD est incohérente :

| Opération | Pattern | Exécution |
|-----------|---------|-----------|
| `create()` / `link()` | Sync → enqueue PendingWork | Différée → `drain()` |
| `update()` / `delete()` | Async → inline DB + rechunk + embed | Immédiate |
| `batch_update()` / `batch_delete()` | Async → inline batch | Immédiate |

Trois bugs ont été nécessaires pour stabiliser l'approche immédiate (Doc 15) :
1. FTS update() hook state mismatch (C++)
2. Deadlock dataflow dans rechunk_simple_entities()
3. Node Map unwrap dans get()/get_many()

L'objectif est d'unifier toutes les mutations dans PendingWork + `drain()`.

## État actuel du code

### PendingWork (`records.rs:374-399`)

```rust
pub struct PendingWork {
    pub entities: Vec<EntityRecord>,      // create()
    pub relations: Vec<RelationRecord>,   // link()
    pub aggregates: Vec<AggregateRecord>, // KB rebuild
}
```

### drain() (`catalog.rs:2146-2187`)

Appelle `build_ingestion_graph()` → DAG dataflow :
```
InsertRecordNode → LinkRecordNode → KBGatherNode → KBUpdateNode
→ KBChunkNode → InsertRecordNode(chunks) → LinkRecordNode(links)
→ KBEmbedNode → FlushNode
```

Services enregistrés : conn, node_id_cache, embedder, embedding_dim, config, kb_metadata, has_sparse, has_dual, chunker_cache. Retourne `FlushResult { processed, failed }`. Support checkpoint.

### update() (`catalog.rs:965-1135`)

1. Compute new hash via `build_content_text()`
2. Query séparée pour lire l'ancien hash (Kuzu SET+RETURN retourne post-SET)
3. MATCH SET tous les champs + `_content_hash`
4. Si content_changed :
   - KB : enqueue AggregateRecords dans `pending.aggregates` (NE drain PAS — le caller doit drain)
   - Simple : appel immédiat `rechunk_simple_entities()` (mini dataflow graph à la volée)
5. Emit `CatalogEvent::EntityUpdated`
6. Return `UpdateResult { uuid, entity, status, reembedded, chunks_created, chunks_deleted }`

### delete() (`catalog.rs:1137-1303`)

1. KB titleFor : cascade-delete chunks + index entries
2. KB contentFor : delete SOURCED chunks, enqueue AggregateRecords
3. Simple : cascade-delete `{Entity}_Chunk`
4. DETACH DELETE entity
5. Remove from node_id_cache
6. Flush FTS immédiat (simple entities seulement)
7. Emit `CatalogEvent::EntityDeleted`
8. Return `DeleteResult { uuid, entity, chunks_deleted, relations_deleted }`

### batch_update() / batch_delete() (`catalog.rs:1309-1831`)

Même logique que update()/delete() mais avec UNWIND pour grouper les requêtes et un seul appel GPU pour les re-embeddings. `rechunk_simple_entities()` appelé une fois pour tout le batch.

### Points critiques

- `merge_port_values()` (`port.rs:240`) rejette Batch+Batch → pas de fan-in natif pour les aggregates
- `get()`/`get_many()` wrappent le résultat dans `{"n": Map({...})}` → les nœuds doivent unwrap
- WASM n'expose pas update/delete → pas de souci backward compat WASM
- 25 variants `CatalogEvent`, dont `EntityUpdated`/`EntityDeleted` émis inline

---

## Plan d'implémentation

### Phase 1 : Foundation — records + ports (~0.5 session)

**Ajouts purs, zéro changement de comportement.**

Nouveaux types dans `records.rs` :
```rust
pub struct UpdateRecord {
    pub entity_name: String,
    pub uuid: String,
    pub data: BTreeMap<String, CypherValue>,
    pub new_content_hash: String,  // pré-calculé à l'enqueue
}

pub struct DeleteRecord {
    pub entity_name: String,
    pub uuid: String,
}
```

Extension de PendingWork :
```rust
pub struct PendingWork {
    pub entities: Vec<EntityRecord>,
    pub relations: Vec<RelationRecord>,
    pub aggregates: Vec<AggregateRecord>,
    pub updates: Vec<UpdateRecord>,     // NEW
    pub deletes: Vec<DeleteRecord>,     // NEW
}
```

Nouveaux port types dans `port.rs` :
```rust
pub enum PortType {
    // ... existants ...
    Updates,
    Deletes,
}
```

**Fichiers** : `records.rs`, `port.rs`
**Tests** : unitaires sur PendingWork (is_empty, total_count)
**Risque** : nul

---

### Phase 2 : Nouveaux nœuds dataflow (~1 session)

**3 nœuds dont la logique est extraite de catalog.rs (pas réécrite).**

#### DeleteRecordNode

| | |
|-|-|
| Input | `deletes` — `BatchPayload<DeleteRecord>` |
| Output | `done` — Empty signal |
| Services | conn, config, kb_metadata, node_id_cache, entity_configs |

Logique extraite de `batch_delete()` :
1. Group par `entity_name`
2. KB titleFor : batch UNWIND DETACH DELETE chunks + index entries
3. KB contentFor : batch UNWIND DETACH DELETE SOURCED chunks, trouver title entities liées
4. Simple : batch cascade-delete `{Entity}_Chunk` par `_parent_uuid`
5. Batch DETACH DELETE entities
6. Remove from node_id_cache
7. Pousse AggregateRecords dans shared service `Arc<Mutex<Vec<AggregateRecord>>>`
8. Pousse DeleteResults dans shared service `Arc<Mutex<Vec<DeleteResult>>>`

#### UpdateRecordNode

| | |
|-|-|
| Input | `updates` — `BatchPayload<UpdateRecord>` |
| Output | `done` — Empty signal, `rechunk_entities` — `BatchPayload<EntityRecord>` |
| Services | conn, config, kb_metadata, entity_configs |

Logique extraite de `batch_update()` :
1. Group par `entity_name`
2. Batch-read old hashes via UNWIND query
3. Detect content changes (`old_hash != new_content_hash`)
4. Batch SET tous les champs + `_content_hash` via UNWIND MATCH SET
5. KB changed : compute AggregateRecords → shared service
6. Simple changed : lire full data (avec unwrap node Map), construire EntityRecords avec `EntityRef::pre_resolved()` → output `rechunk_entities`
7. Pousse UpdateResults dans shared service `Arc<Mutex<Vec<UpdateResult>>>`

#### RechunkDeleteNode (helper)

| | |
|-|-|
| Input | `entities` — `BatchPayload<EntityRecord>` |
| Output | `entities` — pass-through (mêmes records) |
| Services | conn |

Logique : batch-delete old chunks par `_parent_uuid` AVANT que ChunkRecordNode ne crée les nouveaux. C'est la première étape de `rechunk_simple_entities()` extraite.

**Fichiers** : `record_nodes.rs`, `node_factories.rs` (register_builtins)
**Tests** : unitaires avec mock DbConnection
**Risque** : moyen — extraction de logique existante, pas de réécriture

---

### Phase 3 : Intégration dans build_ingestion_graph() (~1 session)

**Le cœur du refactor.** Nouveau DAG :

```
DeleteRecordNode("deletes")
  ├─ done ─────→ UpdateRecordNode("updates")

UpdateRecordNode("updates")
  ├─ done ─────→ InsertRecordNode("inserts")     [existant]
  ├─ rechunk_entities ─→ RechunkDeleteNode("rechunk_delete")
                           └─ entities → ChunkRecordNode("rechunk_chunk")
                                ├─ chunks → InsertRecordNode("rechunk_insert")
                                │             └─ inserted → EmbedNode("rechunk_embed")
                                └─ chunk_links → LinkRecordNode("rechunk_link")
                                                   └─ done → trigger EmbedNode

InsertRecordNode("inserts")     [existant, inchangé]
  └─ done → LinkRecordNode("links")     [existant]

KBGatherNode     [existant, lit aggregates depuis shared service]
  └─ kb_content → KBUpdateNode → KBChunkNode → ...     [existant]

FlushNode     [reçoit triggers de tous les terminaux]
```

**Ordering** : deletes → updates → inserts → links → KB aggregation

**Agrégats** : les AggregateRecords viennent de 3 sources :
1. `pending.aggregates` (create/link existants)
2. DeleteRecordNode (KB contentFor)
3. UpdateRecordNode (KB titleFor/contentFor)

Pas de fan-in port (rejeté par merge_port_values). Solution : shared service `Arc<Mutex<Vec<AggregateRecord>>>`, alimenté par les 3 sources. KBGatherNode modifié pour lire depuis ce service.

**Résultats** : shared services `Arc<Mutex<Vec<UpdateResult>>>` et `Arc<Mutex<Vec<DeleteResult>>>` pour extraction post-drain.

**Fichiers** : `catalog.rs` (build_ingestion_graph), `record_nodes.rs` (KBGatherNode modifié)
**Tests** : E2E drain mixte (create + update + delete en un seul drain)
**Risque** : élevé — point d'intégration central. Mitigé par les 120+ tests de régression existants.

---

### Phase 4 : Nouvelle API + wrappers backward-compat (~0.5 session)

**Nouveaux endpoints sync (enqueue seulement) :**
```rust
pub fn enqueue_update(&mut self, entity_name: &str, uuid: &str,
    data: BTreeMap<String, CypherValue>) -> Result<(), CatalogError>

pub fn enqueue_delete(&mut self, entity_name: &str,
    uuid: &str) -> Result<(), CatalogError>
```

**Wrappers backward-compat (enqueue + drain + extract) :**
```rust
pub async fn update(&mut self, ...) -> Result<UpdateResult, CatalogError> {
    self.enqueue_update(entity_name, uuid, data)?;
    let flush = self.drain().await;
    // extract result depuis flush.update_results
}

pub async fn delete(&mut self, ...) -> Result<DeleteResult, CatalogError> {
    self.enqueue_delete(entity_name, uuid)?;
    let flush = self.drain().await;
    // extract result depuis flush.delete_results
}

// batch_update / batch_delete : N enqueues → 1 drain → extract
```

**FlushResult étendu :**
```rust
pub struct FlushResult {
    pub processed: usize,
    pub failed: usize,
    pub update_results: Vec<UpdateResult>,   // NEW
    pub delete_results: Vec<DeleteResult>,   // NEW
}
```

**Fichiers** : `catalog.rs`, `records.rs` (FlushResult)
**Tests** : tous les 120+ E2E passent sans modification (wrappers = même sémantique)
**Risque** : moyen — extraction résultats depuis shared services post-drain

---

### Phase 5 : Conflict resolution + events + cleanup (~0.5 session)

**Résolution de conflits** dans `build_ingestion_graph()`, avant construction du DAG :
- Delete + Update même UUID → delete gagne, update retiré
- Deux updates même UUID → last-enqueued gagne (replace)
- Delete + Create même UUID → les deux s'exécutent (delete ancien puis create nouveau)

**Events** : enregistrer `Sender<CatalogEvent>` comme service. Les nœuds émettent EntityUpdated/EntityDeleted au lieu de catalog.rs inline.

**Cleanup** :
- Supprimer le code inline de update()/delete()/batch_update()/batch_delete() (dead code)
- Supprimer `rechunk_simple_entities()` (logique dans RechunkDeleteNode + ChunkRecordNode)
- Déprécier ou supprimer batch_update()/batch_delete() (remplacés par N enqueues + 1 drain)

**Fichiers** : `catalog.rs`, `record_nodes.rs`, `events.rs`
**Tests** : tests spécifiques conflits (create+delete, double update, update+delete)
**Risque** : faible — additif et polish

---

## Estimation

| Phase | Effort | Risque | Prérequis |
|-------|--------|--------|-----------|
| 1. Foundation | ~0.5 session | Nul | — |
| 2. Nouveaux nœuds | ~1 session | Moyen | Phase 1 |
| 3. Intégration graph | ~1 session | Élevé | Phase 2 |
| 4. API + wrappers | ~0.5 session | Moyen | Phase 3 |
| 5. Conflicts + cleanup | ~0.5 session | Faible | Phase 4 |
| **Total** | **~3.5 sessions** | | |

Chaque phase est indépendamment testable. Les phases 1-2 n'ont aucun risque de casser les tests existants. La phase 3 est le point critique.

## Avantages du résultat final

1. **Un seul chemin pour toutes les mutations** — create, update, delete passent tous par PendingWork + drain
2. **Un seul appel GPU par drain** — même si on mélange creates + updates + deletes
3. **Évite le bug FTS update hook** — les updates passent par DELETE old chunks + INSERT new chunks (le hook C++ `update()` n'est jamais appelé)
4. **batch_update/batch_delete deviennent obsolètes** — N enqueues + 1 drain = même résultat
5. **Plus simple à raisonner** — toutes les mutations sont différées, drain est le seul point de commit

## Fichiers clés

| Fichier | Phases | Modification |
|---------|--------|-------------|
| `src/records.rs` | 1, 4 | UpdateRecord, DeleteRecord, PendingWork, FlushResult |
| `src/dataflow/port.rs` | 1 | PortType::Updates, PortType::Deletes |
| `src/dataflow/record_nodes.rs` | 2, 3, 5 | DeleteRecordNode, UpdateRecordNode, RechunkDeleteNode, KBGatherNode modifié |
| `src/catalog.rs` | 3, 4, 5 | build_ingestion_graph(), enqueue API, wrappers, cleanup |
| `src/dataflow/node_factories.rs` | 2 | register_builtins() |
| `src/events.rs` | 5 | Event emission via service |
| `tests/e2e_simple_entity.rs` | 3, 4 | Tests drain mixte |

## Vérification

```bash
cargo check --lib --features "rag3db-native,candle-embedder"
cargo test --lib --features "rag3db-native,candle-embedder"
./run_e2e.sh --test e2e_simple_entity    # 15 tests CRUD
./run_e2e.sh --test e2e_native           # update/delete KB
./run_e2e.sh --test e2e_phase0b          # cross-entity KB CRUD
./run_e2e.sh                             # non-régression complète 120+
```
