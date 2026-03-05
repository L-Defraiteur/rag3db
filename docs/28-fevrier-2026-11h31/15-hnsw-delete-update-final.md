# 15 — HNSW DELETE/UPDATE : implémentation finalisée

## Résumé

Suite aux docs 13 et 14, on a finalisé l'implémentation complète du DELETE et UPDATE batché pour l'index HNSW de l'extension vector. **64/64 tests passent.** Commit `98e35566a` poussé sur master.

## Ce qui a été fait dans cette session

### 1. Finalisation du code du doc 14

Trois changements restants du doc 14 appliqués :

- **`deleteFromGraph`** dans hnsw_index.cpp : signature changée de `void` à `DeletedNeighbors` (struct avec vectors lower/upper), ajout du `return {std::move(lowerNeighbors), std::move(upperNeighbors)};`
- **`update()`** : changé `deleteFromGraph(...)` en `(void)deleteFromGraph(...)` pour ignorer le retour (update fait immédiatement un re-insert, pas besoin de collecter les voisins)

### 2. Segfault DeleteBulkRecovery — diagnostic et résolution

Le test `DeleteBulkRecovery` (MATCH (e:embeddings) WHERE e.id <=200 DELETE e) segfaultait. Trois bugs trouvés et corrigés :

#### Bug 1 : `shrinkForNode` crashe sur embeddings null (doc 14)

**Cause** : `shrinkForNode(V)` lit les embeddings des voisins de V. Certains voisins de V ont été supprimés de la base table → embedding null → crash.

**Solution initiale** : remplacer `shrinkForNode` par `cleanEdgesForNode` — une nouvelle méthode beaucoup plus simple qui ne lit AUCUN embedding. Elle vérifie juste `deletedNodes.contains(nbr)` pour filtrer les edges mortes. Algorithme :
1. Scanner les forward edges de V → [A, B, M, D]
2. Vérifier si un voisin est dans deletedNodes → oui (M)
3. detachDelete(FWD) sur V → supprime toutes les edges
4. Ré-insérer seulement [A, B, D] (sans M)

#### Bug 2 : cleanEdgesForNode appelé sur des nœuds eux-mêmes supprimés

**Cause** : dans `finalizeDelete`, on itère `lowerNeighborsToShrink` mais certains voisins sont AUSSI des nœuds supprimés (dans le range 0-200). Appeler `cleanEdgesForNode` sur un nœud supprimé dont les edges ont déjà été detach pouvait crasher.

**Solution** : ajout du guard `if (!state.deletedNodes.contains(nbrOffset))` avant chaque appel à `cleanEdgesForNode` dans `finalizeDelete`.

#### Bug 3 : `deleteState_` null dans executor copié (LE VRAI BUG)

**Cause racine du segfault** : le pipeline system copie les executors via `copy()`. `SingleLabelNodeDeleteExecutor` a un `std::unique_ptr<NodeTableDeleteState> deleteState_` qui n'est PAS copiable. Le copy constructor ne le copie pas → la copie a `deleteState_ = nullptr`. Quand `finalizeInternal` est appelé sur la copie (depuis un worker thread), `*deleteState_` = null dereference → SIGSEGV.

GDB montrait `NodeTable::finalizeDelete` comme crash point, mais c'était trompeur (Release mode + inlining). Les fprintf dans NodeTable ne s'affichaient jamais car le crash était AVANT, dans l'executor `finalize()`.

**Diagnostic clé** : ajout de fprintf dans `SingleLabelNodeDeleteExecutor::finalize()` → `deleteState_=(nil)` confirmé.

**Solution** : null guard dans `finalize()` des deux executors :
```cpp
void SingleLabelNodeDeleteExecutor::finalize(ExecutionContext* context) {
    if (!deleteState_) return;  // Copie sans state, rien à finaliser
    auto transaction = Transaction::Get(*context->clientContext);
    tableInfo.table->finalizeDelete(transaction, *deleteState_);
}
```
Même pattern pour `MultiLabelNodeDeleteExecutor::finalize()`.

### 3. Fix KUZU_ROOT_DIRECTORY → RAG3DB_ROOT_DIRECTORY

Les fichiers test vector (delete.test, insert.test, filter.test, error.test) utilisaient encore `${KUZU_ROOT_DIRECTORY}` (ancien nom Kuzu) au lieu de `${RAG3DB_ROOT_DIRECTORY}`. Le test parser ne reconnaissait que la nouvelle variable → tous les COPY échouaient → cascade d'échecs.

