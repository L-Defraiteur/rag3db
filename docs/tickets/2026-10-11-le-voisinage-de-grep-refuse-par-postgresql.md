# Sur PostgreSQL, le voisinage d'une recherche de code est refusé

- **État** : ouvert (11 octobre 2026) — à la session embarquements, après les lots 2 à 4 du Cypher brut ; l'arbre principal relit `code_tools.rs`
- **Gravité** : blocage (sur PostgreSQL, la recherche de code avec une relation échoue)
- **Atteignable en service** : avec un montage PostgreSQL du backend de code ; rag3db n'est pas concerné
- **Touche rag3weaver** : oui (`src/code_tools.rs`, `voisins_des_scopes`)

## Ce que c'est

`voisins_des_scopes` (`code_tools.rs:1034`) bâtit un saut sans table d'arrivée
(`Hop::untyped`) qui lit des champs du voisin (`name`, `scope_type`, `file_path`,
`start_line`, `end_line`). Le dialecte PostgreSQL le refuse en le nommant : « Hop (champ
sans table d'arrivée) ». rag3db l'accepte, parce qu'un nœud de table quelconque y rend
un champ absent comme nul.

## Ce qu'il faut pour le fermer

Fixer `end = Scope` ne suffit pas : la relation est quelconque. `DEFINES` va vers
`Symbol`, qui n'a pas `scope_type`, et rag3db échouerait sur le champ absent. La forme
juste : la table d'arrivée de la relation (`rel_info`), et des colonnes lues selon
elle, avec `Column::Null` pour un champ qu'elle ne déclare pas. Le témoin : la
recherche de code avec une relation, jouée sur les deux moteurs.
