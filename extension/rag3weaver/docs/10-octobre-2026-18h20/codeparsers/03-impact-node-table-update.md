# impact NodeTable::update, tel que l'outil le rend

11 octobre 2026, au matin. Tout `rag3db/src` (1 543 fichiers C++, 19 046
scopes) indexé dans rag3weaver avec master `d6a2827ce` (codeparsers
`55c27d1`), en mémoire. La question de départ : « qui appelle
`NodeTable::update`, et qui tient le verrou avant ? ». Sonde :
`extension/rag3weaver/tests/sonde_cpp_rag3db.rs`.

## Ce que rend `impact`

L'outil tel qu'un agent l'appelle (`impact`, `name = update`,
`path = src/storage/table/node_table.cpp`, `depth = 3`) :

```
## Départ (1)
- function update — src/storage/table/node_table.cpp:619

## Code qui en dépend, à 1 saut (30)
- function set — src/processor/operator/persistent/set_executor.cpp:57
- function replayNodeUpdateRecord — src/storage/wal/wal_replayer.cpp:722
- … 28 lignes « (par le nom) »

## Code qui en dépend, à 2 sauts (1)
- function replayWALRecord — src/storage/wal/wal_replayer.cpp:440

## Code qui en dépend, à 3 sauts (1)
- function replay — src/storage/wal/wal_replayer.cpp:336
```

**Les deux appelants réels sont trouvés et sûrs** (marqués « type ») :
l'exécuteur de `SET` (`tableInfo.table->update(…)`) et le rejeu du WAL
(`…->cast<NodeTable>()`), puis le rejeu jusqu'à `replay`. Au début de la nuit,
l'outil n'en trouvait aucun.

**Les 28 lignes « par le nom »** sont du bruit : `update` est défini par
plusieurs classes, et `impact` garde les usages qu'il ne sait attribuer à
aucune. Ils noient les deux vrais.

## Ce qu'il dit du verrou

**Aucun appelant ne prend de verrou de mutex** : la section « Verrous pris
sur le chemin » reste vide sur ce chemin. Le verrou n'est pas tenu par
l'appelant avant `update`.

Le même outil dans l'autre sens (ce que `update` appelle) montre où il se
prend :

```
## à 1 saut
- lockKeyOfRow, lockRowForWrite — node_table.cpp
- isUnCommitted, usesLocks, shouldLogToWAL — transaction.cpp
## à 2 sauts
- lockKeyOf, throwIfWrittenByAnotherCommitAfterSnapshot — node_table.cpp
- acquireLock — transaction.cpp
## à 3 sauts
- acquireLocks — transaction.cpp
```

`update` verrouille lui-même la clé et la ligne par le **gestionnaire de
verrous de la transaction** (`lockKeyOfRow → lockKeyOf → acquireLock`), pas
par un mutex. Un verrou d'index à poser devant `update` se placerait là,
dans ce chemin, et non chez les appelants, qui n'en tiennent aucun.

**Ce que la relation LOCKS ne voit pas encore** : ces verrous applicatifs.
Elle relève les gardes sur un mutex (169 sur le moteur, par exemple
`ChunkedNodeGroup::versionInfoMtx` 18 fois, `HashIndexLocalStorage::mtx` 13,
`WAL::mtx` 5, `LockManager::mtx` 5). Pour le gestionnaire de verrous, il
faudrait déclarer, par dépôt, ses fonctions de verrou (`acquireLock`,
`lockRowForWrite`) : une liste dans le gabarit, pas une règle câblée.

## Avant et après la nuit

| Mesure | Avant | Après |
|---|---|---|
| Appelants de `NodeTable::update` vus par `impact` | 0 | **2** |
| Verrous relevés (LOCKS, rag3weaver) | 0 | **169** |
| Appels C++ reliés par codeparsers seul, entre fichiers | 35,0 % | 40,3 % |
| Appels C++ reliés en fichier seul (le reste passe par le rendez-vous) | 11,3 % | 11,7 % |
| Imports C++ reliés, entre fichiers | 24,9 % | 99,3 % |
| Non-résolues sur rag3db `src` (fichier seul) | 100 813 | 86 685 |
| Verrous relevés par codeparsers (exclusifs + partagés) | 0 | 172 |

Le relevé des non-résolues de codeparsers rendait 0 au départ : la colonne
« avant » vient du banc du début de nuit, une fois ce compteur corrigé.

## Ce que l'outil ne sait pas encore

- **Le bruit « par le nom »** des noms ambigus dans `impact` : à ranger dans
  une section à part, au lieu de les mêler aux usages sûrs.
- **Les verrous applicatifs** (gestionnaire de verrous) : une liste de
  fonctions de verrou par dépôt, déclarée au gabarit.
- **« Ce que ça appelle »** n'a pas d'outil : la vue sortante ci-dessus a
  été rendue par la sonde.
- **Les receveurs non typés** : 8 720 arêtes C++ restent posées par le seul
  nom. Le premier trou du banc est la méthode sur un receveur sans type
  (variables de boucle, paramètres de fermeture, liaisons de motif).
