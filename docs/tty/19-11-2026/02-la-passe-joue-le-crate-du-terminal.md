# La passe épinglée joue le crate du terminal

Session tty, 19 novembre 2026. Trou relevé en déposant tty-28 au pousseur de G : un lot Rust se rejoue par `tools/poste/passe_epinglee.sh`, qui ne jouait ni le crate `extension/rag3weaver/tty` ni ses mesures. Un lot qui ne touchait que `tty/` passait donc VERT sans qu'un test du terminal tourne ; même son témoin, « lib », ne le trouvait pas dans la lib de rag3weaver.

## Le correctif

- `tools/poste/required_suites.txt` : la suite `tty`, exigée par `extension/rag3weaver/tty/*`. La garde du push l'exige donc aussi : une ligne « == tty » au résumé, et un journal `tty.log` qui dit « test result: ok ».
- `tools/poste/passe_epinglee.sh`, quand le lot exige `tty` : l'étape « == tty ». Ce n'est pas une suite e2e, elle sort de la liste des suites. L'étape joue, dans le target du worktree (`target-tty-wt/<worktree>`) :
  - `cargo test` avec la vue, puis `--no-default-features`, chacun après un `touch src/lib.rs` ;
  - `cargo clippy --all-targets -- -D warnings` ;
  - le binaire en release ;
  - les deux mesures `mesures/plain_stays_attached.py` et `plain_waits_for_a_resumed_run.py` sur ce binaire.

  L'étape rougit si un test, clippy ou une mesure rougit, ou si le crate n'a pas été compilé deux fois depuis le worktree du lot (la ligne « Compiling rag3weaver-tty (<worktree>) »).

## Le témoin

`tools/poste/test_passe_epinglee.sh`, sur le dépôt jouet, avec un faux `cargo` et de fausses mesures :
- un lot qui touche `tty/` joue le crate avec et sans vue, clippy et les deux mesures, et écrit « == tty » et `tty.log` ;
- un test du crate rouge, clippy rouge ou une mesure rouge rendent la passe rouge ;
- un lot qui ne touche pas `tty/` ne joue rien du terminal.

Vu rouge (f72cbcbe97 : le lot tty passait VERT sans un appel à `cargo`, même avec un test ou clippy rouge), puis vert : 42 cas. Les témoins de la garde (`test_push_garde.sh`) et du pousseur (`test_pusher.sh`) restent verts.

## Le témoin d'un lot du terminal : `SUITE_DU_TEMOIN=tty` (19 novembre, après-midi)

L'étape « témoin » ne savait jouer qu'un test de la lib de rag3weaver (`lib`), d'une suite e2e, ou du navigateur (`navigateur`). Le témoin d'un lot du terminal est un test du crate `extension/rag3weaver/tty`, que `run_e2e.sh` ne trouve pas. Depuis witness-must-run, il aurait été rouge à « aucun test joué ok » ; avant, il partait en `--test tty` et la passe restait verte sans le juger.

`SUITE_DU_TEMOIN=tty`, comme `navigateur` :
- l'étape témoin le renvoie à l'étape « tty », et rougit si le lot ne touche pas `tty/` ;
- l'étape « tty » exige, dans `tty.log`, la ligne `test <chemin>::<filtre> ... ok` ;
- le témoin n'est pas ajouté aux suites e2e.

Témoin de la passe : trois cas, vus rouges (91f7f383d6), puis verts (59 cas). Les témoins de la garde et du pousseur restent verts. Le dépôt d'un lot du terminal se fait ainsi : `enqueue <id> <branche> <tête> rust <filtre> tty - - tty`.

Le pousseur, à la relecture de G : `enqueue` accepte la suite du témoin `tty` (elle était refusée, ni `lib`, ni `navigateur`, ni `tests/tty.rs`), et un train ne l'ajoute pas aux suites en plus d'un lot qui suit (`replay_train`). Cas 32 de `test_pusher.sh`, rouge (50a6ddd67b), puis vert.
