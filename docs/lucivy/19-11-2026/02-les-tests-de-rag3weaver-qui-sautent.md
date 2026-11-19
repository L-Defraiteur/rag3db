# Les tests de rag3weaver qui sautent et passent : le relevé (19 novembre 2026)

Demandé par c7, sur la règle de Lucie : un test qui saute n'est pas vert. C'est le même relevé que celui de lucivy
(PR #46 et #47). Il a été fait par lecture des sources de `extension/rag3weaver` (master du 19 novembre), sans rien
jouer.

## La convention déjà en place

- Un test qui ne peut pas juger imprime `SAUTÉ <suite> — <raison>` et rend la main
  (`tests/common/mod.rs`, `sauter_si_indisponible`). `run_e2e.sh` le **retire des verts** et liste les sautés
  (l. 1343-1350). Un saut qui suit cette convention n'est donc pas un vert.
- Un test qui imprime `non jugé : …` dans une suite exigée est refusé par la garde (`push_garde.sh`,
  `required_suites_played`).
- Toutes les e2e sont `#[ignore]` et jouées par `run_e2e.sh` avec `--ignored`. Un retour silencieux y compte donc
  comme un vert dans les passes.

## 1. Les défauts : des tests qui rendent la main sans `SAUTÉ`

| test | ce qui manque | ce qu'il imprime |
|---|---|---|
| `tests/e2e_cloud_code_agent.rs:157` et `:248` (2 tests) | un modèle (`RAG3WEAVER_LOCAL_LLM` ou Vertex) | `skipped: …` |
| `tests/e2e_conversation_a_plusieurs.rs:258` | un modèle | `skipped: aucun modèle …` |
| `tests/e2e_cloud_schema_probe.rs:49` | Vertex | `skipped` |
| `tests/e2e_hnsw_scale.rs:72-122`, 4 tests (`n4096`, `n1024_dim4`, `n768`, `raw_update_path_n4096`) | `RAG3DB_PROBE_HNSW` (une sonde de plusieurs minutes) | `skipped: long probe …` (dans `probing()`, l. 18-24) |
| `tests/e2e_catalogue_gabarits.rs:457` (`un_motif_ajoute_ses_champs_avant_l_enregistrement`) | le motif `versioned` du catalogue livré | rien |
| `tests/e2e_mesure_index_entier.rs`, 11 tests (l. 56 à 625) | `base()`, une base à mesurer, lue dans l'environnement | rien |

Soit **20 tests** qui comptent comme verts sans avoir rien vérifié quand leur ressource manque.

## 2. Pas des tests : les points d'entrée d'un enfant

`role_enfant()` dans neuf fichiers (`e2e_copy_refuse_rouvre.rs:82`,
`e2e_lecture_seule_apres_fermeture_sans_point_de_reprise.rs:133`, `e2e_reprise_durcie.rs:123`,
`e2e_stockage_du_plein_texte.rs:75`, `e2e_tampon_plein.rs:65`, `e2e_tx_ligne_a_ligne.rs:160`,
`e2e_tx_par_paquet_arret.rs:278`, `e2e_tx_vecteurs_apres_rollback.rs:136`), et `role_hote_qui_sort` dans
`src/commande.rs:1700`. Le test parent les relance en sous-processus, avec `ROLE` et `BASE` posés. Joués seuls,
ils rendent la main. Ils ne cachent aucune vérification, puisque c'est le parent qui juge. Mais chaque passe les
compte comme des verts : environ dix « verts » par passe qui ne sont pas des tests.

## 3. Marginal

- `tests/probe_unique_definer_misses.rs:33` et `tests/sonde_marque_nom.rs:32` : des sondes dont le parcours de
  dossiers saute en silence un dossier illisible (le même défaut que le parcours de vérité de lucivy).
- `src/commande.rs:2139` (`cargo_plays_its_tests_inside_the_sandbox`) rend la main sans `HOME`. `HOME` est toujours
  posé ; on peut le laisser.

## 4. Le lot proposé

- **Le témoin, rouge d'abord** : un test de la lib (ou un script sous `tools/poste/`) qui lit `tests/*.rs` et les
  modules de test de `src/`. Il rougit sur :
  - un message de saut qui n'est pas `SAUTÉ` (`skipped`, `Skipping`, `SKIP`) ;
  - un `return` dans une fonction `#[test]` derrière une variable d'environnement, un fichier ou une ressource
    absente, sans `SAUTÉ` ni `non jugé :` sur la même branche, sauf pour un point d'entrée d'enfant nommé
    `role_*`.

  Rouge sur la liste du § 1.
- **Les correctifs** : une aide commune `common::sauter(suite, raison)`, qui imprime la ligne `SAUTÉ`, utilisée
  partout au § 1. Pour `e2e_catalogue_gabarits`, deux choix : le motif `versioned` existe dans le catalogue livré, et
  le test l'exige ; ou il n'existe pas, et le test se retire. Les sondes du § 3 comptent leurs dossiers écartés.
- **Pour le § 2**, à décider par c7 : soit un marqueur `ENFANT <test>`, que `run_e2e.sh` retire des verts comme
  `SAUTÉ` ; soit on les laisse tels quels, en le disant dans la doc de `run_e2e.sh`.
- **Taille** : deux à trois heures. Le lot touche `tests/` et éventuellement `run_e2e.sh`. Le témoin et la suite
  touchée se rejouent, puis le lot est déposé au pousseur.
