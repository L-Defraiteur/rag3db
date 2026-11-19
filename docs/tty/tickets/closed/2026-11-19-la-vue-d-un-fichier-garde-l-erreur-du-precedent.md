# La vue d'un fichier garde l'erreur du fichier précédent

- **État** : fermé le 19 novembre 2026 par la session tty, à qui ED l'a passé (« prends-la, oui »), branche `cx-file-erreur`. L'effet de lecture de `File.tsx` remet `error` à `null` avec `read` et `plan`. Témoin : `le_chat_guide_une_fenetre_du_depot_detachee`, qui passe de `src/main.rs` (absent) à `src/long.rs` et attend aucune alerte au-dessus de ses lignes. Rouge (ffc64a181e : l'alerte de `src/main.rs`), puis vert.
- **Famille** : produit
- **Gravité** : gêne. La vue d'un fichier lisible montre au-dessus de son texte l'erreur d'un autre fichier.
- **Atteignable en service** : oui. Dès que la vue `File` passe d'un fichier illisible à un autre sans être remontée : la navigation interne d'une fenêtre détachée (tranche 3), ou un lien d'une vue de fichier vers une autre.
- **Touche rag3weaver** : oui (`ui/ide/src/views/File.tsx`).

## Ce que c'est

Dans le témoin `le_chat_guide_une_fenetre_du_depot_detachee`, la fenêtre du dépôt a ouvert `src/main.rs` (absent de l'espace de travail, « no such file in worktree »), puis `src/long.rs`. La vue de `src/long.rs` affichait ses 60 lignes **et**, au-dessus d'elles, « ReadFileNode: no such file in worktree: … src/main.rs ». L'effet de lecture remet `read` et `plan` à `null`, mais jamais `error`.

## Ce qu'il faut faire

Remettre `error` à `null` au début de chaque lecture, comme `read` et `plan`.

## La preuve

Un témoin navigateur, rouge d'abord : un fichier absent, puis un fichier présent ; la vue du second n'a aucune erreur.
