# QueryResult::toString() consomme le résultat

- **État** : corrigé le 4 octobre 2026, commit « fix(api): QueryResult::toString ne consomme plus le curseur du résultat »
- **Gravité** : réponse fausse
- **Atteignable en service** : oui (par l'API C++)
- **Touche rag3weaver** : non vérifié
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : API — à attribuer

## Ce que c'est

`MaterializedQueryResult::toString()` avance le curseur du résultat au lieu d'une copie : après lui, `hasNext()` vaut faux ; sur un résultat déjà entamé, la chaîne est incomplète.

## Recette minimale

```cypher
// C++
auto r = conn->query("MATCH (p:P) RETURN p.id;");
r->toString();
r->hasNext();   // attendu true, obtenu false
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — ResultToStringKeepsTheCursor

## Cause

`src/main/query_result/materialized_query_result.cpp:82-83` : `iterator->` au lieu de la copie locale.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `ef1e0e3cc` (15 novembre 2025).

## Pour le fermer

le témoin passe au vert.
