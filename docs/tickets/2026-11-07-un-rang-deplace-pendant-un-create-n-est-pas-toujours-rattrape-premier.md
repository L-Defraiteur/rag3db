# Un rang déplacé pendant un CREATE de l'index vectoriel n'est pas toujours rendu premier à son nouveau vecteur

- **État** : ouvert, le 7 novembre 2026 (seconde session cœur C++, hotfixs). Le cas est inscrit à
  `test/transaction/concurrence/probabilistic.txt` ; le correctif entre en tête du lot G6 (HNSW hors
  verrous), avec un témoin déterministe s'il s'en trouve un.
- **Gravité** : réponse fausse en silence, à la marge : un rang mis à jour pendant la construction
  de l'index est trouvé, mais pas au premier rang, par une recherche à son vecteur exact.
- **Atteignable en service** : oui, par un CREATE_VECTOR_INDEX sous écritures (le rattrapage de
  (δ), 6fe9ffe9b, et le CREATE concurrent (A), 97daa41ed).
- **Touche rag3weaver** : oui, par la recherche dense.
- **Pour** : hotfixs (rattrapage de (δ)).

## Ce qu'on voit

`VectorIndexCreateInMemory.ChangesCommittedDuringACreateAreSeen` (`vector_index_update_test.cpp`),
check `rows-moved-during-create-found`. Pendant le bâti, des écrivains insèrent, suppriment et
déplacent des rangs. Après le CREATE, chaque rang déplacé est cherché à son nouveau vecteur et
doit sortir premier :

```
moved row 1 not found first at [500, 0 % 7, 0, …, -1]: got 3, at efs 2000 1|0.000000
create under writes: 333 inserted, 333 deleted, 267 moved, 66 moved then deleted during the build;
19934/19934 reached, 0 dead returned, 0 returned twice, 266/267 moved rows found at their new vector
```

Le rang 1 est trouvé, mais au troisième rang, alors que la distance à son vecteur est nulle.

## Les passes

À luciepc, le 7 novembre 2026, binaire du banc de `csr-listes-temoin-3` (moteur de master
ddb856e21) :

| Passe | Rouges |
|---|---|
| le banc complet (comparaison à known_red, charge ~15) | 1 sur 1 (266/267) |
| la famille `VectorIndexCreate*` répétée cinq fois (charge 15) | 1 sur 5 (466/467) |
| le cas seul, dix fois | 0 sur 10 |

Un rouge sur trois passes sous charge, aucun seul : l'entrelacement des écrivains avec le bâti
décide.

## Pour avancer

1. Lire pourquoi le rang n'est rendu qu'au troisième rang : son nouveau vecteur est-il dans le
   graphe avec trop peu de voisins (inséré par le rattrapage après l'élagage), ou l'ancien est-il
   encore atteint par un voisin, ou deux copies du même vecteur se disputent-elles le premier rang ?
2. Un témoin déterministe : un rang déplacé à un instant choisi du bâti (après le dernier lot du
   balayage, avant le rattrapage), puis la recherche à son vecteur.
3. Le correctif, puis la ligne de `probabilistic.txt` retirée dans son commit.
