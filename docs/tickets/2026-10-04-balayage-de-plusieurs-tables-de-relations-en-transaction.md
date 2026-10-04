# Dans une transaction, un balayage de plusieurs tables de relations relit les relations d'une autre table

- **État** : ouvert
- **Gravité** : réponse fausse
- **Atteignable en service** : oui
- **Touche rag3weaver** : non vérifié
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : cœur C++ (stockage)

## Ce que c'est

Dans une transaction, `[r:R1|R2]` relit sous R2 les relations non validées de R1 : doublons, voire colonnes mélangées. Juste après COMMIT.

## Recette minimale

```cypher
CREATE NODE TABLE P(id INT64 PRIMARY KEY);
CREATE REL TABLE R1(FROM P TO P);
CREATE REL TABLE R2(FROM P TO P);
CREATE (:P {id: 1}), (:P {id: 2});
BEGIN TRANSACTION;
MATCH (a:P {id: 1}), (b:P {id: 2}) CREATE (a)-[:R1]->(b);
MATCH (a:P)-[r:R1|R2]->(b:P) RETURN count(*);   -- attendu 1, obtenu 2
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — ScanOfSeveralRelationTablesInATransaction

## Cause

`src/storage/table/rel_table.cpp:33-53` : `setToTable` ne remet pas à zéro l'état de balayage local, partagé entre tables par `scan_multi_rel_tables.cpp:45-46`.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `a8814a143` (23 avril 2026).

## Pour le fermer

le témoin passe au vert.
