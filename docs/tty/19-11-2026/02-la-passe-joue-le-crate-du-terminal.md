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
