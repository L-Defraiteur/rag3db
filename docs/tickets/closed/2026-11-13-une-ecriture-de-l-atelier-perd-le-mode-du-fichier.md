# Une écriture de l'atelier perd le mode, le propriétaire et les liens physiques du fichier

- **État** : corrigé (13 novembre 2026, branche atelier-ecriture-sure, archi). Témoins
  `a_workshop_write_keeps_the_file_mode`, `a_workshop_write_in_a_worktree_keeps_the_file_mode` et
  `a_workshop_write_refuses_a_file_changed_since_it_was_read`, rouges puis verts.
- **Famille** : produit
- **Gravité** : réponse fausse — un script exécutable perd son bit `+x`, un fichier `0600` (un secret) devient lisible de tous selon l'umask
- **Atteignable en service** : oui, à toute écriture hors worktree par l'atelier d'un fichier au mode non défaut ; probablement aussi côté worktree (`WorkingTree::write`, `code_tools.rs` 317-325, même schéma)
- **Touche rag3weaver** : oui (`extension/rag3weaver/src/daemon_host/atelier_fichiers.rs` : `ecrire_atomique` 556)
- **Pour** : rag3weaver archi (l'atelier qui lit et écrit, son auteur)
- **Signalé par** : everything declarative, relecture de l'atelier sur origin/master 73c7f15e1, à la demande de l'orchestration (13 novembre 2026) ; lignes de origin/master à cette date

## Ce que c'est

`ecrire_atomique` écrit un fichier neuf (`fs::write`) puis le renomme sur la cible : le mode, le
propriétaire et les liens physiques de l'ancien fichier sont perdus. Aucun `fsync` avant le
renommage, et le temporaire reste sur le disque si l'écriture échoue.

## Le témoin à écrire

Un fichier `0755` et un `0600` réécrits par l'atelier : leurs modes restent ; un échec d'écriture
ne laisse aucun temporaire.

## Ce qui le fermerait

Copier les permissions de l'ancien fichier sur le temporaire avant le renommage, `fsync`, nettoyer
le temporaire sur erreur ; même vérification côté worktree ; le témoin vert.
