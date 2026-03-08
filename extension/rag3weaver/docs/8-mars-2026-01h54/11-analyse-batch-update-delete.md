# Doc 11 — Analyse : batch update/delete pour le Catalog

Date : 8 mars 2026
Réf : Doc 09 (drain pipelines génériques), Doc 10 (CRUD simple entities)

## État actuel du drain

| Opération | Mécanisme | Batching embedding |
|-----------|-----------|-------------------|
| `create()` + `link()` | Queue → `drain()` | Oui, tout en un seul appel GPU |
| `update()` | **Immédiat** | Non, un par un |
| `delete()` | **Immédiat** | N/A |

Le drain actuel fonctionne déjà bien pour les creates : plusieurs `create()` enfilent dans `PendingWork` (3 vecs typés : entities, relations, aggregates), puis `drain()` construit un graphe dataflow qui batch tout — y compris les embeddings en un seul appel GPU.

**Le problème** : `update()` et `delete()` contournent complètement la queue. Chaque appel fait son propre re-chunk + re-embed inline.

---

## Option écartée : unifier dans le drain (queue approach)

On pourrait étendre `PendingWork` avec deux nouveaux vecs :

```rust
pub struct PendingWork {
    pub entities: Vec<EntityRecord>,      // create
    pub relations: Vec<RelationRecord>,   // link
    pub aggregates: Vec<AggregateRecord>, // KB rebuild
    pub updates: Vec<UpdateRecord>,       // NEW
    pub deletes: Vec<DeleteRecord>,       // NEW
}
```

Où :

```rust
pub struct UpdateRecord {
    pub entity_name: String,
    pub uuid: String,
    pub new_data: BTreeMap<String, CypherValue>,
    pub status_sender: oneshot::Sender<UpdateStatus>, // retour async
}

pub struct DeleteRecord {
    pub entity_name: String,
    pub uuid: String,
    pub status_sender: oneshot::Sender<DeleteStatus>,
}
```

Le flow deviendrait :

```
catalog.update("Product", uuid, data)  → enqueue UpdateRecord, receive oneshot receiver
catalog.update("Product", uuid2, data) → enqueue another
catalog.delete("Product", uuid3)       → enqueue DeleteRecord
catalog.drain()                        → process ALL in order:
  1. Apply field updates (batch UNWIND)
  2. Apply deletes (batch DETACH DELETE)
  3. Detect content changes → batch re-chunk
  4. Batch re-embed (un seul appel GPU pour TOUS les updates)
  5. Resolve oneshot channels with status
```

### Challenges de cette approche

1. **Retour de statut** — `update()` retourne `UpdateStatus` aujourd'hui de manière synchrone. Si on queue, il faudrait soit :
   - `update()` retourne un `Future`/`oneshot::Receiver` (API breaking)
   - `update()` reste sync mais retourne juste "Queued", et le vrai statut vient du `drain()`

2. **Conflits** — Que faire si on `create()` puis `delete()` le même UUID avant drain ? Ou deux `update()` sur le même entity ? Il faudrait une dédup/merge dans le drain.

3. **Ordering KB** — Un update sur un content entity (Chapter) doit trigger un re-aggregate du KB Index. Ça se fait déjà avec `AggregateRecord`, mais il faudrait les générer au moment du drain, pas à l'enqueue.

---

## Option retenue : batch_update / batch_delete

Le vrai bottleneck c'est les embeddings GPU. Un `batch_update()` qui regroupe tous les re-embed en un seul appel résout 90% du problème de perf, sans toucher à l'architecture.

### Avantages

- Pas de breaking change — `update()` et `delete()` individuels restent tels quels
- Pas de complexité de conflict resolution (create+delete même UUID, etc.)
- Pas de question "quel statut retourner" — le batch retourne un `Vec<UpdateStatus>`
- Implémentation simple : une boucle field-update + un seul pass re-chunk + un seul appel embed

### API proposée

```rust
// Batch update — one GPU call for all re-embeddings
let statuses = catalog.batch_update("Product", vec![
    (uuid1, new_data1),
    (uuid2, new_data2),
]).await?;
// statuses: Vec<UpdateStatus>

// Batch delete — one pass for all chunk deletions
let count = catalog.batch_delete("Product", vec![uuid1, uuid2, uuid3]).await?;
```

### Évolution future

Plus tard, si on veut la vraie unification create+update+delete dans un seul drain (queue approach), on pourra le faire par-dessus — mais c'est un refactor plus gros pour un gain marginal par rapport au batching.

---

## Tasks

```
#195 ⬜ Implement batch_update() for KB + simple entities
#196 ⬜ Implement batch_delete() for KB + simple entities
#197 ⬜ E2E tests for batch operations
```
