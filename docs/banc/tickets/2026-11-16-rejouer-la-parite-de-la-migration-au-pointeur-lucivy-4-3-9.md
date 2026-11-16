# Rejouer la parité de la migration au pointeur lucivy 4.3.9

- **État** : ouvert ; bloqué jusqu'au pointeur lucivy 4.3.9 sur master.
- **Famille** : produit
- **Gravité** : moyenne. C'est la dernière preuve que la bascule attend avant de passer les bases de
  Lucie aux générations.
- **Atteignable en service** : oui, chaque recherche lexicale d'une base migrée compte encore
  l'histoire des versions fermées.
- **Touche rag3weaver** : non. Une mesure, sans code produit.
- **Pour** : banc, ou une session Codex (passation du 16 novembre).
- **Signalé par** : c7, 16 novembre.
- **Ouvert le** : 16 novembre 2026

## Le constat

La page `docs/banc/16-11-2026/01-la-parite-de-la-migration.md` (§ 3 et § 10) montre que la copie
migrée A et un index neuf B′ du même arbre diffèrent en bm25 seul : rappel à 10 de 0,968, même
classement sur 14 cas sur 30. La cause est prouvée : le plein texte compte encore les versions
fermées. Retirer ces versions rend le classement de l'index neuf, au rang près (30 sur 30). lucivy
4.3.9 doit corriger ce point sans élaguer.

## Ce qu'il faut faire

Quand `extension/rag3weaver/Cargo.lock` de master porte `lucivy-core` en version `4.3.9` (ou
plus) :
1. rejouer à luciepc, par la tête de master, le relevé bm25 seul sur les copies A et B′ :
   `~/.cache/rag3weaver-build/b-prime-signals.sh`. Le script relit le worktree
   `~/git_workspaces/rag3db-banc-vue` à son commit : le mettre d'abord sur la tête de master
   (`git -C ~/git_workspaces/rag3db-banc-vue checkout --detach <hash>` puis
   `submodule update --init`) ;
2. rejouer la parité tous signaux A contre B :
   `~/.cache/rag3weaver-build/parity-migration.sh`. Ce script refait des copies de B et de A. Il
   attend aussi la fin d'une passe N de la mesure longue (`long-n-0653.log`), déjà finie ;
3. écrire le résultat en § 11 de la page de parité.

Les copies sont dans `~/.cache/rag3weaver-build/banc-parite/` : `migration-lot-A-20261116` (A),
`migration-ref-B-20261116` (B) et `migration-bprime-20261116` (B′). **Ne pas les effacer**, et ne
jamais ouvrir une base de service : on travaille sur des copies reflink.

## La preuve

- Le bm25 seul A contre B′ passe à « même classement 30/30 ». Les scores se jugent avec la
  tolérance du ticket `2026-11-16-la-tolerance-de-score-dans-le-comparateur-de-parite.md` s'il est
  posé, sinon au rang.
- Si ce n'est pas 30/30, c'est un défaut à nommer, avec la liste des cas.
- Chaque relevé dit ses signaux actifs (`[parité] active signals`) : un relevé sans `bm25` ou sans
  `vector` (en tous signaux) ne compte pas.

## Taille

Une heure : deux passes de 2 à 4 minutes, la lecture et la page.
