# Le tour lancé par des notes laissées ne reprend pas l'attribution des dépenses de l'envoyeur

- **État** : ouvert, le 28 novembre 2026
- **Gravité** : comptage faux : les appels au modèle de ce tour ne sont pas comptés comme ceux de la personne qui a écrit
- **Atteignable en service** : oui (une phrase envoyée pendant un tour qui finit sans autre appel d'outil)
- **Touche rag3weaver** : oui (`src/daemon_host/chat.rs`, `start_the_turn_of_left_notes`)
- **Famille** : produit
- **Pour** : recherche
- **Signalé par** : recherche, à la fin du lot `chat-note-mid-turn`

## Le pas exact

Une note reçue pendant un tour et pas livrée avant sa fin part comme message du tour suivant :
`start_the_turn_of_left_notes` lance `chat_turn_from(…, Sender::LeftNotes)` dans un fil neuf. L'attribution que
`chat_turn_by` pose (`crate::usage_ledger::attribute`, avec `human` et `client`) ne vaut que pour le fil de l'appel
d'origine : le tour des notes tourne sans elle. Il n'est donc pas compté comme le tour d'un humain, et, passé le
plafond du mois, il peut être refusé alors que la personne l'aurait passé.

## Pour le fermer

Garder dans `TurnNote` le `by` de l'envoyeur (ce que `by_of` a signé), et rouvrir l'attribution dans le fil du tour
suivant avant `chat_turn_from`. Un témoin : une note laissée par un `by` humain, au-delà du plafond, lance quand même
son tour, et le registre des dépenses le compte comme humain.
