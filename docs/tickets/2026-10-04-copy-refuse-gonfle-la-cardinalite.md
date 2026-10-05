# Un COPY refusé laisse la cardinalité de la table gonflée

- **État** : en partie corrigé (banc, 5 octobre 2026) — confort pour la stèle. L'estimation est recalée sur le nombre de lignes au point de reprise et à la lecture (voie (c), comme le `reltuples` de PostgreSQL) ; reste le moment d'avant tout point de reprise, avec la voie (a) rattachée à l'étape 4 du chargement journalisé (session cœur C++)
- **Gravité** : réponse fausse (et, avant `1ea49837f`, plantage)
- **Atteignable en service** : oui, par tout `COPY` refusé (clé en double, ligne mal formée…)
- **Touche rag3weaver** : non, par lecture : il n'appelle ni `STATS_INFO` ni aucune API de compte du moteur (ses comptes sont des `count(*)`) ; au plus, des plans plus lents à mesure que les paquets défaits gonflent l'estimation
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

## Ce que ce compte faux peut casser (5 octobre, carte des lecteurs)

Depuis `1ea49837f`, plus aucun lecteur ne dimensionne un tableau ni ne borne une boucle par
cette cardinalité estimée (`TableStats::cardinality`). Les frontières des algorithmes de
graphe, les masques de balayage, le partitionneur, et les index vectoriel, plein texte et
spatial lisent le nombre RÉEL de lignes (`numTotalRows`), que l'annulation recule et que la
lecture recalcule. Restent deux lecteurs de l'estimation :
- le planificateur : ordre des jointures, côté build/probe, SIP. Au pire, un plan plus lent ;
- `STATS_INFO`, qui la rend telle quelle.
D'où le classement « confort ».

## Cause

`NodeBatchInsert` fusionne les statistiques de son COPY dans celles de la table dès
`executeInternal` (`node_batch_insert.cpp:138`), donc avant sa validation, et avant que la clé
en double ne soit levée au `finalize`. L'annulation n'a aucun pendant : `TableStats` n'a pas de
recul. Au point de reprise et à la réouverture, l'estimation était écrite et relue telle quelle.

Même famille, par lecture : un COPY sous IGNORE_ERRORS compte ses lignes écartées, et DELETE ne
décrémente jamais. Le recalage les corrige aussi pour la cardinalité ; les comptes de valeurs
distinctes (HyperLogLog, qui ne se décrémentent pas) restent gonflés jusqu'à la voie (a).

## Le recalage (voie (c))

`NodeGroupCollection::checkpoint` et `deserialize` remettent la cardinalité sur `numTotalRows`
(lignes supprimées comprises). Témoins, `vector_index_update_test.cpp` :
- `RefusedCopyLeavesTheCardinalityTrueAfterACheckpoint`, vert ;
- `InflatedCardinalityIsRecalibratedAtOpening` : une base gonflée écrite par le moteur d'avant
  le recalage (400 pour 200), gardée compressée dans `dataset/databases/inflated-cardinality`
  avec son fabricant `FabricateADatabaseWithAnInflatedCardinality` (sauté par défaut). Elle se
  recale à l'ouverture. Vert ;
- `RefusedCopyLeavesTheCardinalityTrueAtOnce`, rouge connu (`cardinality-matches-rows-at-once`)
  jusqu'à la voie (a).

## Pour le fermer

La voie (a) : les statistiques du COPY restent dans la transaction, fusionnées au commit et
jetées à l'annulation. Elle est rattachée à l'étape 4 du chargement journalisé, quand la session
cœur C++ refait le chemin du COPY. La voie (b), reculer la cardinalité à l'annulation, est
écartée (les distincts resteraient faux).
