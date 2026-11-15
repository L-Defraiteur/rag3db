# Mesurer le repli sur une carte réellement pleine

- **État** : ouvert (15 novembre 2026, embarquement ; décision de c7)
- **Pour** : embarquement
- **Famille** : produit (une mesure à jouer, pas de code)
- **Gravité** : manque de preuve. Le mécanisme est prouvé, le cas réel ne l'est pas.
- **Atteignable en service** : oui
- **Touche rag3weaver** : non (une mesure)

## Ce que c'est

Le vrai jeu du 15 novembre à 15 h 03, sur card2, a joué le mécanisme du repli quand la carte est pleine
(`docs/embarquement/13-11-2026/01-activation-et-residence-livre-et-manquant.md`, « Le vrai jeu sur card2 »).
Il a rempli tout ce qu'on attendait de lui : le repli est dit, les 2 284 morceaux passent par le 7879, l'index
dure 34,1 s, et rien ne se lance sur card2. Mais card2 avait alors 15 190 Mio libres, et c'est la marge montée à
4096 qui a forcé le repli. Le cas d'une carte réellement pleine n'a donc pas été joué.

## Ce qu'il faudrait

Une nuit où card2 est réellement prise, avec moins de 13 548 Mio libres, rejouer le même jeu avec la marge par
défaut (2048), au créneau du banc :
- `groupe-repli-jeu/mesure.sh` à luciepc, après avoir rendu la marge du manifeste à 2048 ;
- attendu : rien de lancé sur card2, le repli dit au journal avec la VRAM réelle, tous les morceaux servis par
  le 7879, et un index proche des 34 s, jamais des 904 s.
