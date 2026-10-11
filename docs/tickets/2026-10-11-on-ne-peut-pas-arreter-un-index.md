# On ne peut pas arrêter un index : aucun outil, aucune route de l'hôte

- **État** : ouvert (11 octobre 2026) — trouvé par le premier index réel de rag3db par l'hôte (session codeparsers)
- **Gravité** : blocage (un index qui dérape ne s'arrête qu'en arrêtant l'hôte entier, et avec lui les autres backends, dont la mémoire longue)
- **Atteignable en service** : oui, dès qu'un index est lancé par l'outil `index` sur un hôte partagé
- **Touche rag3weaver** : oui (gabarit `code`, `daemon_host.rs`, `dataflow/index_nodes.rs`)

## Ce que c'est

L'outil `index` lance l'indexation en fond. Rien ne permet de l'arrêter : le gabarit `code`
n'a pas d'outil d'arrêt, et l'hôte n'a pas de route qui ferme un backend.
`stop_indexings_of` (`src/dataflow/index_nodes.rs:267`) existe et sait arrêter l'indexation au
prochain paquet validé, mais il n'est appelé que par le `shutdown` d'un backend
(`src/backend.rs:2945`) et par la réouverture forcée (`src/daemon_host.rs:153`).

Le 11 octobre, pendant le premier index de rag3db (135 592 vecteurs), l'orchestration a fixé
un seuil de 30 Go de mémoire résidente au-delà duquel il fallait arrêter. La seule voie propre
était `kill -TERM` de l'hôte, qui tient aussi la mémoire longue de Lucie.

## Recette

Lancer `index {confirm: true}` sur un gros dépôt par le client MCP, puis chercher un outil ou
une route qui l'arrête : il n'y en a pas.

## Témoin

Aucun. Il faudrait un e2e qui lance un index, appelle l'arrêt, et vérifie que le fil
d'indexation s'est arrêté, que les autres backends de l'hôte répondent toujours, et que
`index` relancé reprend là.

## Ce qu'il faut pour le fermer

Un outil `index_stop` dans le gabarit `code`, ou une route de l'hôte qui arrête l'indexation
d'un backend sans fermer les autres. Il appelle `stop_indexings_of` sur le catalogue de ce
backend et rend le nombre d'indexations arrêtées. La relance reprend déjà là
(marche 0 de la forme C).
