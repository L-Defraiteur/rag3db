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

## Le soir du 15 novembre : prête, sans fenêtre

La mesure est prête à luciepc : `~/.cache/rag3weaver-build/groupe-repli-jeu/nuit.sh`, sous `poste lourd`. Elle n'a qu'une
garde, avant tout bâti puis avant la mesure : card2 sous 13 548 Mio libres (lus), un `va-du-banc` de moins de deux
heures, et le 7879 sur le binaire neuf (`max_chars_per_text` dans `/sante`). L'hôte est bâti d'avance, sans GPU
(`BATI_SEUL=1`). Le guetteur `guetter.sh` lit card2 toutes les 15 s et la lance une fois. Sa marge est celle par
défaut, 2048.

Ce soir, au mot de c7, elle devait partir pendant la répétition d'arbre. card2 n'est jamais passée sous le seuil :
16 966 Mio libres à 21 h 41, 16 901 à 22 h 06 (après les vecteurs de code-principal), 16 773 à 22 h 28 (après ceux de
code-lourd). Les vecteurs d'arbre passent par le 7878 (granite), qui ne remplit pas la carte. À 20 h 03, elle était à
11 844 Mio libres, sous la parité du banc et une étape plus ancienne de la répétition. Le cas réel demande donc une
passe qui occupe card2 **en mémoire** (un modèle résident en plus, un gros travail GPU), avec le « va » de son
propriétaire.
