# COPY refuse un CSV qui finit par un champ vide sans saut de ligne

- **État** : ouvert
- **Gravité** : réponse fausse
- **Atteignable en service** : oui
- **Touche rag3weaver** : non vérifié
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : lecteur CSV — à attribuer

## Ce que c'est

Un fichier qui se termine par `a,b,` sans saut de ligne perd son dernier champ vide : « expected 3 values per row, but got 2 ».

## Recette minimale

```cypher
-- fichier t.csv : 1,x,   (sans saut de ligne final)
CREATE NODE TABLE U(id INT64 PRIMARY KEY, a STRING, b STRING);
COPY U FROM 't.csv' (header=false);   -- refusé
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — CsvEndingWithAnEmptyFieldAndNoNewline

## Cause

`src/processor/operator/persistent/reader/csv/base_csv_reader.cpp:575-583` : aucune branche pour un dernier champ vide en fin de fichier.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `a8c619830` (3 juillet 2026).

## Pour le fermer

le témoin passe au vert.
