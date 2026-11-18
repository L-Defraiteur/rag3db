# tty-21 : les commandes et options du terminal en anglais

Session tty, 18 novembre 2026. Le lot reprend le travail de Codex (branche `cx-tty-english-aliases-7`, 16 et 17 nov.), évalué dans `docs/tty/18-11-2026/01-evaluation-du-travail-de-codex.md`. Seuls le code, les tests et le README sont repris ; l'appareil de qualification reste sur sa branche.

## Ce qui change

- `/help`, `/quit`, `/stop`, `/task`, `/repo`, `/log`, `/clear` (`/clear history`), `/steps`, `/name`, à côté des noms français, qui marchent toujours. `/aide` montre les deux, l'anglais d'abord.
- `--repo`, `--task`, `--request`, `--log`, `--steps`, `--address`, `--since`, à côté de `--depot`, `--tache`, `--demande`, `--journal`, `--etapes`, `--adresse`, `--depuis` et `--backend`, lus en silence. L'aide (`--help`) les dit anciens.
- Tab complète l'argument de `/task` et `/repo` : la complétion choisit ses sources par la commande lue (`commandes::lire`), plus par le préfixe `/tache` ou `/depot`. Ce défaut a été trouvé par Codex.
- L'affichage reste en français.
- Deux retouches de tty sur le code de Codex : le message d'erreur de `--since` dit `--since`, et l'aide de `/clear` prend la phrase et l'apostrophe des autres.

## Les témoins

- `commandes::tests::english_commands_preserve_legacy_semantics` et `english_completion_offers_steps_and_stop_and_task_arguments` : vus rouges sur master, au premier commit du lot (deux échecs), puis verts.
- `main::english_options_tests::english_options_match_legacy_and_last_value_wins` et `tui::tests::partial_english_and_legacy_commands_collect_catalogue_before_completion` : vus rouges sur la branche de Codex (9eab76ff61 pour le second), verts ici.
- Le crate passe 114 + 1 tests, et 22 sans vue ; clippy `-D warnings` ne signale rien.

Le ticket `docs/tickets/closed/2026-11-16-les-commandes-et-options-du-terminal-en-anglais.md` est fermé par ce lot.
