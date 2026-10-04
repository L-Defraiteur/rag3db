# COPY : un champ CSV vide entre guillemets devient NULL

- **État** : corrigé le 4 octobre 2026, commit `34bde7eea` (« fix(copy): un champ CSV vide entre guillemets se lit comme la chaîne vide, plus comme NULL »)
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

## Une décision avant le correctif (4 octobre)

Le correctif est écrit et éprouvé : une indication « champ entre guillemets » passe du
lecteur CSV au test des chaînes NULL, qui ne s'applique plus à un champ entre guillemets.
Le témoin passe au vert. Il est gardé hors de master, en patch
(`extension/rag3weaver/docs/3-octobre-2026-23h31/banc-de-concurrence/annexes/csv-champ-vide-entre-guillemets.patch`), parce qu'il casse un de nos
propres tests : `test/test_files/copy/escaped_newlines.test` affirme qu'un champ `""`
(ligne 6 de `dataset/copy-test/escaped-newlines/physical.csv` et `escaped.csv`) se lit
NULL, des deux côtés.

Choisir entre les deux, c'est décider de ce qu'un `""` veut dire dans nos CSV :
- la chaîne vide, comme le fait Ladybug et comme l'écrit un EXPORT ; on corrige alors aussi
  `escaped_newlines.test` ;
- NULL, comme aujourd'hui ; ce ticket se ferme alors comme « voulu », en le disant dans la
  documentation de COPY.

Tranché par Lucie le 4 octobre : `""` est la chaîne vide. NULL reste le champ vide non
cité, ou le mot déclaré par `null_strings`, comme PostgreSQL. rag3weaver n'est pas touché :
ses COPY déclarent leur propre `null_strings`, et `e2e_chemin_de_masse` l'affirme.

## Pour le fermer

le témoin passe au vert.
