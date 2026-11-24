# Le coupable d'un train de deux est repris à la même tête au lieu d'être rendu

- **État** : ouvert (24 novembre 2026, écrit par G)
- **Pour** : G (optimiseur)
- **Famille** : produit (l'outillage des passes : le pousseur)
- **Gravité** : moyenne. Une passe entière (environ 15 min) dépensée sur un lot dont on sait déjà qu'il est rouge, et
  son auteur prévenu un pas trop tard.
- **Atteignable en service** : non, le pousseur seulement
- **Touche rag3weaver** : non (`tools/poste/pusher.sh`)

## Ce que c'est

Le 24 novembre 2026 à 04 h 37, le train `recherche-chat-input-before-ready+1` (deux lots : `recherche-chat-input-before-ready`
puis `ed-lot-judge`) rougit sur la lib (`chaque_gabarit_livre_se_charge`). La bissection rejoue le premier seul : vert, poussé à
04 h 52. Le second, seule cause possible, reste en tête de file sans être rendu, et le pousseur le reprend à 04 h 52 min
58 s, à la même tête, en train de deux avec `memoire-own-turns` : une passe promise au rouge, et le lot de mémoire
retenu derrière. ED l'a retiré à la main à 04 h 53.

La règle de la bissection (un préfixe vert poussé, le reste en tête de file) est juste quand le reste compte plusieurs
lots ; quand il n'en compte qu'un, ce lot est le coupable par déduction.

## Ce qu'il faut

1. Quand le train a rougi en code (pas en environnement), que le préfixe vert est poussé et qu'il ne reste qu'un lot, ce
   lot est rendu rouge à son auteur, avec le rouge du train et la déduction dite (« le premier rejoué seul est vert »).
2. Le témoin du pousseur (cas 50 b) : après le pas, le premier poussé, le second rendu rouge, sans second rejeu.
