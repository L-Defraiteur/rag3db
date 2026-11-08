# Un backend de code dont la racine a disparu rend une erreur brute et se dit ouvert

- **État** : corrigé (8 novembre 2026, essai 4 de l'équipier Gemini sous supervision c7) — `daemon_status` porte « racine absente : <chemin> » quand `workspace.root` n'existe plus sur le disque, et chaque outil dont la politique touche la racine (`read_files`, `write_files`, `run_commands`) rend nommément « la racine du dépôt a disparu (<chemin>) ».
- **Gravité** : réponse fausse / confort — l'agent reçoit une erreur d'entrée-sortie brute ou un
  refus « catalogue occupé », sans savoir que l'arbre interrogé n'existe plus.
- **Atteignable en service** : oui (un worktree effacé après un lot, le manifeste qui le nommait
  encore chargé par le démon).
- **Touche rag3weaver** : oui (démon, outils MCP du code).
- **Famille** : produit

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

## La cause

À l'ouverture du backend, la présence de `workspace.root` est vérifiée au chargement (`PreparedBackend::load_avec`). Mais une fois le backend ouvert, si le dossier racine disparaît du disque (worktree supprimé après une tâche, disque démonté) :
1. `DaemonHost::state()` ne vérifiait pas la présence physique de la racine et indiquait `open: true` sans mentionner la disparition du dossier.
2. Les outils manipulant les fichiers du workspace tentaient l'opération d'entrée-sortie et rendaient une erreur brute du système (`No such file or directory`).

## Le témoin

`daemon_host::tests::un_backend_de_code_dont_la_racine_a_disparu_est_signale_dans_daemon_status_et_ses_outils` : ouvre un backend de code sur un dossier temporaire, vérifie que `list_files` fonctionne, supprime le dossier source, puis vérifie que `daemon_status` porte « racine absente : <chemin> » et que `list_files` rend l'erreur nommée « la racine du dépôt a disparu (<chemin>) ».

Rouge avant le correctif :

```
test daemon_host::tests::un_backend_de_code_dont_la_racine_a_disparu_est_signale_dans_daemon_status_et_ses_outils ... FAILED
panicked at src/daemon_host.rs:1982:9:
daemon_status doit porter « racine absente : /tmp/.tmpAhTsfV/sources » : {"available_repos":[],"backends":[{"branch":null,"closed":0,"id":"21d37672c08a","index":{"File":{"relations":"never","sparse":null,"text":"never","text_percent":null,"updated_ms":1794102388267,"vectors":"never","vectors_percent":0,"vectors_seconds_left":null},"Library":{"relations":"never","sparse":null,"text":"never","text_percent":null,"updated_ms":1794102388271,"vectors":"never","vectors_percent":0,"vectors_seconds_left":null},"Scope":{"relations":"never","sparse":null,"text":"never","text_percent":null,"updated_ms":1794102388269,"vectors":"never","vectors_percent":0,"vectors_seconds_left":null},"Symbol":{"relations":...
test result: FAILED. 0 passed; 1 failed
```

Vert avec le correctif :

```
test daemon_host::tests::un_backend_de_code_dont_la_racine_a_disparu_est_signale_dans_daemon_status_et_ses_outils ... ok
test result: ok. 1 passed; 0 failed
```

## La fin du lot (hotfixs, 8 novembre 2026)

- `index` et `estimate` lisent la racine aussi : leur politique le déclare maintenant
  (`read_files: true` dans `templates/backends/code/backend.json` et `snapshot.json`), et la garde
  de `call_tool_on` les couvre par cette déclaration, pas par leur nom. Avant, un `index` sur une
  racine disparue tentait son balayage. Le témoin le vérifie (son assertion 3). La politique
  `read_files` ne fait qu'élargir les nœuds permis et exiger une clé `workspace`, que les deux
  gabarits portent ; les copies locales du gabarit se diront « dérivées » jusqu'à leur mise à jour.

## Hors de ce lot

- Le symptôme 3 (« Catalogue occupé » sur `usages`, 17 h 56 et 17 h 58) précède 94354595c
  (11 octobre, 18 h 24), qui met la montre en pause sur une racine disparue et fait attendre une
  lecture derrière un catalogue tenu brièvement (ticket fermé
  `closed/2026-10-11-catalogue-occupe-pour-un-changement-localise.md`) ; le refus résiduel
  pendant une réaction en chaîne est suivi par le ticket ouvert
  `2026-10-12-une-lecture-refusee-pendant-une-reaction-une-fois-sur-neuf.md`.
- Le symptôme 4 (un jeton absent du corpus rend trois morceaux sans rapport à ★0,74) n'est pas
  traité ici.