- Les réexportations et le « nom seul » : en tickets.

## Le 11 octobre au matin, après les lots de la nuit

Même sonde, sur master `6044723a5` (impact, les sûrs d'abord ; l'outil
`callees` ; les verrous déclarés), avec codeparsers `f91fc58` (les boucles
Rust ; les appels par chemin sur un type). Les fonctions de verrou de la
transaction déclarées comme un manifeste le ferait
(`workspace.locks_via: ["acquireLock", "acquireLocks", "lockRowForWrite"]`).

### `impact` (`name = update`, `path = src/storage/table/node_table.cpp`, `depth = 3`)

```
# impact: update

## Départ (1)
- function update — src/storage/table/node_table.cpp:619

Nom ambigu : toutes les définitions sont des départs ; les usages trouvés par le nom seul sont comptés à part, non montrés.

**4 touchés** (2 à 1 saut, 1 à 2 sauts, 1 à 3 sauts) — dont **0 Tests qui la traversent** ; 28 par le nom seul, non montrés.

## Code qui en dépend, à 1 saut (2)
- function set — src/processor/operator/persistent/set_executor.cpp:57
- function replayNodeUpdateRecord — src/storage/wal/wal_replayer.cpp:722

## Code qui en dépend, à 2 sauts (1)
- function replayWALRecord — src/storage/wal/wal_replayer.cpp:440

## Code qui en dépend, à 3 sauts (1)
- function replay — src/storage/wal/wal_replayer.cpp:336

## Verrous pris sur le chemin (1)
- `lockRowForWrite` — update (départ, lock)

```

Les deux appelants réels seuls ; les 28 homonymes en une ligne de compte
(`include_by_name = true` les montre) ; le verrou que `update` prend
lui-même, au départ.

### `callees`, le même nœud dans l'autre sens

```
# callees: update

## Départ (1)
- function update — src/storage/table/node_table.cpp:619

Nom ambigu : toutes les définitions sont des départs ; les usages trouvés par le nom seul sont comptés à part, non montrés.

**21 touchés** (7 à 1 saut, 8 à 2 sauts, 6 à 3 sauts).

## Ce qu’il appelle, à 1 saut (7)
- function usesLocks — src/transaction/transaction.cpp:59
- function shouldLogToWAL — src/transaction/transaction.cpp:99
- function isUnCommitted — src/transaction/transaction.cpp:217
- function lockKeyOfRow — src/storage/table/node_table.cpp:1012
- function lockRowForWrite — src/storage/table/node_table.cpp:1054
- method getStartOffsetOfNodeGroup — src/include/storage/storage_utils.h:56
- method getNodeGroupIdx — src/include/storage/storage_utils.h:59

## Ce qu’il appelle, à 2 sauts (8)
- function getMinUncommittedNodeOffset — src/transaction/transaction.cpp:334
- function lookup — src/storage/table/node_table.cpp:395
- function lockKeyOf — src/storage/table/node_table.cpp:1008
- function setToTable — src/storage/table/node_table.cpp:263
- function initScanState — src/storage/table/node_table.cpp:136
- function getSingleValueDataChunkState — src/common/data_chunk/data_chunk_state.cpp:10
- function acquireLock — src/transaction/transaction.cpp:94
- function throwIfWrittenByAnotherCommitAfterSnapshot — src/storage/table/node_table.cpp:1037

## Ce qu’il appelle, à 3 sauts (6)
- method getNumTotalRows — src/include/storage/local_storage/local_node_table.h:31
- method getStartOffset — src/include/storage/local_storage/local_node_table.h:46
- function getAsValue — src/common/vector/value_vector.cpp:244
- function acquireLocks — src/transaction/transaction.cpp:64
- function throwCouldNotSerialize — src/storage/table/node_table.cpp:1029
- method getNodeGroupIdxAndOffsetInChunk — src/include/storage/storage_utils.h:62

## Verrous pris sur le chemin (3)
- `acquireLock` — lockRowForWrite (1 saut, lock)
- `acquireLocks` — acquireLock (2 sauts, lock)
- `lockRowForWrite` — update (départ, lock)

```

Le chemin du verrou se lit d'un trait : `update` → `lockRowForWrite` →
`acquireLock` → `acquireLocks`. `lockKeyOf` et `lockKeyOfRow` n'y sont pas
comme verrous : elles calculent la clé, elles ne verrouillent pas.

### Les arêtes d'appel, par marque

| Marque | Début de nuit | Ce matin |
|---|---|---|
| type (receveur typé) | 3 377 | **5 517** |
| fichier | 5 088 | 5 308 |
| nom (deviné, non suivi par impact) | 8 720 | **7 215** |
| LOCKS | 169 | 178 (dont 9 par appel déclaré) |

Les 1 505 arêtes « nom » de moins sont devenues sûres, pour l'essentiel
par les appels par chemin sur un type (`StorageUtils::getNodeGroupIdx`,
`Catalog::Get`).
