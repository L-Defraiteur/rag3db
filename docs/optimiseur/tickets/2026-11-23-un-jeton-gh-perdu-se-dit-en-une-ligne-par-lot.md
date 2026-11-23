# Un jeton gh perdu se dit en une ligne par lot, et trente lots atterrissent sans PR

- **État** : ouvert (23 novembre 2026, écrit par G)
- **Pour** : G (optimiseur)
- **Famille** : produit (l'outillage des passes : le pousseur)
- **Gravité** : moyenne. Rien n'est perdu sur master, mais la traçabilité par PR s'arrête sans que personne le voie,
  et le rattrapage grandit à chaque lot.
- **Atteignable en service** : non, le pousseur seulement
- **Touche rag3weaver** : non (`tools/poste/pusher.sh`)

## Ce que c'est

Le 23 novembre 2026, à partir de 18 h 55 (`raw-call-trace`), chaque lot poussé porte au journal du pousseur la ligne
« PR absente : pas de jeton gh de L-Defraiteur ». À 23 h 3x, trente lots avaient atterri sans PR. `gh auth status` à
luciepc dit « The token in default is invalid » pour les deux comptes. Les pushs sur master passent toujours (git par
ssh) : seule l'ouverture des PR, et leur fermeture, dépendent de `gh`.

Le pousseur fait ce qui était voulu (« une PR qui ne peut s'ouvrir ne bloque jamais le push »), mais il le dit comme
un incident de lot, une ligne parmi d'autres, alors que c'est un état du poste : tant que le jeton n'est pas revenu,
chaque lot le répétera. Personne ne l'a vu pendant quatre heures et demie.

## Ce qu'il faut

1. **Une alerte au premier « pas de jeton »** : une ligne `ALERTE` au journal, une fois par perte (pas par lot), et un
   mot dans la boîte de l'orchestrateur (`inbox/c7.txt`), comme le guet du créneau unique ; puis une ligne de retour
   quand le jeton revient.
2. **Le rattrapage proposé, pas oublié** : à la première PR ouverte après une perte, le pousseur liste les lots « PR
   absente » de la période et la commande `pusher.sh pr-catchup <id> <master avant> <tête> <auteur> <titre>` de
   chacun (le master d'avant et la tête se lisent dans les lignes « poussé » du journal).
3. **Un témoin, rouge d'abord** : un faux `gh` (`PUSHER_GH`) qui refuse le jeton ; deux lots poussés : une seule
   alerte, deux lignes « PR absente » ; le faux `gh` rendu valide : la ligne de retour et la liste de rattrapage.

## Ce qui ne dépend pas de nous

La reconnexion (`gh auth login` à luciepc) est le geste de Lucie : aucun agent ne touche à ses identifiants.
