# 13 — Implémentation DELETE/UPDATE réel dans l'extension vector HNSW : progression

## Contexte

Suite au doc 12, on a décidé de **ne pas** faire le workaround InsertWithEmbed dans rag3weaver, mais de corriger l'extension vector elle-même. L'utilisatrice a insisté : "on veut la perfection, pas du scotch".

Deux problèmes critiques dans l'extension vector HNSW :
1. `delete_()` est un **NO-OP** — les edges restent indéfiniment dans les rel tables (fuite mémoire)
2. `initUpdateState()` **throw inconditionnellement** — impossible de faire `SET` sur une colonne indexée HNSW

## Analyse approfondie réalisée

### Architecture HNSW comprise en détail

- **Deux layers** : upper (sparse, ~5% des nœuds) et lower (tous les nœuds)
- **Rel tables FWD-only** : créées avec `storage_direction='fwd'` → `detachDelete(FWD)` ne supprime que les forward edges
- **`shrinkForNode()`** (hnsw_index.cpp:978-1052) fait déjà delete-all-edges + re-insert pruné — c'est le pattern qu'on réutilise
- **`HNSWInsertState`** contient tout : searchState, relDeleteState, relInsertState, nodesToShrink
- **Entry points** : `upperEntryPoint` et `lowerEntryPoint` dans `HNSWStorageInfo`, persistés
- **NULL embeddings** : skippés partout (insert, search, shrink) — un nœud supprimé retourne null → filtré
- **Lucivy FTS** sert de référence : son `update()` fait delete → re-read all columns → replace → re-insert

### Stratégie choisie : lazy cleanup des back-edges

Quand on supprime le nœud X :
1. Scanner ses forward edges AVANT suppression → récupérer la liste des voisins [A, B, C...]
2. Entry point replacement si X est entry point (premier voisin)
3. `detachDelete(FWD)` sur lower + upper rel tables → supprime X→A, X→B, X→C
4. **Trigger `shrinkForNode()` sur chaque ancien voisin** → les back-edges A→X, B→X, C→X sont nettoyées car `shrinkForNode` filtre les embeddings null (ligne 1009)

Coût : O(D × D_voisin) par suppression. Zéro fuite mémoire.

## Ce qui est FAIT

### 1. Header modifié : `extension/vector/src/include/index/hnsw_index.h`

**Stubs retirés de `HNSWIndex` base** (lignes 109-122 supprimées) :
- `initUpdateState()` qui throw → retiré (le `KU_UNREACHABLE` par défaut de `Index` base suffit)
- `initDeleteState()` qui retournait un state vide → retiré
- `delete_()` no-op → retiré

**Stubs ajoutés dans `InMemHNSWIndex`** :
```cpp
std::unique_ptr<DeleteState> initDeleteState(...) override { KU_UNREACHABLE; }
void delete_(...) override { KU_UNREACHABLE; }
```

**Nouveaux structs dans `OnDiskHNSWIndex`** (après HNSWInsertState) :
```cpp
struct HNSWDeleteState final : DeleteState {
    HNSWInsertState insertState;
    HNSWDeleteState(ClientContext*, TableCatalogEntry* node, TableCatalogEntry* upper,
        TableCatalogEntry* lower, NodeTable&, column_id_t, uint64_t degree)
        : insertState{...} {}
};

struct HNSWUpdateState final : UpdateState {
    HNSWInsertState insertState;
    HNSWUpdateState(/* mêmes params */) : insertState{...} {}
};
```

**Nouvelles déclarations publiques dans `OnDiskHNSWIndex`** :
```cpp
std::unique_ptr<DeleteState> initDeleteState(...) override;
void delete_(...) override;
std::unique_ptr<UpdateState> initUpdateState(...) override;
void update(...) override;
```

**Nouvelles déclarations privées** :
```cpp
void deleteFromGraph(Transaction*, offset_t, HNSWInsertState&);
std::vector<offset_t> scanNeighbors(Transaction*, offset_t, bool isUpperLayer, HNSWInsertState&);
```

### 2. Implémentation ajoutée : `extension/vector/src/index/hnsw_index.cpp`

Toutes les méthodes sont implémentées après `checkpoint()` (~ligne 643), AVANT `insertInternal()` :

