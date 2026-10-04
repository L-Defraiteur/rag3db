# Deux défauts de chaînes annoncés par Ladybug, non reproduits chez nous

- **État** : ouvert — non reproduit
- **Gravité** : perte (annoncée) / réponse fausse (annoncée)
- **Atteignable en service** : incertain
- **Touche rag3weaver** : non vérifié
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : banc

## Ce que c'est

Ladybug corrige deux défauts de lecture de colonnes de chaînes ; les recettes déduites de leurs diffs donnent des résultats justes chez nous. Soit nous n'avons pas le défaut, soit il faut une autre forme.

## Recette minimale

```cypher
-- 254a7444d (annoncé : chaînes de relations réécrites aux mauvaises lignes au point de reprise d'une région)
CREATE NODE TABLE N(id INT64 PRIMARY KEY);
CREATE REL TABLE R(FROM N TO N, s STRING);
UNWIND range(0, 2999) AS i CREATE (:N {id: i});
MATCH (a:N), (b:N {id: 0}) WHERE a.id >= 1 CREATE (a)-[:R {s: concat('v', CAST(a.id % 3 AS STRING))}]->(b);
CHECKPOINT;
MATCH (a:N {id: 1500})-[r:R]->() DELETE r;
CHECKPOINT;
MATCH (a:N)-[r:R]->() WHERE r.s <> concat('v', CAST(a.id % 3 AS STRING)) RETURN count(*);   -- 0 chez nous
-- a10cecbc7 (annoncé : lecture filtrée sur plusieurs segments) : 20 000 chaînes de 300 caractères,
-- une ligne sur 97 supprimée : justes avant et après point de reprise.
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — RelationStringsKeepTheirRowsAcrossARegionCheckpoint (garde-fou vert ; pas de témoin pour a10cecbc7)

## Cause

annoncée chez Ladybug : `src/storage/table/string_column.cpp:166-191` et `dictionary_column.cpp:198` (254a7444d) ; `string_column.cpp:240` (a10cecbc7).

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `254a7444d` (9 juillet 2026), `a10cecbc7` (26 juillet 2026).

## Pour le fermer

trouver une forme qui rougit (dictionnaire trié, autre taille de segment), ou montrer en lisant le code que nous n'avons pas le défaut.
