# Un message envoyé pendant un tour du chat était refusé, et se perdait

- **État** : fermé par ce lot (28 novembre 2026), branche `chat-note-mid-turn`
- **Famille** : produit
- **Gravité** : haute : un message de l'utilisatrice disparaît sans être lu
- **Atteignable en service** : oui (envoyer dans le chat d'un dépôt pendant qu'un tour tourne, ou juste après la décision d'une carte d'accord, qui relance un tour)
- **Touche rag3weaver** : oui (`src/daemon_host/chat.rs`, `ui/ide/src/views/Chat.tsx`)
- **Pour** : recherche
- **Signalé par** : un témoin navigateur instable (la page envoie juste après la décision d'une carte d'accord, l'hôte a déjà relancé un tour, le message disparaît) ; décision de l'utilisatrice : « un message envoyé pendant un tour ne se perd jamais ».

## Le pas exact

`DaemonHost::chat_turn_with` refusait tout tour quand un autre tournait sur le même backend : « un tour est déjà en
cours sur ce backend (run …) : attendez sa fin, ou arrêtez-le ». L'IDE grisait le bouton d'envoi pendant un tour ; une
page qui envoyait quand même (un tour relancé par l'hôte, que la page ne savait pas encore) recevait le refus, et le
message ne vivait plus nulle part.

## Le correctif

Un envoi par `/ide/chat` (`chat_turn_by` : la page, le terminal, un agent) pendant un tour du chat devient une **note
du tour** : il entre dans la file `ChatEntry::notes`, le flux de l'IDE le publie (`NoteReceived` sur `chat`), et
l'appelant reçoit `{"received_during_turn": true, "run": …}`. La boîte à outils du tour (`NotesInTheTurn`, autour de
celle du chat, comme `Step::deliver` pour la boîte d'un équipier) livre les notes en tête du résultat d'outil suivant,
par les textes `mid_turn.head` et `mid_turn.line`, et les garde dans `metadata.received_during_turn` du résultat : le
fil les montre là où le modèle les a lues. Les notes qui restent à la fin du tour partent comme message du tour
suivant, marqué `received_during_turn: true`. Une note sort de la file quand elle est livrée : jamais deux fois.

Ce qui garde son refus : le pas d'un équipier (sa boîte lui livre déjà les messages) et un tour relancé par l'hôte
lui-même (`chat_turn` : la relance d'une carte d'accord attend la fin du tour, comme avant).

## Les témoins

`src/daemon_host/chat_note_mid_turn_tests.rs` : la note entre dans le tour et le modèle la cite ; une note laissée
à la fin d'un tour lance le tour suivant ; dans l'IDE, le bouton reste actif, la phrase s'affiche « reçu pendant le
tour » et le modèle la cite.
