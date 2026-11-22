# La migration des versions n'écrit pas sur les textes partagés

- **État** : ouvert (22 novembre 2026, embarquement ; reste du lot L2a des textes partagés, à corriger avant L5)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : basse aujourd'hui. Les vecteurs écrits par cette migration vont aux morceaux seulement ; la migration
  des textes partagés (L1) les reporte sur les textes à la prochaine ouverture qui voit un nouvel ensemble de modèles.
  Après L5, où les morceaux ne portent plus de vecteur, ils seraient perdus.
- **Atteignable en service** : non tant que `shared_text_vectors` est éteint ; oui ensuite, lors d'une migration des
  versions.
- **Touche rag3weaver** : oui, `src/migration_des_versions.rs` (son appel à `record_nodes::write_vectors`)

## Ce que c'est

Le lot L2a fait passer chaque écriture de vecteur sur un morceau par son texte (`write_all` dans `EmbedNode`, et
l'écriture du rattrapage dans `catalog.rs`). `migration_des_versions.rs` appelle `write_vectors` directement sur
`ScopeContent_Chunk`, sans le texte.

## Ce qui reste à faire

Y poser le vecteur sur le texte aussi (`crate::shared_text::set_text_vectors`, colonne de `text_storage` du modèle
neuf), sous le réglage, avec son témoin. **Avant L5** (la bascule).
