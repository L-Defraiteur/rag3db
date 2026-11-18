# Mesurer la pile de résolution de codeparsers sous le manifeste de Lucie

- **État** : fermé (18 novembre 2026, banc) — mesuré sous le manifeste de Lucie, à fichiers égaux : 3 779 Scope sans
  `DEFINED_IN` avant, **0** après (master du 18) ; l'exclusion n'écarte que `app.js` et `utf8proc_data.h`. Page
  `docs/banc/18-11-2026/02-la-pile-de-codeparsers-sous-le-manifeste-de-lucie.md`.
- **Famille** : produit
- **Gravité** : faible. Les chiffres livrés à codeparsers le 16 novembre viennent d'un manifeste sans
  l'exclusion de Lucie : 35 des 63 scopes sans `DEFINED_IN` sont un artefact de ce manifeste.
- **Atteignable en service** : oui. La pile de résolution sert à chaque index.
- **Touche rag3weaver** : non. Une mesure, sans code produit.
- **Pour** : une session Codex (passation du 16 novembre). Les chiffres vont à codeparsers.
- **Signalé par** : codeparsers et c7, 16 novembre.
- **Ouvert le** : 16 novembre 2026

## Le constat

La mesure du 16 novembre (`~/.cache/rag3weaver-build/codeparsers-measure.sh` à luciepc, sorties dans
`banc-parite/sorties/codeparsers-stack/`) a donné, sur deux index neufs de 1de1f86d88 :

| | avant (f10336d3f2) | après (41144f8d22) |
|---|---|---|
| Scope sans `DEFINED_IN` | 3 814 | 63 |
| CONSUMES résolution `type` | 24 834 | 29 043 |
| CONSUMES résolution `nom` | 20 145 | 18 480 |

Son manifeste dérive de `~/.cache/rag3weaver-build/ed-mesure-2a2/backend.json`, sans
`workspace.index_exclude`. Celui de Lucie
(`~/.config/rag3weaver/code-principal/templates/backends/code/backend.json` à luciepc) porte
`{"minified_line_max_bytes": 5000, "code_file_max_bytes": 1048576}`. Avec lui, le bundle
`extension/rag3weaver/templates/ide/app/app.js` n'est pas indexé.

## Ce qu'il faut faire

Les deux mêmes index neufs, avant et après, sur **l'arbre de Lucie**, avec **l'exclusion de Lucie** :
- la source : `~/git_workspaces/rag3db-banc-source-3` à luciepc, 284cc00c0b, sous-module compris.
  `git -C <source> submodule status` ne doit montrer aucune ligne commençant par « - » ;
- le code avant : f10336d3f2 ; le code après : la tête de master du moment ;
- le chemin : `~/.cache/rag3weaver-build/parite-ecrivain.sh`, comme dans `codeparsers-measure.sh`
  (`RAG3WEAVER_PARITE_NO_EMBEDDER=1 ROLES=… BASE0=<base vide> SRC=… SRCC=284cc00c0b SEUL=reindex`) ;
- **une modification de ce script** : après la ligne `sed … ed-mesure-2a2/backend.json > …`, si
  `INDEX_EXCLUDE=1`, ajouter au manifeste écrit
  `workspace.index_exclude = {"minified_line_max_bytes": 5000, "code_file_max_bytes": 1048576}`, par
  un petit `python3 -I`. Le script vit hors du dépôt : en garder la version d'avant à côté, en
  `.avant-index-exclude` ;
- les comptes : `CYPHER=~/.cache/rag3weaver-build/codeparsers-counts.cypher
  ~/.cache/rag3weaver-build/structure-sonde.sh <base> <sortie.json>`. Le nom de base par défaut est
  `code-rag3db-depot.rag3db`, ce qui est le cas ici. Puis la même table que ci-dessus, et les 63
  rejoués avec `~/.cache/rag3weaver-build/codeparsers-63.cypher` ;
- sous `POSTE_MEM_MAX=32G ~/.cache/rag3weaver-build/poste lourd …`.

## La preuve

- **L'artefact d'abord** : dans chaque base, `app.js` doit avoir 0 Scope
  (`MATCH (s:Scope) WHERE s.file_path ENDS WITH 'templates/ide/app/app.js' RETURN count(*)`). Le
  manifeste écrit doit porter `index_exclude`, et le journal dire la source et son commit.
- Le résultat attendu : les 35 scopes JavaScript partent ; il en reste à peu près 28 (Python et une
  lambda C++). L'« avant » recompté avec l'exclusion donne le vrai point de départ.
- Envoyer la table à codeparsers, et l'écrire dans une page du banc.

## Taille

Trois heures, dont deux d'index sous `poste lourd`.
