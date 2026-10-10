# L'index RTree de l'extension geo reçoit des décalages provisoires

- **État** : ouvert — confort, laissé pour plus tard (orchestration, 10 octobre 2026 : l'extension geo n'a aucun utilisateur) ; injoignable aujourd'hui, l'index ne se crée pas (voir plus bas)
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

## Essayé le 10 octobre 2026 (seconde session cœur C++) : l'index ne se crée même pas

La piste a été jouée sur luciepc (sonde jetable, `~/.cache/rag3db-tickets-notes/rtree_probe_scratch_test.cpp`) :

```cypher
LOAD EXTENSION '<racine>/extension/geo/build/libgeo.rag3db_extension';
CREATE NODE TABLE P(id INT64 PRIMARY KEY, x DOUBLE, y DOUBLE);
UNWIND range(0, 9) AS i CREATE (:P {id: i, x: CAST(i AS DOUBLE), y: CAST(i AS DOUBLE)});
CALL CREATE_SPATIAL_INDEX('P', 'idx', ['x', 'y']);
-- Binder exception: Trying to create nested type LIST without child information.
```

`QUERY_SPATIAL_INDEX` est refusé de même. La cause, par les lignes :
- pour une fonction de table sans `inferInputTypes`, le binder construit le type de chaque
  paramètre par `LogicalType(parameterTypeIDs[i])` (`src/binder/bind/bind_table_function.cpp:47-57`) ;
  pour un `LIST`, ce constructeur lève l'erreur faute de type d'élément
  (`src/common/types/types.cpp:543-549`) ;
- `CREATE_SPATIAL_INDEX` et `QUERY_SPATIAL_INDEX` déclarent un paramètre `LIST`
  (`extension/geo/src/function/create_spatial_index.cpp:264-277`,
  `query_spatial_index.cpp:215`) sans `inferInputTypes` ; `QUERY_VECTOR_INDEX`, lui, en a un.

L'index spatial de geo est donc inutilisable dans ce moteur, et le défaut de ce ticket injoignable
tant qu'il ne se crée pas.

## Le lot proposé, le jour où geo servira

1. Un témoin rouge : créer l'index, chercher, trouver le bon point.
2. Un `inferInputTypes` pour les deux fonctions (le type du paramètre fourni).
3. Puis la piste de ce ticket (`BEGIN`, insertion, `ROLLBACK`, recherche), rouge ou dissipée ; si
   rouge, le RTree par `commitInsert` et le crochet d'annulation.
Les fonctions scalaires de geo à paramètres `LIST` passent par un autre chemin : non essayées.