**Fix** : `sed -i 's/KUZU_ROOT_DIRECTORY/RAG3DB_ROOT_DIRECTORY/g'` sur les 4 fichiers.

### 4. Update insert.test

Ligne 41-43 : le test SET sur colonne indexée HNSW attendait `---- error` avec le message "Cannot set property vec...". Maintenant que `update()` fonctionne, changé en `---- ok`.

### 5. Push ld-lucivy

5 fichiers modifiés dans ld-lucivy (sessions précédentes, non commités) : filter fields support, query builder amélioré, schema fixes. Commit `8f1a5bd` poussé sur main.

## Architecture finale

```
DELETE e;  (200 nœuds)
    │
    ▼  (par nœud, dans getNextTuplesInternal)
NodeTable::delete_()
    ├── initDeleteStates()  ← lazy init, crée HNSWDeleteState UNE SEULE fois
    ├── OnDiskHNSWIndex::delete_()
    │       └── deleteFromGraph()
    │               ├── scanNeighbors(lower) → collecte voisins
    │               ├── scanNeighbors(upper) → collecte voisins
    │               ├── entry point replacement si nécessaire
    │               ├── detachDelete(FWD) lower + upper
    │               └── return {lowerNeighbors, upperNeighbors}
    │       └── collecte dans unordered_set (dédupliqué)
    │       └── deletedNodes.insert(offset)
    └── suppression base table
    │
    ▼  (après TOUS les nœuds, dans finalizeInternal)
DeleteNode::finalizeInternal()
    └── executor->finalize()  ← null guard si copie
        └── NodeTable::finalizeDelete()
            └── OnDiskHNSWIndex::finalizeDelete()
                    └── pour chaque voisin (dédupliqué, non-supprimé) :
                        cleanEdgesForNode()
                            ├── scanNeighbors(V)
                            ├── check deletedNodes.contains(nbr)
                            ├── detachDelete(FWD) sur V
                            └── re-insert edges non-supprimées
```

## Fichiers modifiés (commit 98e35566a)

| Fichier | Modifications |
|---------|--------------|
| `src/include/transaction/transaction.h` | `getClientContext()` public getter |
| `src/include/storage/index/index.h` | `finalizeDelete()` virtual |
| `src/include/storage/table/node_table.h` | `indexDeleteStates` + `initDeleteStates/finalizeDelete` |
| `src/storage/table/node_table.cpp` | Implémentation lazy init + finalize |
| `src/include/processor/operator/persistent/delete.h` | `finalizeInternal()` override |
| `src/processor/operator/persistent/delete.cpp` | Implémentation finalizeInternal |
| `src/include/processor/operator/persistent/delete_executor.h` | `finalize()` + `deleteState_` membres |
| `src/processor/operator/persistent/delete_executor.cpp` | State réutilisé + finalize + null guards |
| `extension/vector/src/include/index/hnsw_index.h` | HNSWDeleteState, HNSWUpdateState, DeletedNeighbors, cleanEdgesForNode |
| `extension/vector/src/index/hnsw_index.cpp` | delete_, deleteFromGraph, finalizeDelete, cleanEdgesForNode, update, initDeleteState, initUpdateState |
| `extension/vector/test/test_files/*.test` | KUZU→RAG3DB, SET ok au lieu d'error |

## Tests

- **64/64 tests vector passent** (transaction, delete, insert, filter, error, small, hnsw, query)
- Le test `DeleteBulkRecovery` (200 nœuds supprimés d'un coup) passe sans segfault
- Le test `InsertToNonEmpty` passe avec SET réussi au lieu d'erreur

## Leçons retenues

1. **En Release mode, les backtraces GDB sont trompeuses** — l'inlining fait pointer vers des fonctions jamais atteintes. Toujours ajouter des fprintf pour confirmer le chemin d'exécution.
2. **Les executors sont copiés par le pipeline** — tout `unique_ptr` membre ne sera pas copié. Il faut des null guards dans `finalize()`.
3. **`cmake --build . --target clean` supprime les fichiers ANTLR générés** — il faut les restaurer via `git checkout`.
4. **Après modification de la lib statique, forcer le relink** : `rm -f test/runner/e2e_test` puis rebuild.
5. **cleanEdgesForNode >> shrinkForNode pour le cleanup** — pas besoin de lire des embeddings ni de calculer des distances pour juste retirer des edges mortes.
