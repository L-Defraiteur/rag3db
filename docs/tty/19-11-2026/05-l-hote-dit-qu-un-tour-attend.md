# L'hôte dit qu'un tour attend : `Waiting` et `Resumed`

Session tty, 19 novembre 2026, au mot de c7, dans la forme qu'a donnée archi. Trois tickets du terminal l'attendaient : la cause d'une attente dans le style du thème, et la ligne vivante qui nomme ce qu'elle attend. Seul l'hôte sait qu'un appel attend.

## Les événements

Deux événements sur le bus du backend, sujet `agent`, sur le run du tour :

- `{kind: "Waiting", run, cause, what, since_ms}`
  - `cause` vaut `tool` ou `model` ;
  - `what` est le nom de l'outil ou du modèle (le même que le `model` de `LlmCall`) ;
  - `since_ms` est le temps déjà attendu.
- Pour le modèle, `wait_ms` : la fenêtre annoncée, pour que l'écran dise « reprise dans N s » (remarque d'archi, rouge e7b5f174b6, puis vert).
- `{kind: "Resumed", run, cause, waited_ms}`, seulement après un `Waiting` du même run.

Ils sont émis une fois par attente, au-delà d'un seuil qui existait déjà, et jamais à chaque tour de boucle :

- **un outil** : `teammate.tool_wait_notice_s` (20 s par défaut), le même minuteur que la ligne « attend depuis N s » de la boîte (`Etanche::par_le_backend`). Le `Resumed` part quand l'appel rend ;
- **le modèle** : le recul d'un refus du fournisseur (`attente_du_fournisseur`), sur le run du pas refusé, dont le pas note le run et le modèle au début de son tour (`Step::note_turn_run`). Le `Resumed` part quand la fenêtre est passée. Un `stop` ou une tâche fermée pendant l'attente n'en émet pas : le run est fini.

La cause `index` n'est pas émise par ce lot. Une attente d'index se voit aujourd'hui comme un outil qui attend, et la ligne de la boîte le suppose (« un index tient sans doute l'écriture »).

## La portée (relecture d'archi)

- **Les pas d'un équipier, puis le chat.** Un outil lent dans un tour du chat de l'IDE ne disait rien. C'est fait dans le lot qui suit (`cx-chat-waiting-2`) : le minuteur est partagé (`say_the_wait`), et les outils du chat sont enveloppés (`DitSonAttente`). Ticket fermé : [`docs/tty/tickets/closed/2026-11-19-un-outil-lent-du-chat-de-l-ide-ne-dit-pas-qu-il-attend.md`](../tickets/closed/2026-11-19-un-outil-lent-du-chat-de-l-ide-ne-dit-pas-qu-il-attend.md).
- **Le `Waiting` du modèle part sur le run du pas refusé, déjà fini.** Le terminal, qui suit la tâche, le voit : c'est l'usage visé. Une page qui ne suit que le run du tour en cours (`runRef` dans `Chat.tsx`) ne le verra pas.

## Les témoins

- `daemon_host::teammate_tests::a_waiting_tool_says_so_on_the_bus_once_and_says_when_it_resumes` : un seuil d'1 s, et un `read_file` retenu 2,5 s par le slot tenu du backend de la tâche. Un seul `Waiting` (`tool`, `read_file`, au moins 1 000 ms) sur le run du premier pas, puis un seul `Resumed` (au moins 2 000 ms). Vu rouge (74cf72efe4, puis 9e98d27dfe une fois le seuil commité dans les déclarations, rouge revu sans le code), puis vert.
- `a_provider_refusal_waits_a_window_and_does_not_fail_the_step`, étendu : trois 429 donnent trois `Waiting` (`model`, le nom du modèle), chacun suivi de son `Resumed`, le dernier après au moins 4 s. Vu rouge (9f9cc4d702), puis vert.
- Les modules `daemon_host::teammate` (44), `daemon_host::chat` (21) et `events::` (8) restent verts.

Les deux passes ont été jouées sous la lib épinglée 3d706f709 avec `RAG3WEAVER_MOTEUR_ANCIEN=1`, en le sachant : master a, depuis ce matin, du C++ vectoriel plus neuf que la lib, et le lot ne touche que l'hôte.

## La suite, côté terminal

La seconde moitié de tty-25 : `cause_spans` lit `Waiting` pour nommer l'attente (« read_file attend depuis 20 s… », « le modèle attend le fournisseur… ») et `Resumed` pour la lever.
