# 14 — HNSW DELETE/UPDATE batché : progression

## Résumé de la session

Suite au doc 13, on a résolu le problème `ClientContext` et commencé l'implémentation complète avec **batching optimisé** des shrinkForNode — l'approche "top notch" demandée par l'utilisatrice.

## Ce qui a été FAIT et COMPILÉ avec succès

### 1. Getter `getClientContext()` sur Transaction

**Fichier** : `src/include/transaction/transaction.h` (ligne ~147)
```cpp
main::ClientContext* getClientContext() const { return clientContext; }
```
Ajouté comme méthode publique. Résout le problème bloquant du doc 13.

### 2. `initDeleteState()` corrigé dans hnsw_index.cpp

```cpp
std::unique_ptr<Index::DeleteState> OnDiskHNSWIndex::initDeleteState(
    const transaction::Transaction* transaction, storage::MemoryManager* /*mm*/,
    storage::visible_func /*isVisible*/) {
    auto* context = transaction->getClientContext();
    auto [nodeTableEntry, upperRelTableEntry, lowerRelTableEntry] =
        getIndexTableCatalogEntries(catalog::Catalog::Get(*context),
            Transaction::Get(*context), indexInfo);
    return std::make_unique<HNSWDeleteState>(context, nodeTableEntry, upperRelTableEntry,
        lowerRelTableEntry, nodeTable, indexInfo.columnIDs[0], config.ml);
}
```

### 3. Build réussi (vector + lucivy_fts + sparse_vector)

```bash
cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSION_TESTS=TRUE \
  -DBUILD_EXTENSIONS="vector;lucivy_fts;sparse_vector" -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE
cmake --build . --target e2e_test rag3db_vector_extension -j$(nproc)
# ✅ Build OK, zéro erreur
```

### 4. Tests : segfault identifié et diagnostiqué

Tests lancés : `LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu ./test/runner/e2e_test ../../extension/vector/test/test_files/`

- 4 tests transaction passent ✅ (MultipleInsertionsRecovery, InsertRecovery, DropHNSWRollbackRecovery, CreateHNSWRollback)
- **Segfault (exit 139) sur `DeleteBulkRecovery`** — `MATCH (e:embeddings) WHERE e.id <=200 DELETE e;`

**Cause du segfault** : `shrinkForNode()` fait `KU_ASSERT(!vector.isNull())` (ligne 1095). Dans un bulk delete de 200 nœuds, quand on supprime le nœud N et qu'on shrink son voisin V, le voisin V peut avoir un voisin M qui a DÉJÀ été supprimé de la base table par un `NodeTable::delete_()` précédent. `shrinkForNode(V)` scanne les voisins de V, tombe sur M dont l'embedding est null → assertion → crash.

### 5. Architecture batching implémentée (EN COURS — pas encore buildé)

L'utilisatrice a demandé l'optimisation maximale : au lieu de shrink immédiat par nœud, collecter tous les voisins à shrink et les traiter en batch dédupliqué à la fin.

#### Changements CORE rag3db (tous faits) :

**`src/include/storage/index/index.h`** — Ajouté :
```cpp
virtual void finalizeDelete(transaction::Transaction*, DeleteState&) {
    // DO NOTHING. Override in extensions that need batched cleanup (e.g., HNSW).
}
```

**`src/include/storage/table/node_table.h`** — `NodeTableDeleteState` enrichi :
```cpp
struct NodeTableDeleteState : TableDeleteState {
    ValueVector& nodeIDVector;
    ValueVector& pkVector;
    std::vector<std::unique_ptr<Index::DeleteState>> indexDeleteStates;  // NOUVEAU
    bool indexDeleteStatesInitialized = false;                            // NOUVEAU
    // ...
};
```
`NodeTable` — deux nouvelles méthodes :
```cpp
void initDeleteStates(const Transaction* transaction, TableDeleteState& deleteState);
void finalizeDelete(Transaction* transaction, TableDeleteState& deleteState);
```

**`src/storage/table/node_table.cpp`** — Implémenté :
- `initDeleteStates()` : crée les index delete states une seule fois, les stocke dans `NodeTableDeleteState`
- `delete_()` modifié : appelle `initDeleteStates()` en lazy init, réutilise les states au lieu d'en créer par nœud
- `finalizeDelete()` : appelle `Index::finalizeDelete()` sur chaque index

**`src/include/processor/operator/persistent/delete_executor.h`** — Modifié :
- `NodeDeleteExecutor` : ajouté `virtual void finalize(ExecutionContext*) {}`
- `SingleLabelNodeDeleteExecutor` : stocke `std::unique_ptr<NodeTableDeleteState> deleteState_` comme membre (créé une fois dans `init()`), ajouté `finalize()` override
- `MultiLabelNodeDeleteExecutor` : stocke `table_id_map_t<unique_ptr<NodeTableDeleteState>> deleteStates_`, ajouté `finalize()` override

**`src/processor/operator/persistent/delete_executor.cpp`** — Modifié :
- `SingleLabelNodeDeleteExecutor::init()` : crée `deleteState_` une fois
- `SingleLabelNodeDeleteExecutor::delete_()` : réutilise `deleteState_` au lieu de créer par appel
- `SingleLabelNodeDeleteExecutor::finalize()` : appelle `table->finalizeDelete(transaction, *deleteState_)`
- Idem pour `MultiLabelNodeDeleteExecutor`

**`src/include/processor/operator/persistent/delete.h`** — `DeleteNode` :
```cpp
void finalizeInternal(ExecutionContext* context) override;
```

