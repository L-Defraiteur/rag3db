# Un créneau 2 sans avenir court jusqu'au bout, et le pas du pousseur l'attend

- **État** : ouvert (22 novembre 2026, écrit par G)
- **Pour** : G (optimiseur)
- **Famille** : produit (l'outillage des passes : le pousseur)
- **Gravité** : moyenne. Une passe entière (jusqu'à 60 min) pour rien, qui prend une place et une lib de tests, et la
  file qui attend derrière elle.
- **Atteignable en service** : non, le pousseur seulement
- **Touche rag3weaver** : non (`tools/poste/pusher.sh`)

## Ce que c'est

Le 22 novembre 2026, à 02 h 36, `user-token-2` (train A) est rouge ; `tty-card-2`, son enfant, rejouait en créneau 2
depuis 01 h 58 sur la tête de A. Son lot ne pouvait plus atterrir (« A n'a pas atterri entier »), mais sa passe a
continué : le pas du pousseur attend la fin du créneau 2 (`wait` de `spec_run`) avant de conclure. G l'a arrêtée par ses
pids à 02 h 37 ; la file a repris.

## Ce qu'il faut

Dès que A est rendu (rouge, conflit, retiré) ou qu'il n'atterrit pas entier, le pas arrête la passe du créneau 2 (son
groupe de processus, par ses pids) au lieu de l'attendre, et le dit (« créneau 2 arrêté : A n'atterrit pas »). Témoin :
A rouge pendant une passe B qui dort ; le pas finit sans attendre la fin de B.

## D'ici là

Le pousseur tourne avec un seul créneau la nuit du 21 au 22 (drop-in `un-seul-creneau-nuit.conf`).
