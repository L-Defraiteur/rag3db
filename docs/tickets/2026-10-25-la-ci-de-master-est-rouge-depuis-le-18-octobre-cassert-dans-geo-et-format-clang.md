# La CI de master est rouge depuis le 18 octobre : un `<cassert>` dans geo et le format clang

- **État** : ouvert (relevé par l'orchestration le 25 octobre 2026 ; le premier point a son
  commit sur la branche `ci-cassert-geo`, baf19b9e5, qui part vers master à la fin de la fenêtre
  de hotfixs ; le second attend une fenêtre de formatage)
- **Gravité** : aucun run vert du « CI Workflow » dans l'historique récent : les autres jobs ne
  disent plus rien de master tant que ces deux-là rougissent.
- **Atteignable en service** : non (contrôle de CI).
- **Touche rag3weaver** : non (mais ses annexes de docs sont lues par le contrôle, voir 2).
- **Ouvert le** : 25 octobre 2026, session cœur C++, à la demande de l'orchestration.
- **Pour** : cœur C++.

## 1. « sanity checks » : `scripts/check-no-std-assert.sh extension`

Le script refuse tout `#include <assert.h>` ou `<cassert>` hors du moteur (l'assert du dépôt est
`KU_ASSERT`, `common/assert.h`). `extension/geo/src/index/rtree.cpp` incluait `<cassert>` sans
l'employer (aucun `assert(` dans geo). **Fait** : l'inclusion retirée, le contrôle vert en local
(`bash scripts/check-no-std-assert.sh extension`), commit baf19b9e5 sur `ci-cassert-geo` ; vers
master quand l'orchestration ouvre la fenêtre.

## 2. « code format » : `run-clang-format.py` avec clang-format-18 puis `git diff --exit-code`

Le job reformate `src/`, `test/`, `tools/`, `extension/` en place avec `/usr/bin/clang-format-18`
puis refuse tout diff. Aujourd'hui environ 300 fichiers diffèrent.

**La version n'a pas changé** : la CI épingle clang-format-18 depuis l'amont (b3917d99e, « Bump
clang-format to v18 and enable auto format », bien avant la bifurcation ; `.clang-format` n'a pas
bougé depuis 2022). La dérive est la nôtre : des fichiers écrits ou retouchés depuis la bifurcation
sans passer par clang-format-18 (les postes n'ont pas la 18 : ici clang-format 23, dont la sortie
diffère de la 18 sur ce style, donc **ne pas formater avec la version locale**), et les **29
annexes `.cpp`/`.h` de `extension/rag3weaver/docs/`** (3 octobre 2026 et suivants), des sondes
et des fragments de pages que ce contrôle n'a aucune raison de lire.

**Ce qu'il ne faut pas faire maintenant** : reformater 300 fichiers entrerait en conflit avec
toutes les branches ouvertes (format 41, V2-1a, le rattrapage HNSW de hotfixs).

**Plan, dans une fenêtre que l'orchestration ouvrira quand les lots moteur du jour auront
atterri** :
1. un fichier `.clang-format-ignore` à la racine (le script le lit par défaut,
   `DEFAULT_CLANG_FORMAT_IGNORE`), avec `extension/rag3weaver/docs/*` ; ou l'option `--exclude`
   dans les quatre pas du job ;
2. un **commit de formatage pur, sans logique**, produit par clang-format-18 lui-même (un
   conteneur ou le runner, pas la 23 locale), sur `src/`, `test/`, `tools/`, `extension/` hors
   docs ; relu par `git diff --stat` et par un bâti + `transaction_test`, rien d'autre n'y change ;
3. ensuite, chaque lot passe `run-clang-format.py --dry-run` avec la 18 avant son push, ou le job
   reste le juge.

## Témoin

Le « CI Workflow » de master vert sur ses deux jobs (sanity checks, code format) après 1 puis 2 ;
`gh run list --workflow 'CI Workflow'` en garde la trace.

## Pour le fermer

Les deux jobs verts sur master, l'exclusion des docs posée, et la règle du point 3 écrite dans la
page du cœur sur les passes.
