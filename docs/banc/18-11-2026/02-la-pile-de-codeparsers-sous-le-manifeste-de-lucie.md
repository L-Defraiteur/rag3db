# La pile de résolution de codeparsers sous le manifeste de Lucie (18 novembre 2026)

Le ticket est `docs/banc/tickets/2026-11-16-mesurer-la-pile-de-resolution-de-codeparsers-sous-le-manifeste-de-lucie.md`.
La mesure du 16 novembre comptait 63 Scope sans `DEFINED_IN`. Son manifeste, celui de la base de test du banc, n'avait
pas l'exclusion de Lucie, et 35 de ces 63 venaient de `app.js`. Cette page la refait avec cette exclusion.

## La méthode

- **L'arbre** : 1de1f86d88, le même que le 16, pour que les chiffres restent comparables. La source est un worktree
  propre, avec son sous-module.
- **Après** : un index neuf par le master du 18 (29f5e50f6c, lib 3d706f709), sans embarqueur. Le manifeste de la base
  de test porte l'exclusion de Lucie : `workspace.index_exclude = {minified_line_max_bytes: 5000,
  code_file_max_bytes: 1048576}`. L'index dure 171 s.
- **Avant** : l'index neuf du 16 par f10336d3f2, **recompté sans les fichiers que l'exclusion écarte**. Le code d'avant
  ne se rebâtit pas sur la lib d'aujourd'hui : l'exclusion y est **simulée**, par un filtre sur le chemin dans chaque
  requête de compte.
- **Les fichiers écartés par l'exclusion** se lisent par différence entre les deux bases. Il n'y en a que deux :
  `extension/rag3weaver/templates/ide/app/app.js` et `third_party/utf8proc/include/utf8proc_data.h`.
- **À fichiers égaux** : la source du 16 n'avait pas le sous-module `codeparsers`, et l'après a 184 fichiers de plus,
  tous sous `extension/rag3weaver/codeparsers/`. Les comptes de l'après sont refaits sans ces 184 fichiers.
- Les requêtes sont `~/.cache/rag3weaver-build/codeparsers-counts.cypher` (rangées ouvertes) et
  `codeparsers-63.cypher`.

## Les chiffres

| à fichiers égaux | avant (f10336d3f2, exclusion simulée) | après (master du 18) |
|---|---|---|
| Scope ouverts | 75 707 | 75 801 |
| **Scope sans `DEFINED_IN`** | **3 779** | **0** |
| CONSUMES, résolution `fichier` | 48 454 | 53 456 |
| CONSUMES, résolution `type` | 24 834 | 30 897 |
| CONSUMES, résolution `nom` | 20 024 | 17 984 |
| CONSUMES, résolution `import` | 13 092 | 13 023 |
| CONSUMES, résolution `liaison` | 631 | 631 |
| arêtes vers l'API C (`src/c_api/`, visibilité déclaration) | 0 | 30 |
| CONSUMES depuis des fichiers `.c` | 892 | 1 480 |

- **Il ne reste aucun Scope sans `DEFINED_IN`**, ni sur les fichiers communs, ni sur la base entière (sous-module
  compris).
  - Les 35 de `app.js` partent avec l'exclusion.
  - Les 27 lambdas Python et la lambda de spdlog du 16 sont reliées aussi par le master d'aujourd'hui. Le ticket de
    la clé stable porte peut-être déjà leur correctif ; c'est à codeparsers de le dire.
- Les résolutions `type` et `fichier` gagnent, `nom` recule : c'est la même forme que le 16.

## Les sorties

À luciepc, `~/.cache/rag3weaver-build/banc-parite/sorties/codeparsers-manifest/` : le manifeste, les comptes (avant,
après, après à fichiers égaux), la liste des fichiers écartés. Les scripts sont `~/.cache/rag3weaver-build/cp-manifeste.sh`
et `cp-egal.sh`.
