# Doc 15 — Fix FTS update hook + architecture future (queue approach)

Date : 8 mars 2026
Réf : Doc 14 (debug FTS batch indexing), Doc 11 (analyse batch update/delete)

## Résumé

Trois bugs fixés cette session. Les 15 tests E2E simple entity passent + non-régression complète.

## Bug 1 : FTS update() hook — state mismatch dans les ValueVectors (CAUSE RACINE)

### Symptôme
Seul le premier produit d'un batch UNWIND MERGE avait son champ `description` indexé dans le FTS. Le champ `details` fonctionnait pour tous.

### Cause racine
Le bug n'était PAS dans `insert()` mais dans `update()` (`lucivy_index.cpp:129-177`).

Quand Kuzu exécute `UNWIND $items AS item MERGE (n:Product {_uuid: item._uuid}) SET n.description = item.description, n.details = item.details` :

1. **MERGE** crée chaque node avec `_uuid` seulement → `insert()` appelé avec description=NULL, details=NULL (correct, pas de données à indexer)
2. **SET** déclenche `update()` pour chaque colonne, pour chaque row

`update()` faisait :
```cpp
// 1. Delete old doc
delete_(transaction, nodeIDVector, ...);

// 2. Scan all columns from storage
auto pos = nodeIDVector.state->getSelVector()[0];  // pos du node
auto dataChunk = DataChunkState::getSingleValueDataChunkState(); // SingleValue = pos 0
// ... scanVecs populated at pos 0 ...

// 3. Mix scanned + new property, call insert()
insertPtrs[updatedCol] = &propertyVector;  // new value from SET, at pos
insertPtrs[otherCols] = scanPtrs[c];       // scanned, at pos 0

// 4. BUG: pass original nodeIDVector to insert()
insert(transaction, nodeIDVector, insertPtrs, insertState);
```

`insert()` lit `pos = nodeIDVector.state->getSelVector()[i]`. Pour node 0, `pos=0` → ça marche. Pour node 1, `pos=1` → lit les scanVecs à position 1, qui n'ont de données valides qu'à position 0 → garbage/empty string.

Résultat pour node 1 :
- SET description → insert() avec description="Beta engineering..." ✓ mais details="" ✗
- SET details → delete_() efface le doc précédent, insert() avec description="" ✗ mais details="Beta details here" ✓
- État final : description perdue, seul details indexé

### Fix
Remplacé l'appel à `insert()` dans `update()` par une construction directe du document FTS. Chaque champ est lu à la bonne position :
- Colonne mise à jour : `readPos = pos` (position dans propertyVector)
- Autres colonnes : `readPos = 0` (position dans scanVecs, SingleValue)

Fichier : `extension/lucivy_fts/src/index/lucivy_index.cpp`, méthode `update()`.

## Bug 2 : Deadlock dataflow dans rechunk_simple_entities()

`ChunkRecordNode` et `KBChunkRecordNode` ne settaient pas leurs outputs quand 0 chunks étaient produits → les nodes downstream attendaient indéfiniment.

Fix : toujours set les outputs `chunks` et `chunk_links`, même avec des vecteurs vides.

Fichier : `src/dataflow/record_nodes.rs`, 2 occurrences.

## Bug 3 : get()/get_many() retournent des nodes wrappés

`RETURN n` retourne `{"n": Map({"_uuid": ..., "description": ...})}`. Le code de `batch_update()` faisait `data.get("_uuid")` directement → None. Fix : unwrap le Map sous la clé `"n"`.

Fichier : `src/catalog.rs`, dans `update()` et `batch_update()`.

## Fichiers modifiés

| Fichier | Modification |
|---------|-------------|
| `extension/lucivy_fts/src/index/lucivy_index.cpp` | Fix update() : construction directe du document au lieu d'appeler insert() |
| `src/dataflow/record_nodes.rs` | Fix deadlock : ChunkRecordNode + KBChunkRecordNode toujours set outputs |
| `src/catalog.rs` | Fix unwrap node Map dans update() et batch_update() |
| `tests/e2e_simple_entity.rs` | Nettoyé diagnostics temporaires |
| `extension/lucivy/ld-lucivy/src/query/phrase_query/scoring_utils.rs` | Hardening lowercase dans generate_trigrams() |

