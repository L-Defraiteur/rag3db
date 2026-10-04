# WITH sans colonne utile filtré par un paramètre fait planter

- **État** : ouvert
- **Gravité** : plantage
- **Atteignable en service** : oui
- **Touche rag3weaver** : non (vérifié par l'arbre principal)
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : planificateur (banc, périmètre levé le 4 octobre)

## Ce que c'est

`WITH 1 AS gate WHERE $p … MATCH …` produit un filtre sans enfant et tue le processus.

## Recette minimale

```cypher
CREATE NODE TABLE person(ID INT64 PRIMARY KEY);
UNWIND range(0, 7) AS i CREATE (:person {ID: i});
-- préparé, depth = 1
WITH 1 AS gate WHERE $depth >= 2 MATCH (a:person) RETURN a.ID;   -- SIGSEGV
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — WithGateFilteredByAParameter

## Cause

`src/planner/plan/plan_projection.cpp:12-17` : retour avant `appendDummyScan`.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `8281c845e` (1er septembre 2026).

## Pour le fermer

le témoin passe au vert.
