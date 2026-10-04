# L'index RTree de l'extension geo reçoit des décalages provisoires

- **État** : ouvert — confort pour la stèle ; établi par lecture, non exécuté
- **Gravité** : réponse fausse (annoncée par la lecture)
- **Atteignable en service** : incertain — il faut l'extension geo et un index RTree
- **Touche rag3weaver** : non (il ne se sert pas de l'extension geo)
- **Ouvert le** : 5 octobre 2026, session cœur C++ (carte des détenteurs de décalages provisoires)
- **Pour** : cœur C++, quand l'extension geo servira

## Ce que c'est

À l'insertion ordinaire d'un nœud, `NodeTable::insert` appelle chaque index avec l'identité **provisoire** du nœud (`src/storage/table/node_table.cpp`, boucle sur `indexes`). L'index de clé primaire et l'index vectoriel n'en font rien à ce moment : ils reçoivent les décalages définitifs au commit (`needCommitInsert`). Le RTree, lui, insère tout de suite sous ce décalage (`extension/geo/src/index/rtree_index.cpp:75-98`, de même pour la mise à jour et la suppression, `:106-139`), et rien ne le remappe ensuite.

Avec un seul écrivain, provisoire et définitif sont égaux dans le cas ordinaire. Ils diffèrent en mode multi-écrivains. Et rien ne retire ces entrées à l'annulation de la transaction — par lecture, aucun code d'annulation n'a été trouvé pour cet index.

## Recette minimale

Aucune : non exécuté. Piste : une table à index RTree, `BEGIN`, une insertion, `ROLLBACK`, puis une recherche spatiale qui couvre le point inséré.

## Témoin

Aucun.

## Pour le fermer

Exécuter la piste ci-dessus pour dire si le défaut est réel ; s'il l'est, faire passer le RTree par `commitInsert` comme les deux autres index, et lui donner le crochet d'annulation que demande le ticket de l'index vectoriel (`2026-10-05-copy-annule-sur-une-table-a-index-vectoriel.md`).
