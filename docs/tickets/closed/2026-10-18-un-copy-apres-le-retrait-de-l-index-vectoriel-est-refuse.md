# Un COPY après le retrait de l'index vectoriel est refusé : la finalisation du COPY touche l'index retiré

- **État** : corrigé le 18 octobre 2026 par le commit qui ferme ce ticket (cœur C++), ouvert et
  fermé dans le même commit (régression trouvée par l'arbre principal dans la suite du produit).
- **Gravité** : refus d'un COPY valide (« Catalog exception: _N_<index>_UPPER does not exist in
  catalog »), que le produit traduit en base à rouvrir ; l'ingestion en masse du produit (DROP de
  l'index, COPY, CREATE) ne passait plus.
- **Atteignable en service** : oui, depuis la rétention des index (9b861579a) : tout COPY d'une
  table dont un index vectoriel a été retiré par une transaction validée avant lui.
- **Touche rag3weaver** : oui (`bulk_vector_index`, `e2e_code::building_the_vector_index_in_bulk_beats_row_by_row`,
  rouge sur master faf94cdfc avec la lib 5e3df4563, vert avec la lib 87a362e30 d'avant la
  rétention).
- **Ouvert le** : 18 octobre 2026, session cœur C++, sur le signalement de l'arbre principal.
- **Pour** : cœur C++.

## Ce que c'est

Depuis la rétention, un `DROP_VECTOR_INDEX` validé ne retire pas le holder de la liste des index
de la table : les lecteurs d'avant le voient encore, le point de reprise le récupère. La
finalisation d'un COPY (`NodeBatchInsert::finalize`, `node_batch_insert.cpp:324`) parcourait
**tous** les holders, sans visibilité (`getIndexes()`), et appelait `finalize` sur l'index
retiré : `OnDiskHNSWIndex::finalize` voit plus de lignes dans la table que de lignes reprises,
cherche ses tables internes au catalogue (`getIndexTableCatalogEntries`) et les trouve parties
avec le DROP. Le chemin de validation ordinaire (étape 3 de `NodeTable::commit`) filtre, lui, par
`!isDetached() && isIndexVisible(holder, transaction)`.

Recette (sonde du 18 octobre, déterministe, rouge sur 8f4779bdc comme sur 5e3df4563) :

```cypher
CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);
CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');
CALL DROP_VECTOR_INDEX('Doc', 'doc_index');
COPY Doc FROM 'docs.csv' (header=false);   -- refusé : _0_doc_index_UPPER does not exist in catalog
```

Pourquoi aucune suite ne l'a vu : aucune ne jouait `DROP_VECTOR_INDEX` suivi d'un COPY ; la suite
vector et transaction_test jouent le DROP seul, le COPY seul, ou le COPY sous un index vivant.

## Correctif

- `NodeBatchInsert::finalize` ne finalise que les index que la transaction du COPY voit et
  écrit : `!isDetached() && NodeTable::isIndexVisible(holder, transaction)`, comme l'étape 3 de
  la validation (un index retiré avant le COPY, ou créé par une autre transaction non validée,
  est sauté).
- `NodeTable::rollbackIndexInsert` reçoit la transaction qui annule et applique le même filtre
  (il reculait le compte d'un index retiré ou d'un CREATE en cours d'une autre transaction :
  sans perte, mais faux de principe ; relu par hotfixs).

Reste, à hotfixs (rétention) : `getIndexHolder`, `getIndex`, `getSharedIndex` et `loadIndex` par
nom filtrent `!isDropped()` et non la visibilité (l'extension geo ne voit plus un index R-tree
retiré par une transaction validée après le début de son lecteur ; un nom recréé non validé peut
être pris par une autre transaction).

## Témoin

`test/transaction/copy_after_drop_vector_index_test.cpp` (`CopyAfterDropVectorIndex.*`, se saute
sans l'extension vector) : un COPY dans une table vide après le DROP ; le chemin de masse du
produit sur une table déjà remplie et reprise (DROP, COPY d'un second lot de nœuds et de
relations, CREATE, recherche) ; le même dans un `BEGIN … COMMIT` ; et un lecteur ouvert avant le
DROP qui cherche encore dans l'ancien index pendant le COPY d'après. Chaque cas relu après
`CHECKPOINT` et après réouverture. Rouge sur master, vert sur le correctif.
