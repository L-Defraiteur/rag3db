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

## Pourquoi il n'est pas corrigé avec les autres (4 octobre)

Réduire l'accent grave doublé dans l'analyseur (`transformSymbolicName`) ne suffit pas :
un nom qui contient alors un accent grave s'exporte entre accents graves sans le doubler,
et l'EXPORT écrit du Cypher que l'IMPORT ne relit pas. Ladybug corrige aussi tous les
sites qui écrivent un identifiant dans du Cypher généré, par une fonction qui double
l'accent grave : 17 fichiers, dont le catalogue (`toCypher` des tables, des index, des
séquences, des propriétés), `export_db`, les fonctions de graphe et un fichier de
stockage. Ce n'est plus un correctif borné à l'analyseur ; arrêté là.

## Pour le fermer

le témoin passe au vert.
