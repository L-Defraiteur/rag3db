# Un plan de gabarits dit compatible un changement de `name` alors que c'est l'identité du dossier

- **État** : corrigé le 13 novembre 2026, dans le lot qui l'ouvre (session mémoire, 96). Relevé par
  everything declarative en relisant la première tranche de M8.
- **Famille** : produit
- **Gravité** : un plan vert, puis une ouverture refusée.
- **Atteignable en service** : non : la voie fermée de M8 n'a pas encore d'appelant en service.
- **Touche rag3weaver** : oui, `src/template_plan.rs` (`compatibility`).

## Ce qu'on voyait

`compatibility` ne gardait pas `name`. Or `FormatDuDossier.gabarit` est le nom du manifeste
(`backend.rs`), et `juger_le_manifeste` refuse un dossier `files_root` écrit sous un autre nom
(`fichiers_entite.rs`). Une cible qui renommait un backend à fichiers donnait un plan vert.

## Le correctif

Un changement de `name` est refusé, comme compatibilité à qualifier, quand un `files_root` est
déclaré, chez le local ou chez la cible. Sans `files_root`, rien ne dépend du nom ici, et le témoin
existant d'un nom local gardé reste vert. Témoin :
`template_plan::tests::renaming_a_backend_with_a_files_root_is_not_compatible`, rouge sur
dd8111736 (plan Ok), vert après.
