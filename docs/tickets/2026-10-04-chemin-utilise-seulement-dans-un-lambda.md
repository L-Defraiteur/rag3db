# Un chemin utilisé seulement dans un lambda fait planter

- **État** : ouvert
- **Gravité** : plantage
- **Atteignable en service** : oui
- **Touche rag3weaver** : non vérifié
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : optimiseur (banc, périmètre levé le 4 octobre)

## Ce que c'est

Quand un chemin n'est utilisé que dans le corps d'un lambda, l'optimiseur élague ses segments et la requête tue le processus.

## Recette minimale

```cypher
CREATE NODE TABLE Person(name STRING PRIMARY KEY);
CREATE REL TABLE K(FROM Person TO Person);
CREATE (:Person {name: 'Alice'})-[:K]->(:Person {name: 'Bob'})-[:K]->(:Person {name: 'Charlie'});
MATCH p = (a)-[*1..2]-(b)-[*1..2]-(c)
WHERE a.name = 'Alice' AND b.name = 'Bob' AND c.name = 'Charlie'
RETURN any(x IN [1] WHERE p IS NOT NULL);   -- SIGSEGV
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — PathUsedOnlyInsideALambda

## Cause

`src/optimizer/projection_push_down_optimizer.cpp` ne descend pas dans les expressions lambda.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `0c19d7816` (25 juin 2026).

## Pour le fermer

le témoin passe au vert.
