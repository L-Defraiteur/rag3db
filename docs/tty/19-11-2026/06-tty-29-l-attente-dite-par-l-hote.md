# tty-29 : l'attente dite par l'hôte, nommée à côté de l'animation

Session tty, 19 novembre 2026. C'est la seconde moitié de tty-25, sur l'événement que l'hôte émet depuis `cx-hote-waiting` (page 05) : `Waiting {run, cause, what, since_ms, wait_ms?}`, puis `Resumed`.

## Ce que dit le terminal

- **Un outil qui attend** (`cause: "tool"`) : la ligne vivante passe de « read_file README.md travaille… (N s) » à « read_file README.md attend… (N s) », compté depuis le début de l'attente (`since_ms`). `Resumed` rend « travaille… ».
- **Un modèle que le fournisseur retient** (`cause: "model"`) : son run est déjà fini, et la vue disait « à toi », ce qui était faux. L'animation reste, avec « gemini-3.8-flash attend le fournisseur… (reprise dans N s) », compté sur la fenêtre annoncée (`wait_ms`). `Resumed` la lève.
- Une autre cause dit « <nom> attend… (N s) ».

Le verbe « attend… » est le mot que le glitch déforme (`cause_spans`), et ce qu'il nomme reste lisible. Un nouvel appel, la fin d'un appel ou un nouveau run lèvent aussi l'attente dite.

## Les preuves

- `vue::tests::what_the_host_waits_for_is_named_beside_the_animation`, vu rouge (625b58dce2 : « travaille… (5 s) » là où l'attente durait depuis 25 s), puis vert (6ce49766ab).
- Le crate est vert avec la vue (119) et sans (22). clippy `-D warnings` est propre ; les mesures d'octets par image font partie des 119.
- La passe du terminal est verte (`cx-tty22-passe.sh`) : le crate compilé depuis le worktree, `check_features` 8/8, et les deux témoins `--plain` sur le binaire bâti.

## Le ticket

[La cause d'une attente dans le style du thème](../tickets/2026-11-15-la-cause-d-une-attente-dans-le-style-du-theme.md) : le style est fait (tty-25), et l'attente que seul l'hôte connaît l'est aussi, dès que `cx-hote-waiting` et ce lot sont sur master. Il se ferme avec le « poussé » de ce lot.
