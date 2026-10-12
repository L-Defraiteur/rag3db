# Les rétrécissements de l'insertion dans l'index HNSW sur disque multiplient son coût par 1,3 à 1,6

- **État** : ouvert
- **Gravité** : lenteur. Une fois les nœuds pivots arrivés à leur degré maximal, chaque insertion
  paie ses rétrécissements, et un lot de 64 SET passe de 75-105 ms à 120-155 ms, même avec des
  points de reprise
- **Atteignable en service** : oui, à tout embarquement (`embed_set`) dans un index déjà posé
- **Touche rag3weaver** : oui, le temps d'embarquement
- **Ouvert le** : 12 octobre 2026, session du banc (le témoin de la pente d'insertion, lecture du
  cœur C++)
- **Pour** : hotfixs (l'extension vector)

## Ce que c'est

Après un CREATE_VECTOR_INDEX, les degrés sont compacts, et les premières insertions par SET ajoutent
des arêtes sans rien rétrécir. Vers 1 300 SET, les nœuds pivots atteignent leur degré maximal, et
chaque insertion déclenche environ quatre rétrécissements : un balayage des voisins, la lecture de
leurs vecteurs, un `detachDelete` des arêtes du nœud, puis leur réinsertion. Le régime est permanent.
La construction en mémoire (`InMemHNSWLayer::shrinkForNode`) rétrécit sur place, sans supprimer et
réinsérer les arêtes.

## Mesuré (master 9da274476, poste principal, sous `poste lourd`)

Le témoin de la pente (`VectorIndexInsertSlope.CostPerBatchSettles`), courbe avec un CHECKPOINT tous
les 8 lots, ce qui écarte la part qui vit en mémoire (son ticket :
`2026-10-12-sans-point-de-reprise-l-insertion-sous-hnsw-ralentit-lot-apres-lot`). Moyenne des 5
premiers lots, puis des lots 25 à 30, en ms par lot de 64 :

| passe | premiers lots | lots 25 à 30 | rapport |
|---|---|---|---|
| 1 | 105 | 154 | 1,46 |
| 2 | 84 | 136 | 1,62 |
| 3 | 105 | 135 | 1,29 |

Au profil (lots 16 à 63, sans point de reprise), `OnDiskHNSWIndex::shrinkForNode` pèse 16,5 % en
cumulé et `createRels` 31 % ; sur les lots 0 à 15, `shrinkForNode` n'apparaît pas.

Pour référence, `CREATE_VECTOR_INDEX` sur 11 000 vecteurs prend 0,42 à 0,57 ms par vecteur, et
l'insertion ligne à ligne 1,2 à 2,4 ms (lots de 64).

## Témoin

Aucun contrôle : le rapport « lots 25 à 30 sur premiers lots » de la courbe avec points de reprise
est imprimé par `VectorIndexInsertSlope.CostPerBatchSettles`. Une borne s'écrira avec le correctif
(par exemple : le rapport au plus 1,15).

## Pour le fermer

Le rétrécissement sur place dans l'index sur disque, comme la construction en mémoire le fait,
au lieu de `detachDelete` puis des réinsertions. Le rapport remesuré.