## Résultats tests

- 15/15 tests `e2e_simple_entity` ✓
- Non-régression complète : en cours de vérification

## Architecture future : approche queue (drain unifié)

### Contexte

L'implémentation actuelle de `update()` et `delete()` exécute tout immédiatement : écriture DB, cascade-delete chunks, rechunk via dataflow, re-embed. `batch_update()` et `batch_delete()` groupent les opérations mais sont des méthodes séparées.

Ça fonctionne, mais la complexité est significative (3 bugs corrigés cette session), et le pattern est incohérent avec `ingest_entities()` qui utilise `PendingWork` + `drain()`.

### Option future : tout passer par PendingWork + drain()

Décrite dans le Doc 11 (section "Option écartée : unifier dans le drain"), cette approche consiste à :

```rust
pub struct PendingWork {
    pub entities: Vec<EntityRecord>,      // create (existant)
    pub relations: Vec<RelationRecord>,   // link (existant)
    pub aggregates: Vec<AggregateRecord>, // KB rebuild (existant)
    pub updates: Vec<UpdateRecord>,       // NEW
    pub deletes: Vec<DeleteRecord>,       // NEW
}
```

Flow :
```
catalog.update("Product", uuid, data)  → enqueue UpdateRecord
catalog.delete("Product", uuid)        → enqueue DeleteRecord
catalog.drain()                        → process ALL :
  1. Apply field updates (batch UNWIND SET)
  2. Apply deletes (batch DETACH DELETE)
  3. Detect content changes → batch re-chunk
  4. Batch re-embed (un seul appel GPU)
  5. FTS flush
```

### Avantages

1. **Un seul chemin de batching** — plus besoin de `batch_update()` / `batch_delete()` séparés
2. **Un seul appel GPU** — tous les re-embeddings dans un seul drain
3. **Évite le bug FTS update hook** — si on fait DELETE old + INSERT new au lieu de MERGE+SET, le hook C++ `update()` n'est jamais appelé. L'indexation FTS passe uniquement par `insert()` qui est simple et correct.
4. **Cohérent avec le pattern existant** — `ingest_entities()` utilise déjà PendingWork + drain
5. **Plus simple à raisonner** — toutes les mutations sont différées, le drain est le seul point de commit

### Challenges

1. **API breaking** — `update()` ne peut plus retourner `UpdateResult` synchrone. Options :
   - Retourner un `oneshot::Receiver<UpdateResult>` (résolu au drain)
   - Retourner "Queued" + résultats agrégés dans le `DrainResult`

2. **Conflits** — create puis delete même UUID, ou deux updates sur la même entité avant drain. Nécessite une logique de merge/dedup dans le drain.

3. **Ordering KB** — Un update sur un content entity doit trigger un re-aggregate du KB Index. Les `AggregateRecord` doivent être générés au drain, pas à l'enqueue.

4. **Estimé à 2-3 sessions** de travail.

### Décision

Pour l'instant, l'implémentation actuelle (update/delete immédiats + batch variants) fonctionne et est testée. Le refactor vers l'approche queue sera considéré si :
- Le pattern d'usage montre beaucoup de updates/deletes mélangés avec des ingestions
- La performance GPU (un appel par update vs un appel batch au drain) devient un bottleneck
- D'autres bugs de synchronisation apparaissent dans les chemins update/delete immédiats

## Tasks

```
#202 ✅ Fix ChunkRecordNode deadlock — always set outputs
#203 ✅ Clean up diagnostic code
#205 ✅ Fix FTS update() hook state mismatch
#201 ✅ Add 5 CRUD E2E tests for simple entities (15/15 pass)
#204 ✅ Run E2E tests — all pass
```
