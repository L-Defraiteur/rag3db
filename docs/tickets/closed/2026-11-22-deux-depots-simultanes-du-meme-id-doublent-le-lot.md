# Deux dépôts simultanés du même id doublent le lot dans la file

- **État** : corrigé (22 novembre 2026, lot pusher-enqueue-duplicate-race : l'id est vérifié de nouveau sous le verrou de la
  file, avec l'écriture ; témoin rouge d'abord, test_pusher.sh cas 49, six dépôts simultanés pour une seule ligne).
  Écrit par G.
- **Pour** : G (optimiseur)
- **Famille** : produit (l'outillage des passes : le pousseur)
- **Gravité** : basse. Un lot doublé est pris deux fois, ou son verdict ne touche qu'une des deux lignes.
- **Atteignable en service** : non, le pousseur seulement
- **Touche rag3weaver** : non (`tools/poste/pusher.sh`)

## Ce que c'est

Le 22 novembre 2026, à 01 h 32, après le rouge de `user-token-2`, la session tty l'a redéposé à 01:32:06 et G à 01:32:18,
de même pour `tty-card-2`. La file a tenu chaque lot deux fois : `enqueue` vérifiait « l'id est-il déjà en file ? » avant
de prendre le verrou de la file, et les deux dépôts passaient la vérification avant que l'un écrive. G a retiré les
secondes lignes sous le verrou, à la main.

## Le correctif

Dans le bloc qui tient le verrou de la file, `enqueue` vérifie de nouveau l'id avant d'écrire ; s'il y est, le dépôt est
refusé (« déposé en même temps ») et rien n'est écrit ni journalisé.
