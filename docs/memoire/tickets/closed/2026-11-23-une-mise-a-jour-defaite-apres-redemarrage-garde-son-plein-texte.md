# Une mise à jour défaite après un redémarrage gardait son plein texte

- **État** : corrigé (23 novembre 2026), branche `cx-memoire-fts-unbuffered`.
- **Famille** : produit
- **Gravité** : la recherche par mots rendait le texte défait, et ne trouvait plus le texte rétabli, alors que la base
  tenait le bon. Aucune erreur ni trace.
- **Atteignable en service** : oui, par le retour arrière d'une migration (`Catalog::migration_rollback_graph`), qui
  rejoue `UpdateRecordNode::undo` hors de tout lot, après un redémarrage de l'hôte. Les écritures ordinaires (outils,
  ingestion, `put_fiche`) passent par un lot en transaction et son tampon : elles n'étaient pas touchées. Un témoin le
  montre : une fiche mise à jour après un redémarrage se trouve par son mot neuf.
- **Touche rag3weaver** : oui, `reindex_fts_rows` (`dataflow/record_nodes.rs`).
- **Signalé** : le 23 novembre 2026, à la lecture du code, en cherchant pourquoi un rappel rendait des fiches à côté.

## La cause

`reindex_fts_rows` prenait l'identifiant interne d'une ligne dans le cache de la session (`NodeIdCache`). Hors lot,
sans tampon, une ligne absente de ce cache était sautée par un `continue` muet. Après un redémarrage, le cache est vide :
toutes les lignes l'étaient.

## Le correctif

Hors lot, les lignes absentes du cache ont leur identifiant lu en base (`select_node_ids`), comme le fait déjà
`DeleteRecordNode`. Une ligne qui n'a d'identifiant ni en cache ni en base est comptée et dite au journal, jamais
sautée sans un mot.

## Le témoin (vu rouge d'abord)

`an_update_undone_after_a_restart_reindexes_the_restored_text` (`tests/e2e_undo/after_restart.rs`) : mise à jour,
cache vidé, annulation. Le texte rétabli se trouve, le texte défait ne se trouve plus, sans réingestion.