- **`initDeleteState()`** : Récupère ClientContext depuis Transaction (champ `clientContext` privé sur Transaction), crée HNSWDeleteState
- **`delete_()`** : Itère nodeIDVector, appelle `deleteFromGraph()` pour chaque
- **`initUpdateState()`** : Crée HNSWUpdateState (au lieu de throw)
- **`update()`** : `deleteFromGraph()` puis `insertInternal()` si nouvelle valeur non-null. Utilise `CommitInsertEmbeddingScanState` pour créer un EmbeddingHandle depuis le propertyVector
- **`scanNeighbors()`** : Scanne les forward edges d'un nœud dans un layer, retourne la liste des offsets voisins
- **`deleteFromGraph()`** : Algorithme complet :
  1. `scanNeighbors(lower)` et `scanNeighbors(upper)`
  2. Entry point replacement si nécessaire
  3. `detachDelete(FWD)` sur les deux rel tables
  4. `shrinkForNode()` sur chaque ancien voisin (lazy cleanup)

## Ce qui reste à faire

### Problème potentiel : accès au ClientContext depuis initDeleteState

`initDeleteState()` reçoit un `const Transaction*` mais `HNSWInsertState` a besoin d'un `ClientContext*`. La classe `Transaction` a un membre `clientContext` (transaction.h:157) mais il est **privé**. Il n'y a pas de getter public `getClientContext()`.

**Solutions possibles** (à investiguer) :
1. Vérifier si `Transaction::Get(ClientContext&)` est un pattern existant — on pourrait passer par le NodeTable qui a accès au context
2. Regarder comment Lucivy gère ça — son `initDeleteState` ne crée pas de state complexe
3. **Approche simplifiée** : dans `initDeleteState`, retourner un state léger (juste un flag). Reporter la création du `HNSWInsertState` au premier appel de `delete_()` qui reçoit un `Transaction*` (et on peut essayer de retrouver le context)
4. **Mieux** : changer `initDeleteState` pour accepter un `ClientContext*` dans la signature — mais c'est une interface virtuelle de la classe `Index` base, modifiable car c'est notre fork

**Recommandation** : Option 4 est la plus propre. `initDeleteState` dans `Index` base (storage/index/index.h:144) prend `const Transaction*`. On peut ajouter un paramètre `ClientContext*` optionnel ou modifier la signature. Vérifier d'abord les autres callers.

### Tests à écrire/modifier

1. **`test/test_files/insert.test`** (lignes 41-43) : Le test `SET t.vec = [...]` attend actuellement une erreur. Après notre fix, ça doit **réussir**. Changer `---- error` en `---- ok` et ajouter une vérification search.

2. **Nouveau `test/test_files/update.test`** :
   - INSERT → search → SET vec → search (résultat mis à jour)
   - SET vec to NULL → search ne trouve plus
   - Multiple updates, update après CHECKPOINT

3. **`test/test_files/delete.test`** : Devrait toujours passer (le comportement observable ne change pas, mais maintenant les edges sont nettoyées en interne)

### Build et validation

```bash
# Build extension vector + tests
cd packages/rag3db/build/release
cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSION_TESTS=TRUE \
  -DBUILD_EXTENSIONS="vector" -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE
cmake --build . --target vector_test -j$(nproc)

# Lancer les tests
./test/runner/e2e_test extension/vector/test/test_files/
```

Ensuite revenir à rag3weaver pour valider les tests Phase 2 (le `SET n.kb_embedding = [...]` de l'EmbedProcessor ne devrait plus throw).

## Edge cases identifiés et gérés dans le code

1. **Delete entry point** → premier voisin comme remplacement, ou INVALID_OFFSET si graph vide
2. **Delete nœud avec embedding NULL** → jamais dans le graphe, detachDelete no-op
3. **Update NULL → non-NULL** → deleteFromGraph no-op, puis insertInternal
4. **Update non-NULL → NULL** → deleteFromGraph supprime edges, pas de re-insert
5. **Update seul nœud (entry point)** → entry point mis à INVALID_OFFSET par deleteFromGraph, puis insertInternal le remet comme entry point (insertToLayer lignes 920-924)
6. **Back-edges** → nettoyées via shrinkForNode sur chaque voisin

## Fichiers modifiés dans cette session

| Fichier | État |
|---------|------|
| `extension/vector/src/include/index/hnsw_index.h` | ✅ Modifié — structs + overrides ajoutés, stubs retirés |
| `extension/vector/src/index/hnsw_index.cpp` | ✅ Modifié — implémentations ajoutées (mais problème ClientContext à résoudre) |
| `extension/vector/test/test_files/insert.test` | ❌ À modifier |
| `extension/vector/test/test_files/update.test` | ❌ À créer |

## Plan de la session doc 12 (rappel)

Le plan initial du doc 12 (merge InsertOp + EmbedOp dans rag3weaver) est **abandonné** au profit de cette correction dans l'extension vector. Une fois l'extension corrigée, le flow existant de rag3weaver (InsertProcessor crée le chunk avec NULL embedding → EmbedProcessor fait SET) fonctionnera nativement.
