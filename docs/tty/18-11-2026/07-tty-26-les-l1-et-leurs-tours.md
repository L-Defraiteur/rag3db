# tty-26 : les L1 du run et leurs tours, dans le terminal (sur le faux hôte)

Session tty, 18 novembre 2026. La chaîne du L1 jusqu'au tour, côté terminal : `docs/memoire/18-11-2026/02-l-archiviste-en-deux-etapes.md`, « La chaîne, du L1 jusqu'au tour », point 4 (« un L1 montre sa plage, et un geste ouvre les tours par `read_turns`, avec leur état »). Le lot est en branche (`cx-tty-26`, sur `cx-tty-23`) jusqu'à ce que l'hôte expose les routes ; c'est le lot d'archi.

## La forme, accordée le 18 nov.

- `GET /ide/l1?backend=<id>&run=<run>` → `{l1: [{apport, fiche, section, run, from_turn, to_turn}]}`, dans l'ordre des tours. Le run de l'équipier vaut `<tâche>:<parent>`, celui du chat `chat:<backend>` (archi).
- `POST /ide/turns/read {apport}` (ou `{run, from, to}`) → `{turns: [{turn, state, messages}]}`, avec les états `indexed`, `from_chat_file` et `gone`, dits tels quels.

Archi a pris ces noms ; il donnera le commit de ses routes. Si un nom change, une ligne suffit à le recaler (`src/tui.rs`).

## Ce que fait le terminal

- `/memory` liste les L1 du run de la tâche : « les L1 du run <run> (/turns <n> ouvre ses tours) : », puis, numérotées, des lignes « 1. fiche:<fiche> — <titre> · tours 12 à 15 » (« tour 16 » pour un tour seul).
- `/turns <n>` ouvre les tours du n-ième L1 dans la visionneuse de Ctrl-O, sous l'en-tête « L1 <n> · tours 12 à 15 » :
  - chaque tour est dit avec son état : « tour 12 · indexed » ;
  - ses messages suivent : « [user] … », « ⎿ run_command (ok) … » ;
  - un tour parti le dit : « tour 14 · gone : ni dans l'index, ni dans le fichier du chat ».
- La visionneuse gagne une troisième sorte d'entrée, le document, après les appels et les messages de tty-23.
- Le code neuf est en anglais (`src/memory.rs`, `open_document`, `/memory`, `/turns`) ; l'affichage reste en français.

## Les témoins

- `memory::tests::memory_lists_the_l1_of_the_run_with_their_range_and_fiche` et `the_turns_of_an_l1_say_their_state_and_a_gone_turn_is_said`, et la lecture de `/memory` et `/turns` par les commandes : vus rouges sur leurs coquilles vides, en local.
- `vue::tests::a_document_opens_in_the_viewer_like_a_call` : vu rouge par mutation (`open_document` vidé, 0 entrée au lieu d'une).
- Ce lot n'a pas commité son rouge avant le code : un écart à la règle, dit ici.
- Le harnais : la séance 1i, sur les routes `GET /ide/l1` et `POST /ide/turns/read` du faux hôte. Elle est rouge contre le binaire de master, puis verte ; le harnais entier est VERT sous `tmux-local-ouvert`.
- Le crate passe 121 + 1 tests, et 23 sans vue ; clippy `-D warnings` ne signale rien.
