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
