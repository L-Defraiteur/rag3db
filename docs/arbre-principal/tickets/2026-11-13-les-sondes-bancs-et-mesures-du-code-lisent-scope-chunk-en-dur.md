# Les sondes, bancs et mesures du code lisent `Scope_Chunk` en dur (lot 1a-2)

- **État** : ouvert (13 novembre 2026), arbre principal ; la suite du lot 1a des vues, à part par
  décision de c7.
- **Famille** : produit
- **Gravité** : sur une base à versions de scope, ces tests mesureraient zéro ou échoueraient sur une
  table absente ; aucun ne tourne dans une passe ordinaire.
- **Atteignable en service** : non (des tests).
- **Touche rag3weaver** : oui (tests).

## Ce que c'est

Depuis le lot 1a, une base de code **neuve** a le schéma à versions : les morceaux et les vecteurs
du code vivent sur `ScopeContent_Chunk`, reliés à `ScopeContent` par `ScopeContent_CHUNKED_FROM`,
et la version rejoint ses occurrences `Scope` par `ScopeContent_DERIVED_FROM`
(`docs/arbre-principal/13-11-2026/29-contrat-de-la-version-de-scope.md`). Les tests ordinaires
passent par `rag3weaver::code::scope_chunks` (la table, le parent, la relation, le chemin vers
l'occurrence, selon la forme de la base). Restent ceux-ci, qui lisent `Scope_Chunk` en dur :

- `tests/sonde_morceaux_vides.rs` (18 lectures) ;
- `tests/e2e_mesure_index_entier.rs` (16, lignes 356 à 527 : mesurerait zéro sans échouer) ;
- `tests/e2e_banc_etage.rs` (5, lignes 983 à 1009) ;
- `tests/e2e_banc_hnsw.rs` (1) ;
- `tests/e2e_mesure_ingestion_code.rs` (2).

## Ce qu'il faut pour le fermer

Chacun passe par `code::scope_chunks`, sans changer ce qu'il mesure, et exige au moins un morceau
avant de juger ou de mesurer. Une mesure qui porte vraiment sur la forme d'avant le dit et prend
`code::register_code_schema_sans_versions`.
