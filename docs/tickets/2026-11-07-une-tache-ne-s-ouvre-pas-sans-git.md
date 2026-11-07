# Une tâche ne s'ouvre pas sans git

- **État** : ouvert (7 novembre 2026), demandé par Lucie par c7 : « il doit pouvoir aussi
  démarrer un truc même sans git du tout ».
- **Famille** : produit
- **Gravité** : fonction absente. Quand ni le manifeste ni sa `workspace.root` ne sont dans un
  dépôt git, l'équipier ne peut ouvrir aucune tâche.
- **Atteignable en service** : oui, toute racine hors git.
- **Touche rag3weaver** : oui (`src/daemon_host/taches.rs` : ouverture, relecture, fusion, rejet ;
  la boîte de l'équipier).

## Ce qu'il faut pour le fermer

- La racine de la tâche est `workspace.root` résolue, ou à défaut le dossier du manifeste.
- Si cette racine est dans un dépôt git, la tâche est un worktree sur une branche (le cas actuel).
- Sinon, la tâche s'ouvre quand même. Son espace est un dossier de travail à elle, sous le dossier
  des tâches, copié par la même porte que la copie de base (reflink si possible) et compté dans la
  taille dite. Elle n'a pas de branche (`repo: null`, `branch: null`), et la boîte le dit : « sans
  git : les changements se relisent par le diff du dossier ».
- La relecture (le diff ligne à ligne de TaskReview), la fusion et le rejet ont leur chemin sans
  git : un diff de dossiers, une recopie des fichiers changés, un effacement du dossier.
- Une option `git_init: true` ou un outil initialise un dépôt dans cet espace si l'utilisatrice
  le veut, jamais d'office.

Témoin par l'hôte : une racine sans git ; la tâche s'ouvre, l'équipier lit et écrit, et la
proposition arrive.
