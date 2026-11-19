# Le ménage du banc sur la tablette (18 novembre 2026)

Ma liste est consignée dans `docs/orchestration/18-11-2026/02-le-menage-de-la-tablette.md`, ligne « banc ». Lucie a
dit oui le 18 novembre.

## Ce qui est effacé, et par qui

- **Dix worktrees**, retirés par le banc avec `git worktree remove`, après une revérification : rien d'ouvert, aucune
  modification, HEAD identique à origin. La liste annonçait onze worktrees : c'était une erreur de compte, il y en
  avait dix. Leurs branches restent sur origin, comme archives. `df` est passé de 131 à 133 Go libres.
- **Les notes** (`~/.cache/rag3db-banc-notes/` : `source-freeze-16nov`, `public-corpus-20261116`, les dossiers
  d'octobre et des 12 et 13 novembre) ont été **effacées par Lucie elle-même**, le 18 à 13 h 55. Elle a tapé la
  commande dans la session de c7 (« ! cd ~/.cache/rag3db-banc-notes && rm -rf … /// je t'autorise explicitement »).
  C'est exactement la liste approuvée.
  - Le `rm -rf` que le banc avait tenté au même moment avait été refusé par le classifieur de permissions. **Il n'a
    rien fait.** Le banc ne l'a pas contourné.
  - Les notes gardées sont intactes : `etalonnage`, `compaction`, `decision`, `oracle-final-16nov`.

## Ce qui reste

- **Cinq worktrees** de lots posés : `~/.cache/rag3weaver-build/wt-banc-juge` à la tablette, et
  `rag3db-banc-e2`, `-e3`, `-repose` et `-juge` sous `~/git_workspaces/` à luciepc. Ils sont propres, et leurs
  branches sont sur master. `git worktree remove` les refuse parce qu'ils contiennent un sous-module initialisé, même
  après `submodule deinit`. Ils ne partent qu'avec `git worktree remove --force` : à faire au prochain ménage, au mot
  de Lucie.
- `wt-banc-etalonnage` et `target-banc-etalonnage` (l'étalonnage en cours), et `target-banc-juge` (luciepc et
  tablette, la passe des lots du banc).
