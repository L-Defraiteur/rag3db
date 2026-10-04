# UNION plante quand une branche projette deux fois la même propriété

- **État** : ouvert
- **Gravité** : plantage
- **Atteignable en service** : oui
- **Touche rag3weaver** : non vérifié
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : planificateur (banc, périmètre levé le 4 octobre)

## Ce que c'est

Une branche de UNION qui projette deux fois la même propriété fait une lecture hors bornes.

## Recette minimale

```cypher
CREATE NODE TABLE Person(id INT64 PRIMARY KEY, age INT64);
CREATE (:Person {id: 1, age: 30});
MATCH (a:Person) RETURN 1, 2 UNION ALL MATCH (b:Person) RETURN b.age, b.age;   -- SIGSEGV
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — UnionArmProjectingAPropertyTwice

## Cause

`src/planner/operator/logical_union.cpp:14, 48`, `src/processor/map/map_union.cpp:23` : la portée dédupliquée est indexée par position.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `26be67f84` (25 juin 2026).

## Pour le fermer

le témoin passe au vert.
