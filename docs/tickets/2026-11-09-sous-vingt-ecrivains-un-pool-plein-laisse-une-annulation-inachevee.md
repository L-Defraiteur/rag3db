# Sous vingt écrivains, un pool plein laisse une annulation inachevée

- **État** : ouvert (9 novembre 2026, cœur c++)
- **Famille** : moteur
- **Gravité** : indisponibilité — la base refuse toute transaction jusqu'à sa réouverture
- **Atteignable en service** : à établir. Le banc le reproduit seul, sur master avant et après
  29a7bc52a ; un hôte sous un pool de tampons serré et des DDL concurrents y ressemble.
- **Touche rag3weaver** : par le moteur (l'hôte devrait fermer et rouvrir la base)
- **Pour** : cœur c++

## Ce que c'est

`TwentyWriters.DdlEveryMinute` (`test/transaction/concurrence/twenty_writers_scenario_test.cpp`)
rougit seul sous poste lourd, sur 813d7eda8 (master sans le correctif des deux sens) comme sur
52d73a301 (avec). La lib du banc est bâtie de 9bae60786 et suivants, en Release, à luciepc.

Ce que dit son journal (`~/.cache/rag3db-moteur-notes/csr/dep-base.log` à luciepc) :

| Message | Nombre |
|---|---|
| `BEGIN refusé : Buffer manager exception: Unable to allocate memory! The buffer pool is full and no memory could be freed!` | 9 |
| `BEGIN refusé : A rollback of this database could not complete, so it must be closed and reopened before it is used again` | 318 434 |

Le contrôle qui échoue est `index-creation-refuses-no-reader` (8 refus de lecteurs), puis une
exception non rattrapée (`unknown file: Failure`). Les fenêtres « 5 CHECKPOINT », « 6 ADD b
DEFAULT » et « 6 migration annulée » ne voient plus que des refus.

**Lecture** : un pool plein fait échouer une annulation, et la base se met d'elle-même en état
« à rouvrir » (la garde posée pour une annulation inachevée). Tout ce qui suit est refusé. La
question est de savoir si l'annulation peut échouer faute de mémoire, ou si elle devrait réserver
ce qu'il lui faut avant de commencer.

`TwentyWriters.DdlEveryFifteenSeconds` montrait les mêmes « buffer pool is full » dans le banc
entier, mais passe seul des deux côtés.

## Les seuils de temps notés ici

Deux cas de coût rougissent seuls, sur les deux mêmes versions. Ce sont des seuils à revoir avant
d'en faire un défaut :
- `Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize/SetInBatchesOf64` ;
- `Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize/SetInBatchesOf64WithRelation` : 7,95 ms
  par ligne à 15 000 lignes pour 5,86 permises, contre 1,93 ms à 1 500.

`TwentyWriters.KilledDuringTheCheckpoint` (enfant mort de SIGSEGV avant le point de reprise) n'a
été vu que dans le banc entier ; il passe seul des deux côtés. À rejouer sous charge, sans ticket.

## Témoin

`TwentyWriters.DdlEveryMinute` lui-même, rouge seul. Un témoin réduit reste à écrire : une
annulation qui échoue faute de pool, puis un BEGIN.

## Pour le fermer

La cause de l'annulation inachevée, le correctif, le témoin vert.
