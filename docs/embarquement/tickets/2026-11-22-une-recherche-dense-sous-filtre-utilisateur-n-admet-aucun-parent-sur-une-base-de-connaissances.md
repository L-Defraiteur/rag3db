# Une recherche dense sous filtre utilisateur n'admet aucun parent sur une base de connaissances

- **État** : ouvert (22 novembre 2026, embarquement ; relevé en écrivant les témoins du lot L3 des textes partagés)
- **Pour** : embarquement (le relevé) ; à départager avec qui tient `admitted_parent_uuids_for` (recherche)
- **Famille** : produit
- **Gravité** : moyenne. La branche dense rend zéro résultat, sans avertissement, quand un filtre utilisateur porte
  sur un champ texte de la cible ; un appelant croit que rien ne correspond.
- **Atteignable en service** : à départager. Relevé sur une base en mémoire à une base de connaissances (`main`,
  alimentée par `Document`) ; non essayé sur une base de code.
- **Touche rag3weaver** : oui, `src/dataflow/generic_search_nodes.rs` (`admitted_parent_uuids_for`, puis
  `Forme::resolve_filter_to_ids`)

## Ce que c'est

Témoin de contrôle joué le 22 novembre (branche `shared-text-l3`, lib 2c4bddcd3, réglage `shared_text_vectors`
**éteint**, donc l'ancien chemin des morceaux) : deux `Document` de même corps, titres `first` et `second`, la base de
connaissances `main` (`title_for`, `content_for`). Recherche `SearchSignals::VECTOR`, `ResultMode::SourceResolved`,
`Consistency::Immediate` :

- sans filtre : les deux documents reviennent ;
- avec `FilterBuilder::new().eq("title", "second")` : **aucun résultat**, `vector_count: 0`, aucun avertissement.

Le moteur n'y est pour rien : la projection compilée par `compile_filter_for_vector_in`
(``MATCH (n)-[:`main_CHUNKED_FROM`]->(p:`main`) WHERE p.`title` = $…``) admet bien le morceau de `second`, et
`p.title` vaut `second` en base. Le résultat se perd ensuite, à l'admission des parents en Rust
(`admitted_parent_uuids_for`, qui résout le filtre en uuids par `resolve_filter_to_ids` sur `main`), puis
`resolve_vector_chunks_admitted`.

## Ce qui reste à faire

1. Rejouer le témoin de contrôle (il est dans l'historique de la branche `shared-text-l3`, commit `wip sonde L3 4`)
   et imprimer l'ensemble admis par `admitted_parent_uuids_for`.
2. Départager : un filtre sur un champ texte passe-t-il par le plein texte (non ouvert sur cette base), ou par la
   table ? Comparer avec un champ non texte.
3. Un témoin rouge, puis le correctif, ou bien un avertissement si un filtre ne peut pas être résolu.

Le témoin de L3 (`a_shared_text_comes_back_only_through_the_chunks_the_filter_admits`) juge donc le moteur des textes
partagés directement, avec le filtre compilé par le catalogue, et non la recherche de bout en bout.