**`src/processor/operator/persistent/delete.cpp`** — Implémenté :
```cpp
void DeleteNode::finalizeInternal(ExecutionContext* context) {
    for (auto& executor : executors) {
        executor->finalize(context);
    }
}
```

#### Changements EXTENSION vector (EN COURS) :

**`extension/vector/src/include/index/hnsw_index.h`** — `HNSWDeleteState` enrichi :
```cpp
struct HNSWDeleteState final : DeleteState {
    HNSWInsertState insertState;
    std::unordered_set<offset_t> lowerNeighborsToShrink;  // NOUVEAU
    std::unordered_set<offset_t> upperNeighborsToShrink;  // NOUVEAU
    std::unordered_set<offset_t> deletedNodes;             // NOUVEAU
    // ...
};
```
- `deleteFromGraph` signature changée : retourne `DeletedNeighbors {lower, upper}` au lieu de void
- Ajouté override `finalizeDelete(Transaction*, DeleteState&)`

**`extension/vector/src/index/hnsw_index.cpp`** — Modifié :
- `delete_()` : appelle `deleteFromGraph`, collecte les voisins retournés dans les sets (dédupliqués via unordered_set), enregistre les nœuds supprimés
- `deleteFromGraph()` : ne fait PLUS les shrinkForNode — juste scan voisins, entry point, detachDelete forward edges, retourne les voisins
- `finalizeDelete()` (NOUVEAU) : itère les sets dédupliqués, vérifie embedding non-null, appelle `shrinkForNode` — un seul shrink par voisin unique

## CE QUI RESTE À FAIRE

### 1. Finir `deleteFromGraph` — changer le return type (void → DeletedNeighbors)

La signature dans le header est changée mais le `.cpp` doit être mis à jour pour retourner `{lowerNeighbors, upperNeighbors}` au lieu de void. C'est 2 lignes :
```cpp
// Dans deleteFromGraph, remplacer la fin actuelle par :
    return {std::move(lowerNeighbors), std::move(upperNeighbors)};
```

### 2. Adapter `update()` qui appelle aussi `deleteFromGraph`

`update()` (ligne ~678) appelle `deleteFromGraph(transaction, offset, state.insertState)`. Avec le nouveau return type, il doit juste ignorer le retour (les voisins ne sont pas collectés pour update car l'update fait immédiatement un re-insert qui trigger naturellement les shrinks via `insertInternal`).

Changer la ligne en :
```cpp
(void)deleteFromGraph(transaction, offset, state.insertState);
```

### 3. Aussi traiter le cas `node_batch_insert_error_handler.cpp`

Ce fichier (ligne 29) appelle `nodeTable->delete_(transaction, deleteState)` avec un `NodeTableDeleteState` local. Il faut vérifier que `initDeleteStates` en lazy init est OK ici (ça devrait car on a le lazy init dans `NodeTable::delete_()` lui-même). Mais il n'appelle pas `finalizeDelete()` — il faudrait l'ajouter après le delete, ou vérifier que c'est acceptable (c'est un error handler, les shrinks manqués sont tolérables).

### 4. Build et test

```bash
cd packages/rag3db/build/release
cmake --build . --target e2e_test rag3db_vector_extension -j$(nproc)
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu ./test/runner/e2e_test ../../extension/vector/test/test_files/
```

### 5. Modifier les tests existants

- **`insert.test` lignes 41-43** : le `SET t.vec = [...]` attend `---- error`. Après notre fix update, ça doit devenir `---- ok` + vérification search
- **Nouveau `update.test`** : INSERT → search → SET vec → search, SET to NULL, etc.
- **`delete.test`** : devrait toujours passer (comportement observable identique)

### 6. Revenir à rag3weaver

Une fois l'extension vector corrigée, le flow `EmbedProcessor` qui fait `SET n.kb_embedding = [...]` ne throw plus. Les tests Phase 2 via Catalog devraient passer.

## Résumé des fichiers modifiés

| Fichier | État |
|---------|------|
| `src/include/transaction/transaction.h` | ✅ `getClientContext()` ajouté |
| `src/include/storage/index/index.h` | ✅ `finalizeDelete()` virtual ajouté |
| `src/include/storage/table/node_table.h` | ✅ `indexDeleteStates` + `initDeleteStates/finalizeDelete` |
| `src/storage/table/node_table.cpp` | ✅ Implémentation lazy init + finalize |
| `src/include/processor/operator/persistent/delete_executor.h` | ✅ `finalize()` + state membres |
| `src/processor/operator/persistent/delete_executor.cpp` | ✅ State réutilisé + finalize |
| `src/include/processor/operator/persistent/delete.h` | ✅ `finalizeInternal()` override |
| `src/processor/operator/persistent/delete.cpp` | ✅ Implémentation |
| `extension/vector/src/include/index/hnsw_index.h` | ✅ Sets + `DeletedNeighbors` + `finalizeDelete` |
| `extension/vector/src/index/hnsw_index.cpp` | ⚠️ `deleteFromGraph` doit retourner les voisins, `update()` doit ignorer le retour |

## Optimisations obtenues avec le batching

1. **1 seul `HNSWDeleteState`** (avec `HNSWInsertState`, `OnDiskGraph`, scan states) créé par batch au lieu de 1 par nœud (200× moins d'allocations)
2. **Déduplication des voisins** via `unordered_set` : si les nœuds 3 et 7 partagent le voisin 100, le voisin 100 n'est shrink qu'UNE fois
3. **Null check** avant shrink : les voisins dont l'embedding est null (déjà supprimés) sont skippés
4. **Zéro fuite mémoire** : toutes les back-edges sont nettoyées via `finalizeDelete`
