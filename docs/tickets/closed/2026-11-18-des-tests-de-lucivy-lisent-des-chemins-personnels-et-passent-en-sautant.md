# Des tests de lucivy lisent des chemins personnels et passent en sautant

- **État** : corrigé (19 novembre 2026), dans lucivy main : PR #46 (04caeb7, `.vault` et `.env*` jamais lus par le
  parcours de corpus) et PR #47 (e223176). Les quatre diagnostics et `bench_vs_tantivy` sont `#[ignore]` et lisent
  leurs chemins dans l'environnement (`LUCIVY_DIAG_INDEX`, `BENCH_DATASET`, `LUCIVY_BENCH_VS_TANTIVY_DIR`). Les sauts
  passent par `common::usable`. Témoin `test_tests_hold_no_personal_path` : rouge sur main (12 lignes), vert ensuite.
  Tests seulement : pas de version.
- **Gravité** : moyenne, pour la valeur des verdicts. `cargo test -p lucivy-core` peut dire vert alors que des
  tests n'ont rien vérifié. Et un test de vérité peut lire des fichiers de `.vault/`.
- **Atteignable en service** : non (tests et bancs seulement).
- **Touche rag3weaver** : non.
- **Famille** : produit
- **Pour** : lucivy

## Ce qui se passe

- **Chemins personnels en dur**, d'un ancien poste :
  - `lucivy_core/benches/bench_vs_tantivy.rs:25-28` (`/home/luciedefraiteur/linux_bench`, `…/lucivy_bench_vs_tantivy`,
    `…/lucivy_bench_sharding`) ;
  - `lucivy_core/tests/test_diagnostics.rs:15` ;
  - `test_store_fallback.rs:5` ;
  - `test_lock_investigation.rs:5` ;
  - `test_search_mismatch.rs:8`.

  Quand le chemin manque, certains rendent la main sans rien vérifier, et cargo les compte verts.
- **D'autres tests sautent sans le dire comme un rouge** : `test_luce_roundtrip` et `test_luce_playground_search`
  impriment « Skipping » et passent si `playground/dataset.luce` manque.
- **`.vault/` dans le parcours** : `test_fuzzy_ground_truth.rs:290-293`, `test_regex_ground_truth` et
  `test_merge_contains` parcourent la racine du dépôt avec une liste d'exclusions qui n'a pas `.vault`. Seuls
  `.vault/README.md` et `.vault/load.sh` sont suivis par git, mais un fichier non suivi qui y serait posé serait lu et
  indexé par le test.

## Ce qu'il faut faire

1. Ajouter `.vault` (et `.env*`) aux exclusions des trois parcours.
2. Passer les tests de diagnostic à chemin personnel en `#[ignore]`, avec leur chemin lu d'une variable
   d'environnement (`LUCIVY_DIAG_INDEX`…) et un message clair s'il manque. Pas de chemin d'un poste dans le code.
3. Les tests qui sautent passent par `common::usable` (ou une garde de la même forme) : ils échouent avec la recette,
   sauf `LUCIVY_CORPUS_ABSENT_OK=1`, qui dit le saut en toutes lettres.

## Comment le prouver

- Un témoin rouge pour les exclusions : un parcours sur un dossier temporaire qui contient `.vault/secret` ne doit pas
  le rendre.
- Puis `cargo test --release -p lucivy-core --no-fail-fast`, en relevant les tests ignorés ou sautés avant et après.
