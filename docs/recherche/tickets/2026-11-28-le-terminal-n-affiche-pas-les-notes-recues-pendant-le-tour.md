# Le terminal n'affiche pas les messages reçus pendant un tour

- **État** : ouvert, le 28 novembre 2026
- **Gravité** : affichage manquant : la phrase est bien reçue par l'hôte, mais le terminal ne la montre pas comme telle
- **Atteignable en service** : oui (un message envoyé au chat d'un dépôt pendant un tour, depuis l'IDE ou un autre client)
- **Touche rag3weaver** : oui (`tty/src/`)
- **Famille** : produit
- **Pour** : tty
- **Signalé par** : recherche, à la fin du lot `chat-note-mid-turn`

## Le pas exact

Depuis le lot `chat-note-mid-turn`, un envoi par `/ide/chat` pendant un tour du chat devient une note du tour.
L'hôte publie `NoteReceived` (`backend`, `turn_run`, `text`, `at_ms`) sur le sujet `chat` du flux. Il garde la note
dans `metadata.received_during_turn` du résultat d'outil qui l'a livrée, ou sur le message du tour suivant quand elle
y est partie. L'IDE affiche la phrase marquée « reçu pendant le tour » ; le client du terminal (`rag3weaver-tty`) ne
lit ni l'événement ni ces métadonnées. La phrase n'apparaît pas pendant le tour, et à la relecture elle n'est pas
distinguée.

## Pour le fermer

Dans la vue du terminal, afficher la ligne de la personne à la réception de `NoteReceived`, avec la mention « reçu
pendant le tour », et lire `received_during_turn` à la relecture d'un fil. Un témoin de la vue : une trame
`NoteReceived` pendant un tour donne la ligne marquée.
