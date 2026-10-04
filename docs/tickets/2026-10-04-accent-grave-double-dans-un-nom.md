# Un accent grave doublé dans un nom n'est pas réduit

- **État** : ouvert
- **Gravité** : réponse fausse
- **Atteignable en service** : oui
- **Touche rag3weaver** : non vérifié
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : analyseur — à attribuer

## Ce que c'est

Dans `` `a``b` ``, la double apostrophe inverse n'est pas réduite : le nom stocké est ``a``b`` au lieu de ``a`b``, et un EXPORT régénère un nom différent.

## Recette minimale

```cypher
CREATE NODE TABLE `a``b`(id INT64 PRIMARY KEY);
CALL show_tables() RETURN name;   -- attendu a`b, obtenu a``b
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — EscapedBacktickInAName

## Cause

`src/parser/transformer.cpp:193`.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `f03139f95` (12 août 2026).

## Pour le fermer

le témoin passe au vert.
