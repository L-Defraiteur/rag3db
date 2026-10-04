# OPTIONAL MATCH qui répète un nœud lié double les lignes

- **État** : corrigé le 4 octobre 2026, commit `e18dc42d5` (« fix(planificateur): un OPTIONAL MATCH qui répète seul un nœud déjà lié ne multiplie plus les lignes »)
- **Gravité** : réponse fausse
- **Atteignable en service** : oui
- **Touche rag3weaver** : non (vérifié par l'arbre principal)
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : planificateur (banc, périmètre levé le 4 octobre)

## Ce que c'est

Un nœud déjà lié sans étiquette, répété seul dans le motif d'un OPTIONAL MATCH, est rebalayé : les lignes sont multipliées.

## Recette minimale

```cypher
CREATE NODE TABLE L1(id INT64 PRIMARY KEY, k11 INT64);
CREATE NODE TABLE X(id INT64 PRIMARY KEY);
CREATE REL TABLE T1(FROM X TO L1);
CREATE (:X {id: 1})-[:T1]->(:L1 {id: 1, k11: 1});
MATCH (n0:L1), (n3) OPTIONAL MATCH (n6)-[:T1]->(n0), (n3) WHERE n0.k11 = 1 RETURN count(*);
-- attendu 2, obtenu 4
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — OptionalMatchRepeatingABoundNode

## Cause

`src/planner/plan/plan_join_order.cpp:51-53` : un graphe de requête sans relation, entièrement corrélé, n'est pas sauté.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `e36c93e59` (18 juillet 2026).

## Pour le fermer

le témoin passe au vert.
