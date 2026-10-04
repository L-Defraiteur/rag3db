# Un terme de WHERE qui ne cite que des paramètres est ignoré

- **État** : ouvert
- **Gravité** : réponse fausse
- **Atteignable en service** : oui
- **Touche rag3weaver** : non (vérifié par l'arbre principal)
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : planificateur (banc, périmètre levé le 4 octobre)

## Ce que c'est

Dans le WHERE d'un MATCH ou d'un OPTIONAL MATCH, un terme qui ne cite aucune variable disparaît du plan, même relié par AND. Justes : `$x IS NULL OR n.f = $x`, et `WITH n WHERE $x`.

## Recette minimale

```cypher
CREATE NODE TABLE T(id INT64 PRIMARY KEY);
UNWIND range(0, 4999) AS i CREATE (:T {id: i});
-- préparé, flag = false
MATCH (n:T) WHERE $flag RETURN n.id;   -- attendu 0 ligne, obtenu 5 000
MATCH (n:T) WHERE $flag AND n.id < 10 RETURN n.id;   -- attendu 0, obtenu 10
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — PredicateOnParametersOnly

## Cause

`src/planner/plan/plan_join_order.cpp:57` n'écarte que les littéraux : le prédicat est rattaché à un graphe, marqué évalué, et jamais émis.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `8281c845e` (1er septembre 2026).

## Pour le fermer

le témoin passe au vert.
