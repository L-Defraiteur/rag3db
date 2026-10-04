# Un COPY refusé laisse la cardinalité de la table gonflée

- **État** : ouvert — confort pour la stèle, sa condition posée (`STATS_INFO` dit qu'il rend une estimation)
- **Gravité** : réponse fausse (et, avant `1ea49837f`, plantage)
- **Atteignable en service** : oui, par tout `COPY` refusé (clé en double, ligne mal formée…)
- **Touche rag3weaver** : à vérifier ; son chargement en masse passe par `COPY`, et le test `e2e_code` qui meurt en interrompt un
- **Ouvert le** : 4 octobre 2026, session du banc (essai déterministe de la corruption d'`e2e_code`)
- **Pour** : cœur C++ (stockage, statistiques)

## Ce que c'est

Un `COPY` refusé est annulé : ses lignes disparaissent, mais pas ce qu'il avait ajouté aux
statistiques de la table. La cardinalité compte encore les lignes annulées, y compris
après réouverture : 401 pour une table de 200 lignes.

## Recette minimale

```cypher
CREATE NODE TABLE D(id INT64 PRIMARY KEY, vec FLOAT[4], s STRING);
UNWIND range(0, 199) AS i CREATE (:D {id: i, vec: [i, 1, 2, 3], s: 'x'});
CHECKPOINT;
-- refused.csv : les id 200 à 399, puis une ligne d'id 5 (clé en double)
COPY D FROM 'refused.csv' (header=false);      -- refusé : « Found duplicated primary key value 5 »
MATCH (d:D) RETURN count(*);                     -- 200
CALL STATS_INFO('D') RETURN cardinality;         -- attendu 200, obtenu 401 (et encore 401 après réouverture)
```

## Témoin

`test/transaction/concurrence/vector_index_update_test.cpp` :
- `RefusedCopyLeavesTheCardinalityTrue` : rouge, étiquette `cardinality-matches-rows` ;
- `CreateIndexAfterARefusedCopy` : la conséquence la plus grave du défaut, verte depuis
  `1ea49837f`. Avant ce commit, la sonde de la même recette tuait le processus (SIGSEGV,
  trois passes sur trois sur `b1b4df161`).

## Cause

Selon la lecture de la session cœur C++ :
- `NodeBatchInsert` fusionne ses statistiques dans celles de la table
  (`src/processor/operator/persistent/node_batch_insert.cpp:129-136`) ;
- l'annulation (`rollbackInsert`) ne touche que `numTotalRows`.

Les statistiques par colonne (nombre de valeurs distinctes) sont probablement gonflées de
la même façon. Ce n'est pas vérifié.

## Qui lit la cardinalité

Recherche de `getTableCard()` dans `src/` et `extension/`, le 4 octobre après `1ea49837f` :

- `src/planner/join_order/cardinality_estimator.cpp:45` et `:199` : l'estimation des plans.
  Une valeur gonflée ne rend qu'un plan moins bon, pas une réponse fausse.
- `src/function/table/stats_info.cpp:41` : `CALL STATS_INFO`, qui rend la valeur à
  l'utilisateur. C'est la réponse fausse du témoin.

Avant `1ea49837f`, l'extension vector la lisait aussi pour dimensionner ou borner :
- `extension/vector/src/function/create_hnsw_index.cpp:62` : le parcours de la création
  d'index, d'où le plantage ;
- `query_hnsw_index.cpp:293` et `vector_search_function.cpp:165` : le tableau des nœuds
  déjà visités.

Ces trois lectures lisent désormais `getNumTotalRows`. Aucun autre lecteur n'est trouvé ;
les fonctions de graphe (GDS) utilisent déjà le nombre total de lignes.

## Correctif de l'amont (à lire, ne pas copier)

Aucun trouvé dans la revue des amonts du 4 octobre.

## Pour le fermer

L'annulation d'un `COPY` retire aussi ce qu'il a fusionné dans les statistiques. Une
autre voie : ne fusionner les statistiques qu'à la validation. Le témoin
`RefusedCopyLeavesTheCardinalityTrue` passe alors au vert.

## La condition de la stèle (4 octobre 2026, banc)

La stèle classe ce ticket en confort à condition que `STATS_INFO` dise qu'il rend une
estimation. Le moteur n'a pas de champ de description pour ses fonctions ; la phrase est
sur la déclaration (`StatsInfoFunction`, `simple_table_function.h`) : des estimations
destinées au planificateur, `cardinality` compte aussi les lignes d'un `COPY` refusé, les
`*_distinct_count` sont approchés, et le compte exact se fait par `MATCH … count(*)`.
Renommer la colonne `cardinality` changerait ce que lisent les appelants : non fait, à
décider ailleurs.
