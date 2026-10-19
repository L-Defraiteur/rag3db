# Le cache de colonne du CREATE_VECTOR_INDEX est clé par identifiant de propriété, l'index le cherche par numéro de colonne

- **État** : ouvert (lu dans le code le 19 octobre 2026, carte des numéros de colonnes pour
  V2-1a ; témoin à écrire)
- **Gravité** : blocage nommé : « Miss cached column, which should never happen. » au
  CREATE_VECTOR_INDEX d'une table dont une propriété a été supprimée avant la colonne indexée.
- **Atteignable en service** : oui : rag3weaver supprime des propriétés aux migrations de
  schéma et crée ses index de vecteurs après.
- **Touche rag3weaver** : oui.
- **Ouvert le** : 19 octobre 2026, session cœur C++.
- **Pour** : cœur C++ (une ligne ; avec V2-1a ou avant).

## Ce que c'est

`CACHE_ARRAY_COLUMN` crée son `CachedColumn` sous la clé `(tableID, propertyID)`
(`src/function/table/cache_column.cpp:61-62` ; signature `local_cached_column.h:11`), et
`InMemEmbeddings` le cherche sous `(tableID, columnID)`
(`extension/vector/src/index/hnsw_graph.cpp:72-80`, `CachedColumn::getKey(tableID, columnID)`).
Identifiant de propriété et numéro de colonne ne coïncident que tant qu'aucune propriété de la
table n'a été supprimée puis compactée (`vacuumColumnIDs` renumérote les colonnes, jamais les
identifiants de propriété). Après un DROP puis un point de reprise, `propertyID = 2` et
`columnID = 1` pour la même colonne : le cache est rempli sous 2, cherché sous 1, et l'index
lève l'exception.

## Recette (lue, pas jouée)

```cypher
CREATE NODE TABLE D(id INT64 PRIMARY KEY, junk INT64, vec FLOAT[4]);
CREATE (:D {id: 1, junk: 0, vec: [1.0, 0.0, 0.0, 0.0]});
ALTER TABLE D DROP junk;
CHECKPOINT;                        // vec : propriété 2, colonne 1
CALL CREATE_VECTOR_INDEX('D', 'idx', 'vec');   // attendu : créé ; lu : « Miss cached column »
```

## Témoin

À écrire dans `test/transaction/` (extension vector requise, comme
`copy_after_drop_vector_index_test.cpp`).

## Pour le fermer

Une seule clé : `cache_column.cpp` prend le numéro de colonne (`tableEntry->getColumnID(propertyID)`)
pour créer le cache, comme l'index le lit. Avec V2-1a les deux restent distincts (un numéro
stable n'est pas un identifiant de propriété) : la clé par numéro est la bonne.
