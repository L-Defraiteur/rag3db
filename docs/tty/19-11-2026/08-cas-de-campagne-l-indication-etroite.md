# Cas de campagne : l'indication de la saisie vide, trop étroite

```json
{
  "parent": "f2a3872baf1895f1ae7ea6b2e984afb91fa21fcc",
  "tree": "extension/rag3weaver/tty",
  "paths": [
    "extension/rag3weaver/tty",
    "extension/rag3weaver/templates",
    "extension/rag3weaver/scripts"
  ],
  "needs_lib": false,
  "witness": "vue::tests::the_empty_input_says_how_to_break_a_line",
  "test": "cargo test -j 8 -- --exact vue::tests::the_empty_input_says_how_to_break_a_line",
  "suites": [
    "cargo test -j 8",
    "cargo test -j 8 --no-default-features",
    "cargo clippy -j 8 --all-targets -- -D warnings"
  ],
  "hints": [],
  "ours": "113d44f32359e25925e3621f52b4bba0a18d450b"
}
```

Le bloc est celui du lanceur (forme d'archi). `witness` porte les filtres exacts, séparés par une espace ; `test` les passe à libtest avec `--exact`. Les `suites` sont celles de la suite exigée `tty` qui existent au parent : les mesures `plain_*` n'y sont pas encore (elles datent du 16 novembre, après le parent).

`tree` est le dossier d'où les commandes partent ; `paths` sont les chemins à archiver (`git archive <parent> <paths>`) : le crate inclut les thèmes de `extension/rag3weaver/templates/themes` (`include_str!`), et sans eux il ne compile pas ; `theme::tests::the_palette_script_…` lance `../scripts/themes_palettes.py`, qui lit `templates/theme-roles.json` (relevé par archi au jeu de 18 h, corrigé le 19 au soir : avec `templates` et `scripts` entiers, seuls les témoins sont rouges au parent). Un target par cas : deux arbres de cas sur un même `CARGO_TARGET_DIR` ont rejoué le binaire de l'autre (vu le 19 nov. en vérifiant ces deux cas). Vérifié sur l'arbre archivé, dans un target propre : le témoin est rouge au parent.

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
