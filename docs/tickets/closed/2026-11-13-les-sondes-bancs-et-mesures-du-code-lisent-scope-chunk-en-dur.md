# Les sondes, bancs et mesures du code lisent `Scope_Chunk` en dur (lot 1a-2)

- **État** : corrigé (18 novembre 2026, c399bc86a7, 38da65ce87, d57c36968e), arbre principal ; la suite
  du lot 1a des vues, à part par décision de c7. Le reste (`sonde_identifiants_lucivy.rs`) est un ticket à
  part : `docs/arbre-principal/tickets/2026-11-18-la-sonde-des-identifiants-lucivy-lit-l-index-plein-texte-de-scope.md`.
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
- `tests/sonde_identifiants_lucivy.rs` : l'index plein texte de `Scope` (`fts_handle(SCOPE)`), qui vit
  sur `ScopeContent` dans une base à versions.

## Ce qu'il faut pour le fermer

Chacun passe par `code::scope_chunks`, sans changer ce qu'il mesure, et exige au moins un morceau
avant de juger ou de mesurer. Une mesure qui porte vraiment sur la forme d'avant le dit et prend
`code::register_code_schema_sans_versions`.

## Fermeture (18 novembre 2026)

Poussé sur master par la garde après une passe verte (journal `arbre-sondes` à luciepc) :

- **c399bc86a7** : le rouge. Le banc HNSW exige au moins un morceau lu ; vu rouge avant le correctif
  (« no scope chunk read: the bench would measure an empty table »).
- **38da65ce87** : le correctif. `e2e_banc_hnsw`, `e2e_mesure_ingestion_code`, `e2e_banc_etage` et
  `sonde_morceaux_vides` lisent la table par `code::scope_chunks`. `sonde_morceaux_vides::compter_une_base` la
  choisit par le tampon de forme de la base. Les mesures à morceaux vides de `e2e_mesure_index_entier`
  refusent une base à versions par leur nom (ticket du 18 novembre sur cette forme qui va disparaître).
- **d57c36968e** : la doc de `base()` remise au-dessus de `base()`.

**Le sujet de 38da65ce87 dit « NOT COMPILED, NOT PLAYED ».** C'était vrai quand il a été écrit, avant l'arrêt
du 16 novembre à 10 h 5x. Il est historique : un commit poussé ne se réécrit pas. Le lot est compilé et joué
depuis. La passe `arbre-sondes` du 18 novembre, 14 h 03, sur la tête reposée sur 411be25cb2, en est le juge :

- témoin `e2e_banc_hnsw` 1/1 ;
- `check_features` 8/8 ;
- lib 2 614 ;
- témoins navigateur 45/45 ;
- 0 test non jugé.

La lib était 59ac9c86bc, sous `RAG3WEAVER_MOTEUR_ANCIEN=1`.

Les lectures de `Scope_Chunk` qui restent sont voulues :

- les mesures de la forme d'avant, dans `e2e_mesure_index_entier` ;
- la branche « forme d'avant » de `compter_une_base` ;
- des commentaires.
