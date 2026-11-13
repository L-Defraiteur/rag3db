# Une suppression sous un index vectoriel rempli par SET coûte avec sa taille avant le point de reprise

- **État** : ouvert (13 novembre 2026, cœur c++)
- **Famille** : moteur
- **Gravité** : lenteur — une suppression de lignes indexées coûte environ quatre fois plus sur un
  index dix fois plus grand, tant qu'il n'est pas passé par un point de reprise
- **Atteignable en service** : oui en principe ; c'est la forme du produit (vecteurs posés par SET
  depuis NULL en lots de 64, puis suppression d'un lot de morceaux)
- **Touche rag3weaver** : par le moteur (le DETACH DELETE d'un lot de morceaux vectorisés)
- **Signalé par** : cœur c++ — banc décrit dans ce ticket
- **Pour** : cœur c++

## Ce que c'est

Le témoin `Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize`
(`test/transaction/concurrence/vector_index_cost_test.cpp`) compare le temps médian par ligne d'une
suppression sur deux tables indexées, de 1 500 et de 15 000 lignes, avant puis après un point de
reprise. Le seuil : la grande table sous deux fois la petite, plus 2 ms de marge contre le bruit
(`SIZE_FACTOR = 2`, `NOISE_MS = 2`). Pour un facteur 10 sur la taille, un coût en logarithme passe
et un coût linéaire rougit. Le seuil mesure donc ce qu'il doit mesurer : il ne se relâche pas sans
une mesure qui prouve le contraire.

Deux de ses trois variantes rougissent, seules, sur master sans aucun lot (le 9 novembre, sur
`813d7eda8`) comme avec le lot de l'annulation sous pool plein (le 13 novembre, bâti de
`48c3f6cdc`, à luciepc). La troisième, l'index créé par CREATE sur la table pleine, passe.

| Variante (13 novembre) | Index | 1 500 lignes | 15 000 lignes | Permis |
|---|---|---|---|---|
| `SetInBatchesOf64` | non checkpointé | 1,97 ms | 8,01 ms | 5,94 ms |
| `SetInBatchesOf64` | checkpointé | 1,96 ms | 2,39 ms | 5,91 ms |
| `SetInBatchesOf64WithRelation` | non checkpointé | 2,08 ms | 8,16 ms | 6,15 ms |
| `SetInBatchesOf64WithRelation` | checkpointé | 2,05 ms | 2,52 ms | 6,09 ms |
| `CreateOnFullTable` | non checkpointé | 1,73 ms | 2,15 ms | vert |

Le 9 novembre : 7,95 ms par ligne à 15 000 lignes pour 5,86 permises, 1,93 ms à 1 500.

**Lecture** : seul l'index rempli par SET et pas encore écrit par un point de reprise coûte avec sa
taille, environ quatre fois plus pour dix fois plus de lignes. Après le point de reprise, ou quand
l'index est bâti d'un coup par CREATE, le coût reste à peu près constant. La piste est donc la
partie en mémoire de l'index HNSW tenue entre deux points de reprise : la recherche des voisins à
réparer, ou le parcours des insertions en attente, au moment d'une suppression.

## Témoin

Les deux variantes elles-mêmes, inscrites dans `test/transaction/concurrence/known_red.txt` avec
l'étiquette `delete-independent-of-index-size`.

## Pour le fermer

La cause dans la suppression sous un index HNSW non checkpointé, le correctif, les deux variantes
vertes et leurs lignes retirées de `known_red.txt` dans le même commit. Ou bien une mesure, sous
`poste mesure`, qui prouve que la croissance vient du banc et non du moteur ; le seuil ne change
qu'avec elle.
