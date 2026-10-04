# MERGE d'une clé répétée rend une autre valeur que celle stockée

- **État** : ouvert
- **Gravité** : réponse fausse
- **Atteignable en service** : oui
- **Touche rag3weaver** : non vérifié
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : exécuteur — à attribuer

## Ce que c'est

Quand la même clé apparaît deux fois dans un lot de MERGE, la seconde ligne rend une valeur par défaut réévaluée (un UUID jamais stocké), ou l'identifiant d'un autre nœud.

## Recette minimale

```cypher
CREATE NODE TABLE A(id STRING DEFAULT gen_random_uuid(), stuff INT64, PRIMARY KEY(id));
UNWIND [1, 1] AS i MERGE (a:A {stuff: i}) RETURN a.id;   -- deux UUID, un seul stocké
CREATE NODE TABLE B(id SERIAL, stuff INT64, PRIMARY KEY(id));
UNWIND [1, 2, 1] AS i MERGE (b:B {stuff: i}) RETURN i, b.id;   -- la 3e ligne rend id 2, qui n'existe pas
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — MergeOfARepeatedKeyReturnsTheStoredNode

## Cause

`src/processor/operator/persistent/insert_executor.cpp:109-115`, `src/processor/operator/persistent/merge.cpp:74-78` : `skipInsert()` réévalue les valeurs par défaut et ne repose pas l'identifiant du nœud trouvé.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `c5a3c7385` (25 mai 2026). Ne reprendre que la relecture par identifiant ; leur correctif change aussi le nombre de lignes rendues.

## Pour le fermer

le témoin passe au vert.
