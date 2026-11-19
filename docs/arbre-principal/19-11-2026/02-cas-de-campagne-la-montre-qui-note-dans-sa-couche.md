# Cas de campagne : la montre qui note dans la couche du backend

Arbre principal, 19 novembre 2026, pour la campagne des correctifs passés (chantier de Lucie). C'est un cas court, en
Rust, dans le crate rag3weaver. Le témoin n'a pas besoin de la lib rag3db à l'exécution : il surveille un dossier
temporaire, avec un registre `Lag` en mémoire.

## Le point de départ

- **Commit parent** : `f54bb40e17` (19 novembre 2026, 9 h 10, sur la branche `arbre-principal/montre-par-couche-2`).
  Le témoin y est, rouge ; le correctif n'y est pas.
- **Arbre** : `extension/rag3weaver`. Les fichiers en jeu sont `src/watch.rs` et `src/watch/tests.rs`.
- Ce cas atterrit sur master avec le dépôt `pile-arbre-5b1-2` (lot 5b-1 de 2a′-3), qu'il suit au pousseur.

## Le signalement, tel qu'il était

> 2a′-3, lot 5b : une montre sert une couche. Celle du backend sert la couche du backend ; celle d'un worktree, la
> couche de travail de ce worktree. `watch::spawn` reçoit désormais la `LagKey` de la couche qu'il sert, et non plus la
> seule cellule. Mais un changement sous une montre de couche de travail est encore noté dans la couche du backend de
> la même cellule : une vue du backend se dit alors en retard pendant qu'on édite un autre worktree.

## Le témoin

- **Filtre** : `watch::tests::a_watch_notes_in_the_layer_it_serves`, dans `extension/rag3weaver/src/watch/tests.rs`.
  Il est identique sur la tête de la chaîne (`eb60d1525d`).
- **Suite** : `cargo test --lib` du crate `extension/rag3weaver`, avec la feature `code`.
- **Rouge au parent** : l'assertion « the change is not noted in the layer of the watch » échoue. Le retard est noté
  sous `LagKey::backend(cell)`.
- **Suites exigées par les chemins** : aucune pour `src/watch.rs`. Notre correctif touche aussi `src/catalog/*` et
  `src/code_sync.rs`, donc `e2e_code_sync`.

## Les indices présents dans l'arbre

- `Lag::note_in(&LagKey, …)` existe déjà, à côté de l'ancienne `Lag::note(&Scope, …)`.
- La ligne fautive porte un commentaire `RED STUB` dans `watch.rs`.

## Notre correctif (caché à Algareth, pour le juge)

`bbd469fdfa`, « feat(watch): a watch notes in the layer it serves, and the lag's cell-only aliases die » : six
fichiers, +35 −69.

- `src/watch.rs` +2 −1 suffit au témoin (`lag.note_in(&layer, …)`).
- Le reste retire les anciennes méthodes à la cellule seule et fait passer chaque appelant par la couche :
  - `src/catalog/retard.rs`, `src/backend.rs` ;
  - `src/code_sync.rs`, `src/dataflow/catalog_read.rs` ;
  - `src/dataflow/reindex_nodes.rs`.
- Un correctif plus étroit, `watch.rs` seul, ferait aussi le vert. C'est au juge de dire s'il vaut le nôtre.
