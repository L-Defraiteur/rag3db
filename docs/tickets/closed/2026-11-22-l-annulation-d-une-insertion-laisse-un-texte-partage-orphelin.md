# L'annulation d'une insertion laisse un texte partagé orphelin

- **État** : corrigé (22 novembre 2026, lot L2b `shared-text-l2b`, témoin vu rouge puis vert ; ouvert le 22 novembre 2026, embarquement ; reste du lot L2a des textes partagés, à corriger avant L5)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : basse. Un texte sans morceau occupe sa ligne et son vecteur ; il ne fausse aucune recherche, puisque la
  recherche par les textes revient toujours aux morceaux admis.
- **Atteignable en service** : non tant que `shared_text_vectors` est éteint (le défaut) ; oui ensuite, à chaque
  annulation d'une insertion de morceaux.
- **Touche rag3weaver** : oui, `src/dataflow/record_nodes.rs` (`InsertRecordNode::undo`)

## Ce que c'est

Le lot L2a (9dd3a9d424) retire les textes devenus orphelins sur les quatre voies de suppression des morceaux
(`DeleteRecordNode`, `RechunkDeleteNode`, `set_aside_by_rule`, `purge_layer`) : les empreintes sont lues avant, le
retrait borné à elles se fait après, en un énoncé. `InsertRecordNode::undo` supprime aussi des lignes, des morceaux
compris, par `Write::Delete { by: "_uuid", cascade: true }`, sans ce retrait : le texte que l'insertion annulée avait
créé reste, sans arête.

## Ce qui reste à faire

Le même retrait borné dans l'annulation (c7, 22 novembre : « le plus complet »), avec son témoin : une insertion
annulée ne laisse aucun texte que seul son morceau désignait. **Avant L5** (la bascule).
