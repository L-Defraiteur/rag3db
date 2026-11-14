# Un tenant de poste qui reçoit un signal ne garde pas le verrou jusqu'à la fin de sa commande

- **État** : corrigé (14 novembre 2026, branche tenant-signal) : le verrou de `poste push` est protégé dès qu'il est pris.
- **Famille** : produit
- **Gravité** : blocage ; une mesure peut entrer pendant que la commande d'un tenant interrompu tourne encore.
- **Atteignable en service** : oui, en envoyant TERM à un `poste mesure|lourd` qui tient la place.
- **Touche rag3weaver** : oui, `extension/rag3weaver/scripts/poste`.
- **Pour** : optimiseur (G).
- **Ouvert le** : 14 novembre 2026

## Ce que c'est

Le cas 6c de `extension/rag3weaver/scripts/test_poste.sh` (« un tenant qui reçoit un signal garde le verrou
jusqu'à la fin de sa commande ») est **rouge sur master** (fb8b8eb93), et aussi sur la branche de l'enveloppe CPU :
le défaut préexiste à ce lot.

## La recette

`bash extension/rag3weaver/scripts/test_poste.sh` rend `ÉCHEC  un tenant qui reçoit un signal garde le verrou
jusqu'à la fin de sa commande`. Le témoin joue sur ses propres verrous (`POSTE_LOCK_DIR`), jamais sur les réels.

## Le témoin

Le cas 6c lui-même.

## La cause

Lue le 14 novembre. Le cas concerne `poste push`, pas `poste lourd`. Le script prend le verrou, puis lit l'état
de la CI de master (`etat_de_la_ci`, jusqu'à 20 s de réseau, ajoutée le 18 octobre en b0c0de81e), et **n'installe
son `trap` qu'après**, une fois la commande lancée. Un TERM reçu pendant la lecture de la CI tuait donc le script :
le verrou était rendu, la commande jamais lancée, sans un mot. Le témoin envoie son signal à 1 s : vert avec
`POSTE_CI=0`, rouge avec la lecture de la CI.

## Pour le fermer

Le cas 6c vert, sans relâcher les autres cas du témoin.

## Le correctif

Le `trap TERM INT HUP` est posé **dès la prise du verrou**, avant `etat_de_la_ci`. Un signal reçu avant la
commande est dit, et la commande court ; celui reçu pendant la commande garde l'ancien message. Le cas 6c est
vert avec la lecture de la CI : 29 OK et 0 ÉCHEC à `test_poste.sh`, contre ÉCHEC sur master.
