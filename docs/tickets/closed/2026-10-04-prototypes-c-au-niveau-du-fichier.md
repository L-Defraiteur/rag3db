# Les prototypes C au niveau du fichier sont du bruit de références

- **État** : corrigé — codeparsers `9531860` (11 oct. 2026, P1 de la stèle du produit) ; témoin `codeparsers/tests/prototypes_c.rs`
- **Gravité** : réponse fausse
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui

## Ce que c'est

Un prototype hors de tout namespace (`int libre(int a);` dans un `.h`, la
moitié des en-têtes C) ne devient ni scope ni membre : il tombe dans un
`file_scope_NN` dont les références viennent d'une expression régulière,
sans exclusion — `libre`, `a`, `int`… Les déclarations dans un namespace ou
une classe C++ sont traitées depuis la branche codeparsers `declarations`
(`c3a3656`) ; pas celles du niveau fichier.

## Recette minimale

```c
/* foo.h */
int libre(int a);
```

## Cause

`extract_file_scopes` (base) extrait les références du texte entre scopes
par regex (`extract_identifier_references_from_text`).

## Pour le fermer

Relever les `declaration` à déclarateur de fonction du niveau fichier
comme membres du scope de fichier, et retirer leurs noms des références.

## Correction (11 octobre 2026)

codeparsers `9531860` : `attach_file_level_prototypes` (commun au C et au
C++) fait d'un prototype du niveau fichier — y compris sous `#ifndef` et
`extern "C"` — un membre `Function` du scope de fichier, et retire à ses
lignes son nom et ceux de ses paramètres des références ; les types cités
restent. Trouvé en chemin et corrigé : un `struct` ou une `class` sans corps
(un paramètre `const struct Point* p`, une déclaration anticipée) faisait un
second scope du même nom. Arêtes d'appel de rag3db/src comparées une à une
avant/après : identiques.
