# COPY : un champ CSV vide entre guillemets devient NULL

- **État** : ouvert
- **Gravité** : réponse fausse
- **Atteignable en service** : oui
- **Touche rag3weaver** : non vérifié
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : lecteur CSV — à attribuer

## Ce que c'est

Un champ `""` est comparé aux chaînes NULL après le retrait des guillemets, et devient NULL. Un aller-retour EXPORT puis IMPORT change donc `''` en NULL.

## Recette minimale

```cypher
-- fichier q.csv : 1,""
CREATE NODE TABLE T(id INT64 PRIMARY KEY, s STRING);
COPY T FROM 'q.csv' (header=false);
MATCH (t:T) WHERE t.s IS NULL RETURN count(*);   -- attendu 0, obtenu 1
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — QuotedEmptyCsvFieldIsNotNull

## Cause

`src/function/cast_from_string_functions.cpp:861-870` ne sait pas si le champ était entre guillemets.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `2fc419036` (12 août 2026).

## Pour le fermer

le témoin passe au vert.
