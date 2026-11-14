# Un tenant de poste qui reçoit un signal ne garde pas le verrou jusqu'à la fin de sa commande

- **État** : ouvert (14 novembre 2026), relevé par G en jouant `test_poste.sh` pour l'enveloppe CPU.
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

Pas encore lue. Piste : le `trap … TERM` de `poste` sort par `exit 143` alors que la commande tourne encore,
ce qui rend les descripteurs 8 et 9 ; à départager entre le `trap` et la manière dont le témoin envoie le signal.

## Pour le fermer

Le cas 6c vert, sans relâcher les autres cas du témoin.
