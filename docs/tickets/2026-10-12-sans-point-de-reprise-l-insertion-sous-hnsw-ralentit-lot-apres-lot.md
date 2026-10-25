# Sans point de reprise, l'insertion de vecteurs sous index HNSW ralentit lot après lot

- **État** : ouvert — la part du CSR corrigée (12 octobre 2026, cœur C++, page coeur-cpp/13 §5 :
  la compaction des lignes mortes du CSR en mémoire, à la validation qui ajoute au nœud et à la
  fin de la validation qui a supprimé, pour les lignes que plus aucun instantané ouvert ne voit).
  Sur la lib du lot (d6b446956, trois passes sous `poste mesure`) : 5 derniers lots sans point de
  reprise contre avec, 154 / 131 ms (×1,17), 169 / 109 (×1,55), 152 / 126 (×1,20) — contre ×1,5
  à ×1,74 avant. Une quatrième passe sur le lot rebasé sur master 0c0af5bf5, qui porte 2bd65cbb8
  de hotfixs (la chaîne des versions raccourcie) : 154 / 113 ms, ×1,37. Ce qui reste de pente
  sans point de reprise (135 → 160 ms du lot 22 au lot 63) n'est donc ni les lignes mortes ni la
  chaîne des versions seule : c'est le coût propre du chemin de lecture en mémoire — les
  relations et les vecteurs écrits depuis le dernier point de reprise se lisent ligne à ligne
  (`ChunkedNodeGroup::lookup`, `isSelected`, l'info de mise à jour) là où le chemin persistant
  balaie des colonnes ; sans point de reprise, la part des voisins lus par ce chemin croît avec
  chaque lot. Le cas est en probabilistic.txt avec ses comptes ; à profiler sur un master qui
  porte les deux lots (le banc rejoue trois fois), et à fermer quand le témoin est vert trois
  passes sur trois
- **Gravité** : lenteur qui s'accumule. Sans point de reprise, le coût d'un lot de 64 SET
  croît d'environ 1,1 ms par lot, sans plafond vu sur 64 lots
- **Atteignable en service** : oui. rag3weaver pose les vecteurs par SET depuis NULL
  (`embed_set`), lot après lot, et un premier index en écrit des dizaines de milliers avant le
  point de reprise
- **Touche rag3weaver** : oui. C'est une part du temps d'embarquement (20 ms par vecteur mesurés
  par le proto avant les lots du cœur)
- **Ouvert le** : 12 octobre 2026, session du banc (après le lot de l'insertion du cœur C++,
  3cca0ae1e)
- **Pour** : le cœur C++ (la compaction de la liste d'un nœud au commit)

## Ce que c'est

Un index fait par CREATE sur 11 000 vecteurs de 768, puis 4 096 autres lignes qui reçoivent leur
vecteur par SET depuis NULL, en 64 lots de 64. L'index ne grandit que de 37 %, et un coût en
log N n'en bouge que de quelques pour cent. Or, sans point de reprise, le coût d'un lot :
- monte d'abord de 75-100 ms à 120-160 ms vers le lot 25 (le régime des rétrécissements, voir le
  ticket de l'extension, `2026-10-12-les-retrecissements-de-l-insertion-hnsw-alourdissent-son-cout`) ;
- puis continue de croître en ligne droite, environ 1,1 ms par lot, jusqu'à 160-230 ms au lot 63.

Avec un CHECKPOINT tous les 8 lots, la même courbe monte à 100-140 ms et reste plate jusqu'au bout.
Ce qui croît vit donc en mémoire jusqu'au point de reprise.

## Mesuré (master 9da274476, poste principal, sous `poste lourd`)

Moyenne des 5 derniers lots (ms), sans et avec un point de reprise tous les 8 lots, dans la même
passe :

| passe | sans | avec | rapport |
|---|---|---|---|
| 1 | 229 | 132 | 1,74 |
| 2 | 230 | 137 | 1,68 |
| 3 | 208 | 123 | 1,70 |
| deux passes de la veille de la borne | 163, 166 | 108, 110 | 1,5 |

Profil par échantillonnage de piles (eu-stack toutes les 0,05 s, fils actifs ; lots 0 à 15 contre
lots 16 à 63 sans point de reprise) : `CSRNodeGroup::scanCommittedInMemRandom` passe de 3,8 % à
28,6 % (cumulé), `ChunkedNodeGroup::lookup` de 23 à 41 %, `VersionInfo::isSelected` de 0 à 8 % de
temps propre ; la chaîne des versions pèse moins (`UpdateInfo::lookup` 0 → 9 %, `getEmbedding`
19 → 26 %) ; `RelTableData::findMatchingRow` reste sous 1 %.

## Cause (lecture du cœur C++, appuyée par le profil)

Chaque rétrécissement d'un nœud tue des lignes de sa liste dans le CSR des arêtes de l'index. Ces
lignes mortes restent dans l'index de la liste (`rowIndices`) jusqu'au point de reprise, et chaque
balayage des voisins de ce nœud (la recherche d'insertion) les relit et les écarte par
`isSelected`. Leur nombre croît avec les SET faits depuis le dernier point de reprise.

## Témoin

`VectorIndexInsertSlope.CostPerBatchSettles` (`test/transaction/concurrence/vector_index_cost_test.cpp`),
contrôle `insert-cost-without-checkpoint-stays-close` : à la fin, les 5 derniers lots sans point de
reprise au plus 1,2 fois ceux de la courbe avec, plus 2 ms. En known_red, rouge trois passes sur
trois. La courbe avec points de reprise est imprimée (fin sur plateau) sans contrôle : une garde
qui la jugeait (`checkpoint-keeps-insert-flat`) rougissait selon la charge du poste, au rejeu du
push du troisième pas de la jointure par clé (138,9 contre 100,8 ms sous une mesure et deux bâtis
d'autres sessions), et a été retirée le 12 octobre.

## Pour le fermer

La compaction de la liste d'un nœud au commit : retirer de `rowIndices` les lignes que ce commit
tue (elles restent dans le groupe pour les lecteurs d'avant, jusqu'au point de reprise). Le témoin
vert, joué plusieurs fois.

## Rejoué le 25 octobre 2026 (banc)

Master 6005e11cc, à luciepc, trois passes sous `poste mesure`, la charge relevée avant chacune :
load sur une minute de 6,3 à 7,9 (au-dessus de la référence d'octobre, environ 0,2 par cœur),
sans compilation ni test parallèle, et les moyennes sur 5 et 15 minutes encore hautes (15 à 23).

| passe | sans point de reprise, 5 premiers → 5 derniers | avec point de reprise, 5 derniers | rapport |
|---|---|---|---|
| 1 | 110 → 164 ms | 122 ms | ×1,35 |
| 2 | 76 → 158 ms | 119 ms | ×1,32 |
| 3 | 76 → 156 ms | 119 ms | ×1,32 |

Les chiffres retrouvent ceux d'octobre à 5 % près : la pente n'a pas changé sur master, et le défaut
reste entier. Une passe du cœur prise pendant son comparateur complet avait vu ×0,61 : sous cette
charge, tout ralentit d'environ trois fois, et la courbe avec point de reprise plus que l'autre. Le
témoin quitte donc `known_red.txt` pour `probabilistic.txt`, puisque c'est une borne de temps qui
dépend de la charge. Il se juge seul, sous `poste mesure`, avec la charge relevée.

