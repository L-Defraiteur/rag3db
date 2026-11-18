# tty-23 : les messages entre sessions dans le terminal (rendu, sur le faux hôte)

Session tty, 18 novembre 2026. Ticket `docs/tty/tickets/2026-11-16-les-messages-entre-sessions-dans-le-terminal.md`. Le rendu est fait sur la forme de ligne proposée à archi. Le lot ne va pas sur master avant qu'archi ait confirmé cette forme (consigne de c7).

## Ce que montre le terminal

Une ligne de la boîte `from: "session"`, avec `peer`, `direction` (`in` ou `out`), `kind: "message"` et `text` :
- **reçue** : « › Message de @<peer> : <début>… (Ctrl-O pour tout lire) », sur une ligne, quand le message dépasse sa première ligne ou 80 caractères ; entière sinon. La ligne discrète dit « message de @<peer> » ;
- **émise** : « “<début>…” → envoyé à <peer> » ; la ligne discrète dit « envoyé à <peer> » ;
- **l'accusé** (`kind: "delivery"`, `reply_to` l'envoi, `delivery` et `delivery_reason`) : « ⎿ refusé : <raison> », « ⎿ en file » ou « ⎿ remis » ; un `delivered` porté par l'envoi lui-même ne dit rien de plus ;
- **Ctrl-O** ouvre le message entier dans la visionneuse. Ses entrées sont désormais les appels d'outil et les messages, dans l'ordre d'arrivée ; l'en-tête dit « message » ou « appel d'outil » ;
- **en `--plain`**, le message s'écrit entier, avec son interlocuteur.

## Les témoins

- `vue::tests::a_received_session_message_is_folded_with_a_hint_to_open_it`, `a_sent_session_message_says_where_it_went_and_its_delivery`, `ctrl_o_opens_the_whole_message_among_the_calls` : vus rouges (0f0abad4e2), puis verts.
- `plain::tests::a_session_message_says_its_peer_in_plain_text` : vu rouge, puis vert.
- Le harnais : la séance 1h, sur la route neuve `POST /faux/session` du faux hôte. Ses cinq points (reçu, replié, émis, refus dit, ouvert par Ctrl-O) sont rouges contre le binaire de master, puis verts ; le harnais entier est VERT sous `tmux-local-ouvert`. Sous tmux, l'ouverture par Ctrl-O se juge sur l'écran que tmux montre (`capture-pane`), puisque tmux redessine lui-même ce qui change.
- Le crate passe 118 + 1 tests, et 23 sans vue ; clippy `-D warnings` ne signale rien.

## Ce qui attend

- La page de relevé du bus par archi, et la forme qu'elle fixe. Si elle diffère, `Vue::session_line`, `plain::ligne_de_boite` et la route du faux hôte s'y calent.
- Une demande reçue qui demande un accord passe par la porte, avec les mêmes boutons : elle viendra comme les autres lignes `approval`, avec le `peer` d'origine, quand le mode pair existera.
