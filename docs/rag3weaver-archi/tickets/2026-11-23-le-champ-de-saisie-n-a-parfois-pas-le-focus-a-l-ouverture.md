# Le champ de saisie n'a parfois pas le focus à l'ouverture d'un dossier

- **État** : ouvert, le 23 novembre 2026
- **Famille** : produit
- **Gravité** : basse : un témoin navigateur qui rougit parfois ; à l'ouverture d'un dossier, le focus reste sur la page au lieu du champ de saisie, et l'utilisatrice doit cliquer avant d'écrire
- **Atteignable en service** : à voir (le même ordre d'événements qu'au témoin)
- **Touche rag3weaver** : oui (`src/daemon_host/open_folder_chat_tests.rs`, la page du chat de l'IDE)
- **Pour** : recherche (l'IDE)
- **Signalé par** : archi, passe du lot `voice` à luciepc, le 23 novembre 2026

## Ce qui s'est passé

`daemon_host::open_folder_chat_tests::opening_a_folder_lands_on_its_chat_with_a_welcome` a rougi une fois, à
`src/daemon_host/open_folder_chat_tests.rs:40` :

> the agent is ready: the input has the focus — left: "BODY", right: "TEXTAREA"

Passe sur la tête 974dc04698 (lot `voice`, qui ne change que des textes d'invite de l'agent) : 72 témoins navigateur
verts, celui-là rouge.

## Les rejeux

La série entière des 73 témoins navigateur (`scripts/temoins_navigateur.sh`), à luciepc, juste après :

| tête | essai 1 | essai 2 |
|---|---|---|
| 974dc04698 | 73/73 | 73/73 |
| master 643fca10e8 | 73/73 | 73/73 |

Aucun rouge reproduit en quatre passages.

## À faire

Lire si le focus est donné avant que le champ existe (rendu différé) ou repris par un autre élément ; et si le témoin
attend l'événement (le champ posé et focalisé) plutôt qu'un instant.

## Une cause lue, écartée par la mesure (23 novembre 2026, soir)

La cause lue dans `ui/ide/src/views/Chat.tsx` (l'effet `if (chat?.on) input.current?.focus()` sur `[chat?.on]` ne se
rejoue pas quand le chat est prêt avant que le champ soit monté) a été forcée par un témoin,
`the_input_takes_the_focus_when_the_chat_is_ready_before_it_is_mounted` (même fichier). Le premier `/ide/etat` y est
rendu sans le dépôt, et le second arrive une seconde plus tard. Le témoin relève les instants dans la page :

| essai | chat prêt (ms) | champ inséré (ms) | désactivé à l'insertion | focus |
|---|---|---|---|---|
| 1 | 90 | 1 096 | non | `#message` |
| 2 | 130 | 1 096 | non | `#message` |
| 3 | 88 | 1 088 | non | `#message` |

Un champ inséré pendant que le chat est prêt n'est pas désactivé, et `autoFocus` le focalise. Cet ordre ne perd donc pas
le focus. La cause est ailleurs : un remontage désactivé (`chat.on` qui repasse à faux pendant une relecture), ou un
autre élément qui prend le focus. Les deux témoins du focus disent maintenant, quand ils rougissent, les requêtes avec
leurs instants, l'élément focalisé avec son chemin et l'état du champ. Le prochain rouge départagera.
