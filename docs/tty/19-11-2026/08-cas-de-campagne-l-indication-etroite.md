# Cas de campagne : l'indication de la saisie vide, trop étroite

Session tty, 19 novembre 2026, pour la campagne des correctifs passés (chantier de Lucie ; lanceur d'archi, juge du banc). C'est un cas court, en Rust, dans le crate du terminal.

## Le point de départ

- **Commit parent** : `f2a3872baf9c` (15 novembre 2026, 15 h 31). Le témoin y est, rouge ; le correctif n'y est pas.
- **Arbre** : `extension/rag3weaver/tty` suffit (crate autonome, sans la lib rag3db).

## Le signalement, tel qu'il était

> Dans le terminal de l'agent, la saisie vide dit en sourdine comment passer à la ligne. Sous tmux, elle dit « sous tmux, Maj-Entrée envoie : Alt-Entrée pour la ligne (ou, dans tmux : set -g extended-keys on ; set -g extended-keys-format csi-u) ». Dans une fenêtre étroite (60 colonnes), cette indication est coupée par le début : elle perd « sous tmux, Maj-Entrée envoie », c'est-à-dire ce qu'elle a de plus utile. Vu par le harnais sous tmux. Trop étroite, l'indication doit garder son début, et dire qu'elle est coupée.

## Le témoin

- **Filtre** : `vue::tests::the_empty_input_says_how_to_break_a_line`, dans `extension/rag3weaver/tty/src/vue.rs`.
- **Suite** : `cargo test` du crate `extension/rag3weaver/tty`, avec la vue (défaut). Le témoin est identique sur master (vérifié le 19 nov. sur 63e24ff7a3).
- **Rouge au parent** : l'assertion « trop étroite, l'indication garde son début » échoue. L'indication à 60 colonnes ne commence pas par « › sous tmux, Maj-Entrée envoie : Alt-Entrée », ou ne finit pas par « … ».
- **Suites exigées par les chemins** : `tty` (`tools/poste/required_suites.txt`, depuis `cx-passe-tty`). C'est le crate avec et sans vue, clippy `-D warnings`, et les deux mesures `plain_*`.

## Notre correctif (caché à Algareth, pour le juge)

`113d44f323`, « fix(tty): trop étroite, l'indication de la saisie vide garde son début et finit par … » : `extension/rag3weaver/tty/src/vue.rs`, +21 −1.
