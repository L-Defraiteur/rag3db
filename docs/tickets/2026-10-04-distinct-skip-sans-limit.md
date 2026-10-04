# RETURN DISTINCT … SKIP sans LIMIT rend zéro ligne

- **État** : ouvert
- **Gravité** : réponse fausse
- **Atteignable en service** : oui
- **Touche rag3weaver** : non (vérifié par l'arbre principal)
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : optimiseur (banc, périmètre levé le 4 octobre)

## Ce que c'est

`RETURN DISTINCT … SKIP n` sans ORDER BY ni LIMIT rend 0 ligne. SKIP sans LIMIT après une extension récursive rend trop peu de lignes. Justes : sans DISTINCT, avec ORDER BY, avec LIMIT, WITH DISTINCT … SKIP.

## Recette minimale

```cypher
CREATE NODE TABLE T(id INT64 PRIMARY KEY);
UNWIND range(0, 4999) AS i CREATE (:T {id: i});
MATCH (n:T) RETURN DISTINCT n.id SKIP 10;   -- attendu 4 990 lignes, obtenu 0
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — SkipWithoutLimit

## Cause

`src/optimizer/limit_push_down_optimizer.cpp:40-67` pousse `skip + UINT64_MAX`, qui déborde ; `src/processor/map/map_distinct.cpp:16-25` prend alors la limite égale au skip.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `079aa8458` (18 août 2026).

## Pour le fermer

ne pousser une limite que si elle existe ; le témoin passe au vert.
