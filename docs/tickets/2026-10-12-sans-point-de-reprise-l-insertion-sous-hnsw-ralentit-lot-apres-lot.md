# Sans point de reprise, l'insertion de vecteurs sous index HNSW ralentit lot après lot

- **État** : ouvert
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
trois. La garde `checkpoint-keeps-insert-flat` (avec points de reprise, les 5 derniers lots au plus
1,2 fois les lots 25 à 30) est verte.

## Pour le fermer

La compaction de la liste d'un nœud au commit : retirer de `rowIndices` les lignes que ce commit
tue (elles restent dans le groupe pour les lecteurs d'avant, jusqu'au point de reprise). Le témoin
vert, joué plusieurs fois.
