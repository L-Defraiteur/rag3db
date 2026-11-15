# tty-16 (suite) : jamais un mot d'état nu, aucun verdict de l'hôte, la cause à côté de l'animation

Session tty, 15 novembre 2026, sur les retours de Lucie au binaire d'essai, rapportés par c7.

## Ce qu'elle a dit

- « j'ai plus la petite animation de chargement :/ »
- « pourquoi ça m'écrit "attend", c'est horrible, on doit jamais voir "attend" comme ça : que des animations de chargement ou bien des trucs vivants »
- « "fait sans plan" veut rien dire pour l'user, il s'en cale »
- « je veux voir l'animation et comprendre ce qu'il attend exactement »

## L'animation dès que l'équipier part

Relu sur le flux de sa tâche atelier : l'hôte n'émet `LlmCall` qu'au retour du modèle (l'événement porte `ms` et `finish`). Entre `RunStarted` et ce premier retour, la vue restait sur « attend » pendant toute la première réflexion. Désormais, `RunStarted` passe la vue en « réfléchit ».

## La ligne d'état

- **Au travail** : l'animation du thème.
- **Pendant que le texte du modèle coule** : le cœur du thème suivi de « … ». Le texte qui coule est le mouvement. L'image entière, même figée, redite à chaque ligne qui monte, passait le plafond d'octets par token : 36,7 Ko pour 1 000 tokens, pour un plafond de 35 Ko, avec le seuil gardé.
- **Au repos** : une animation nouvelle, `repos`, dans les cinq thèmes. C'est un souffle de glyphes et une phrase dans la voix du thème :

  | Thème | Phrase |
  |---|---|
  | daemon | « LE CERCLE T'ÉCOUTE… » |
  | alien | « À TOI DE TRANSMETTRE… » |
  | cyberpunk | « YOUR MOVE, CHOOM… » |
  | cyborg | « EN VEILLE : À TOI… » |
  | forêt | « LA CLAIRIÈRE T'ÉCOUTE… » |

  Les cadres ASCII gagnent `À`, `É` et `È` là où ils manquaient. `scripts/themes_palettes.py --check` reste vert.
- **Fini sans avoir répondu** depuis la dernière demande : « il a fini sans répondre : redemande-lui, ou Ctrl-O pour voir ce qu'il a fait ».
- **En erreur** : « l'équipier s'est arrêté sur une erreur : Ctrl-O ou /journal pour voir ».

« attend », « fini : fait sans plan », « écrit » et « attend la boîte » ne s'affichent plus.

## La ligne du haut : aucun verdict de l'hôte

- Les mots de l'hôte (« tâche ouverte… ») et les étapes faites ne défilent plus.
- Les appels de plomberie (`step_done`, `report`, `edit_plan`) ne défilent pas.
- Un appel réussi garde sa ligne, sans « fait (3 ms) ». Un appel refusé le dit.
- Tout reste à Ctrl-O et au journal, et `--etapes` remontre tout.

## La cause à côté de l'animation

- « le modèle répond… (gemini-3.8-flash, 12 s) » ;
- « run_command cargo check travaille… (40 s) ».

Le style propre à chaque thème et les attentes que seul l'hôte connaît (quota, index) sont au ticket `docs/tty/tickets/2026-11-15-la-cause-d-une-attente-dans-le-style-du-theme.md`.

## Les témoins

Chacun a été vu rouge avant son code :
- `the_status_line_never_shows_a_bare_state_word` ;
- `a_run_that_starts_plays_the_thinking_animation_until_its_first_model_answer` ;
- `what_is_awaited_is_named_beside_the_animation`.

Sept anciens tests décrivaient les mots d'état ou les verdicts de l'hôte. Ils décrivent maintenant la règle, sans perdre ce qu'ils vérifiaient. L'un exige en plus qu'aucune image ne se redessine pendant que le texte coule.

Le harnais attend, après `o`, l'invite de repos ou la phrase de fin sans réponse, au lieu du mot « fini ». Son `themes.json` est refait depuis les thèmes de la branche.

Le crate passe 105 tests, clippy `-D warnings` ne signale rien, et le harnais entier est vert sous tmux local ouvert.
