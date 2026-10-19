# Les statistiques par colonne ne sont pas compactées au point de reprise après un DROP

- **État** : corrigé le 19 octobre 2026 par le commit qui déplace ce fichier (`git log --follow`),
  V2-1a : les numéros ne bougent plus, les statistiques gardent leur entrée à leur numéro ; témoin
  `StableColumnIDs.TheRemainingColumnsKeepTheirOwnStatistics`
- **Gravité** : réponse fausse, en silence : le planificateur et `STATS_INFO` lisent le nombre
  de valeurs distinctes d'une autre colonne.
- **Atteignable en service** : oui, dès qu'une propriété qui n'est pas la dernière est
  supprimée puis qu'un point de reprise passe (rag3weaver supprime des propriétés aux
  migrations de schéma).
- **Touche rag3weaver** : oui (plans du produit estimés sur de fausses cardinalités).
- **Ouvert le** : 19 octobre 2026, session cœur C++.
- **Pour** : cœur C++, lot V2-1a (numéros de colonnes stables) : le défaut disparaît avec des
  numéros qui ne bougent plus, à condition que `TableStats` suive le même format à trous.

## Ce que c'est

Au point de reprise, `NodeGroupCollection::checkpoint`
(`src/storage/table/node_group_collection.cpp:213-231`) filtre `types` par `state.columnIDs`
(les colonnes encore vivantes) mais **pas `stats`** : `TableStats::columnStats` reste un vecteur
indexé par les anciens numéros. Juste après, `NodeTable::checkpoint` renumérote les colonnes de
façon contiguë (`vacuumColumnIDs(0)`, `node_table.cpp:1064`). Après un DROP de la colonne `k`
(pas la dernière), `columnStats[k]` porte toujours la statistique de l'ancienne colonne `k`
alors que le numéro `k` désigne désormais l'ancienne `k+1`. `getNumDistinctValues(columnID)`
(`src/include/storage/stats/table_stats.h:44-47`), lu par
`cardinality_estimator.cpp:168-171` et par `STATS_INFO` (`src/function/table/stats_info.cpp`),
rend donc la statistique d'une autre colonne. Un ADD qui suit range sa statistique à
`columnStats.size()`, qui n'est plus le nouveau numéro.

## Recette (lue, pas jouée)

```cypher
CREATE NODE TABLE T(id INT64 PRIMARY KEY, a INT64, b INT64);
UNWIND range(1, 1000) AS i CREATE (:T {id: i, a: i, b: i % 2});
CALL STATS_INFO('T') RETURN *;      // a : ~1000 distincts, b : 2
ALTER TABLE T DROP a;
CHECKPOINT;
CALL STATS_INFO('T') RETURN *;      // attendu b : 2 ; lu : b porte ~1000 (la statistique de a)
```

## Témoin

À écrire dans `test/transaction/` avec V2-1a : la recette, et la même après réouverture (le
vecteur dense est sérialisé tel quel, `table_stats.cpp:47-52`).

## Pour le fermer

Avec V2-1a, `TableStats` devient indexé par numéro stable (trous compris) et n'est plus jamais
« compacté » hors de la compaction sans transaction ; le témoin passe. Sans V2-1a, le correctif
local serait de filtrer `stats` comme `types` au même endroit.
