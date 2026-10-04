# Les prototypes C au niveau du fichier sont du bruit de références

- **État** : ouvert
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
