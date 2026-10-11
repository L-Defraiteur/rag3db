# Un backend de code dont la racine a disparu rend une erreur brute et se dit ouvert

- **État** : ouvert (11 octobre 2026, 19 h 20 ; relevé par le cœur C++ à la demande de Lucie, sur
  le serveur MCP `code-rag3db-luciepc`).
- **Gravité** : réponse fausse / confort — l'agent reçoit une erreur d'entrée-sortie brute ou un
  refus « catalogue occupé », sans savoir que l'arbre interrogé n'existe plus.
- **Atteignable en service** : oui (un worktree effacé après un lot, le manifeste qui le nommait
  encore chargé par le démon).
- **Touche rag3weaver** : oui (démon, outils MCP du code).

## Ce que c'est

Le démon de luciepc tient trois backends, dont `code-bascule`, racine
`/home/lucied/git_workspaces/rag3db-bascule` — un worktree du banc, effacé après sa preuve de la
bascule. Le serveur MCP de Lucie est lié à ce manifeste. Observé à 17 h 55-17 h 58 (heure luciepc) :

1. `search_code` (balayage) : « IO error for operation on /home/lucied/git_workspaces/rag3db-bascule:
   No such file or directory (os error 2) » — brut.
2. `daemon_status` : le backend est `open: true`, rien ne dit que sa racine manque.
3. `usages` : « Catalogue occupé : une indexation le tient en ce moment » par intermittence
   (17:56, 17:58 ; libre à 17:57:25) — probablement le guetteur qui réessaie sur la racine absente,
   ou l'indexation d'un autre backend qui tient un catalogue commun ; dans les deux cas une
   **lecture** est refusée.
4. `search_code` (indexé) sur un jeton absent du corpus : trois chunks sans rapport à ★0,74-0,75.

## Attendu

- `daemon_status` dit « racine absente » pour ce backend, et chaque outil répond nommément
  (« la racine /…/rag3db-bascule n'existe plus : fermer ce backend ou le relier à un arbre »).
- Une lecture (`usages`, `impact`, `search_code` indexé) sert le dernier état indexé pendant une
  indexation, avec « indexation en cours depuis N s ».
- Un jeton inconnu donne un score bas ou « aucune correspondance lexicale ».
- L'essai d'édition à chaud (un fichier déposé, `usages` qui le voit) reste à faire sur un backend
  dont la racine existe (`code-principal`) ; le fichier d'essai `test/essai_indexation_a_chaud.cpp`
  est dans l'arbre principal de luciepc pour cela.

## Témoin

Aucun encore : à écrire côté rag3weaver (un backend ouvert sur un dossier, le dossier retiré, les
trois outils interrogés).
