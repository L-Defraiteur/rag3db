# Le ménage d'arbre principal après la bascule des bases

- **État** : ouvert (16 novembre 2026), arbre principal ; bloqué par la bascule des bases de Lucie.
- **Famille** : produit
- **Gravité** : place disque ; des worktrees et des copies de bases de la répétition restent aux deux postes.
- **Atteignable en service** : non.
- **Touche rag3weaver** : non (fichiers locaux des postes seulement).
- **Pour** : arbre principal ; à faire par Codex après la bascule et le verdict du banc (B′).
- **Signalé par** : arbre principal, rapport de session `docs/arbre-principal/16-11-2026/00-rapport-de-session.md`.

## Ce qu'il faut retirer, et comment

Toujours par `extension/rag3weaver/scripts/garde_worktree.py` pour un worktree (il archive les fichiers non
suivis et refuse une branche non poussée), jamais `rm -rf` sur un worktree. Aucune branche distante n'est effacée.
- **À luciepc** : les worktrees `~/git_workspaces/rag3db-arbre-vues`, `rag3db-arbre-migration`, `rag3db-sync-2bp`,
  `rag3db-sync-2bc` ; sous `~/.cache/rag3weaver-build/bascule/` les copies `rep-*-copie-*` et `essai-requete-*`,
  les binaires `rag3weaver-backend-*`, les journaux `journal-code-*`. **Garder** `rep-code-principal-copie-20261115-2046`
  tant que le banc n'a pas rendu son verdict B′ ; garder `repetition-bascule.sh` (il sert à la bascule réelle).
- **Au portable** : les worktrees `~/.cache/rag3weaver-build/wt-vues`, `wt-migr-gen`, `wt-anglais`, `wt-stack` ;
  les copies `~/.cache/rag3weaver-build/1b-copie/` (la preuve de 1b `migree-20261114-1641` ne part qu'après la bascule
  réelle, aux deux postes) ; les targets `target-vues`, `target-migr-gen`, `target-arbre-*`.

## Ce qu'il faut pour le fermer

La liste de ce qui est parti, avec la place rendue (`du -sh` avant et après), dans le rapport de session du jour.
