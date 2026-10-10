# Arbre principal — rapport de session, 10 octobre 2026

*Mis à jour le 10 octobre au soir. La version du matin (pause pour le redémarrage du
poste) est dans l'historique de ce fichier.*

## Ce qui est sur master

| Commit | Quoi |
|---|---|
| `1b0d5483b`, `a67c083a3`, `21a1d67c5` | **La bascule des défauts** : plein texte en fichiers pour une base neuve, transaction par paquet active, paquets de 2 048. La transaction suit `DialectCapabilities` : coupée et dite dans `warnings` sur un dialecte sans `transactions`, refus nommé si elle est demandée. Deux tests ajustés (ci-dessous). |
| `633d9bf90` | README et page des défauts (`5-octobre-2026-08h45/01-les-defauts-bascules.md`) au présent, avec les décisions de Lucie. |
| `f9aa29a28` | Le pointeur codeparsers sur `4c7897c` (tree-sitter-scss en copie locale) ; le Cargo.lock perd `source` et `checksum`. |
| `1647c75c3` | `landlock` pour Linux seulement. |
| `5f5147ab9` | Ticket « une arête ne dit pas comment elle a été résolue » : un appel nu importé par `use` est marqué `import`. Le trou connu : pas de e2e ciblé TypeScript ni Python. |

**La batterie de la bascule** (lib de 10:46, hors carte locale, régime doux) : 72 suites
vertes sur 74. Les deux rouges étaient des tests :
- `e2e_points_de_reprise_nettoyes` : une source neuve passe en transaction, qui coupe les
  points de reprise du dataflow ; le fils joue désormais le chemin sans transaction ;
- `e2e_agent_loop` : l'assertion cherchait `[SENT_TO]` alors que le rendu dit
  `~ Sent to ~` depuis `98478b5ef` (4 octobre). Rouge aussi sur master, jamais rejoué
  depuis.

Après le rebase sur master (rag3weaver-ir, Hop, la fuite de pages corrigée par
`0aed3c4b5`) : lib commune rebâtie à 15:36 (version de stockage 40), puis la lib,
`--tests --no-run` et sept suites rejouées, toutes vertes.

## Les branches poussées, pas encore sur master

- **`embarqueur-absent`** (`c98d4ff9b`) : `AbsentEmbedder::new(modèle, dimension)`. Il
  porte le nom et la dimension du modèle attendu, et la dette se compte contre lui.
  - Aucune écriture n'embarque, et le mot du repli est dit une fois par catalogue.
  - Le rattrapage rend 0 sans toucher à l'index.
  - Aucun débit n'est sondé ni noté.
  - La branche dense dit « not available ».
  - L'avertissement nommé est donné au démarrage.
  - Vert sur la lib de 15:36 (lib 1251, `e2e_embarqueur_absent`). L'optimiseur (portes
    du backend, `paquet-npm`) s'y lie.
  - À rejouer sur la lib du COPY journalisé avant la fusion.
- **`commande-en-fond-2`** (`cbebe4268`), sur `execution-asynchrone` (`da25d8e0f`) :
  - `run` en fond prend la poignée de l'appel (`tool_handle`) ;
  - sa fin est postée dans la boîte du run (`agent_bus`, `agent_inbox`) ;
  - sa mort est confiée à la portée du run (`run_scope`), et la boîte reçoit « tuée à la
    fin du run » (texte partagé avec le témoin de la recherche, ne pas le reformuler
    seul) ;
  - les huit témoins de la page `01-la-commande-en-fond.md` sont joués : lib 1262 ;
  - fusion après `execution-asynchrone`, puis rebase sur master.
  - Nom neuf : le rebase réécrivait les commits, et le message d'un commit portait une
    ligne d'attribution à retirer. L'ancienne `commande-en-fond` est abandonnée.
- **`temoin-fuite`** (worktree `~/.cache/rag3weaver-build/wt-fuite`, pas encore poussée).
  C'est le témoin produit de la fuite de pages, dans `e2e_tx_par_paquet_arret` :
  `en_fichiers_…` et `en_blobs_une_mort_pendant_un_copy_journalise_se_rejoue_et_se_retrouve`.
  - Un paquet de 150 000 scopes (un groupe plein) est validé en COPY journalisé.
  - L'écrivain meurt au paquet suivant ; la reprise rejoue et va au bout.
  - Les comptes doivent égaler ceux d'un témoin sans arrêt, et `g5_7` doit se retrouver
    par mot et par vecteur.
  - Preuve que le COPY a bien été journalisé : `copy_journal_fallbacks` = 0 au témoin.
  - Crochet `RAG3WEAVER_TEST_COPY_JOURNALISE=1` : point de reprise forcé coupé, seuil
    du journal à 4 Gio.
  - **Pas encore joué** : la passe a été arrêtée pour la batterie du moteur basculé.

## En cours

**La batterie complète contre le moteur basculé** (`ff9bad960`, le COPY journalisé
devient le défaut), demandée par l'orchestration :
- l'arbre principal est avancé sur master `aeec6888f` ;
- en poste lourd : rebâti de la lib commune, lib, `--tests --no-run`, puis la batterie
  hors carte locale ;
- chaque rouge sera trié : moteur / test qui supposait le point de reprise forcé / autre.

**Les bases de test se refont** : depuis `0aed3c4b5`, une base écrite par la nouvelle lib
n'est plus lisible par l'ancienne.

## Ce qui reste, dans l'ordre

1. La batterie : le compte, les rouges triés, à l'orchestration.
2. Le témoin de la fuite, sur la nouvelle lib. Pousser `temoin-fuite`, donner son vert au
   cœur C++ (rag3db-91) pour son ticket, puis fusionner.
3. L'embarqueur absent, rejoué sur la nouvelle lib, puis fusionné. Prévenir l'optimiseur
   (rag3db-90).
4. La commande en fond, au signe de la recherche (rag3db-97).
5. Plus tard, pour embarquement : `Catalog::execute_raw` refuse par son nom sur un dialecte
   sans `cypher`.

## Les worktrees

| Worktree | Branche | Note |
|---|---|---|
| `wt-defauts` | `defauts-bascules-2`, avancée sur master | sert aux batteries |
| `wt-absent` | `embarqueur-absent` | |
| `wt-fond` | `commande-en-fond-2` | |
| `wt-fuite` | `temoin-fuite` | |
| `wt-docs-2` | détaché sur master | les docs |

Chacun a le sous-module codeparsers initialisé et le lien `extension/vector/build` vers
l'arbre principal.

## Arrêt pour le nettoyage du disque (10 octobre, soir)

Sur demande de Lucie, relayée par l'orchestration : tout arrêté par pid, tout le travail
local committé et poussé. Les `target` cargo, les `build/` et les worktrees vont être
effacés ; rien n'a été perdu de mon côté.

**La batterie du basculement** (lib de 17:27, moteur `aeec6888f` avec `ff9bad960`, sans
`d10b92306` ; depuis `wt-defauts` sur `aeec6888f` ; hors carte locale, régime doux) :
- lib et `--tests --no-run` verts ;
- **61 suites vertes, aucun rouge**, interrompue pendant la 62ᵉ, `e2e_sparse_dump` ;
- à reprendre à partir de `e2e_sparse_dump`, sur une lib rebâtie : la lib de 17:27 sera
  effacée avec `build/`. Suites jouées, dans l'ordre :
  `e2e_agent_loop`, `e2e_arret_brutal`, `e2e_avis_du_modele`, `e2e_banc_cohesion`, `e2e_banc_etage`, `e2e_banc_hnsw`, `e2e_banc_liens`, `e2e_banc_qualite`, `e2e_banc_relations`, `e2e_banc_texte_brut`, `e2e_banc_vertex_embedding`, `e2e_batch_observe`, `e2e_catalogue_gabarits`, `e2e_charge_ingestion`, `e2e_checkpoint`, `e2e_chemin_de_masse`, `e2e_cloud_code_agent`, `e2e_cloud_schema_probe`, `e2e_code`, `e2e_code_sync`, `e2e_conversation_a_plusieurs`, `e2e_copy_liens`, `e2e_copy_refuse_rouvre`, `e2e_dataflow_observe`, `e2e_drain_unified`, `e2e_entites_derivees`, `e2e_estimate`, `e2e_generic_search`, `e2e_graph_tool`, `e2e_graphe_et_paquets`, `e2e_highlight_long_text`, `e2e_hnsw_scale`, `e2e_idempotent_registration`, `e2e_impact`, `e2e_impact_fichier`, `e2e_invariant_des_vecteurs`, `e2e_lecture_du_catalogue`, `e2e_lecture_mermaid`, `e2e_lecture_seule_apres_fermeture_sans_point_de_reprise`, `e2e_liens`, `e2e_mesure_joignabilite`, `e2e_mesure_sync_source`, `e2e_native`, `e2e_phase0b`, `e2e_plans_par_lot`, `e2e_points_de_reprise_nettoyes`, `e2e_preuve_d_existence`, `e2e_prise_atomique`, `e2e_profile_overhead`, `e2e_rag3daemon`, `e2e_reacteur`, `e2e_recherche_dense_apres_suppressions`, `e2e_repli_de_branche`, `e2e_reprise_durcie`, `e2e_rerank`, `e2e_result_mode`, `e2e_rouvrir`, `e2e_scope`, `e2e_search`, `e2e_search_queue`, `e2e_simple_entity`, `e2e_sparse_dump`.

**Les branches, à l'arrêt :**

| Branche | Tête | État |
|---|---|---|
| `embarqueur-absent` | `eaa5dbe19` | `c98d4ff9b` vert (lib de 15:36) ; `be3e9ab8d` (count_rows) et `eaa5dbe19` (`Level::NotDeclared`, accordé par l'orchestration) **non compilés**. |
| `commande-en-fond-3` | `8c5105f7b` | lib 1264 verts, sur `execution-asynchrone` `ca757a315` ; à rebaser sur master par `git rebase --onto master ca757a315` au signe de la recherche. |
| `temoin-fuite` | `88a1958d6` | **jamais rendu** : lancé une fois sur la lib de 15:36, arrêté avant sa fin. |
| `commande-en-fond`, `commande-en-fond-2` | — | abandonnées (remplacées par `-3`). |

**La reprise, dans l'ordre :**
1. Rebâtir la lib commune ; finir la batterie à partir de `e2e_sparse_dump`, et dire
   « fini » à la recherche, qui l'attend pour sa mesure.
2. `embarqueur-absent` : lib, `--tests --no-run` (un champ public change),
   `e2e_embarqueur_absent`, `e2e_etat_non_declare`. Puis master, et prévenir l'optimiseur
   (rag3db-90).
3. Le témoin de la fuite ; son vert pour le ticket du cœur C++ (rag3db-91).
4. `getRecoveryLoadFailures()` sous `warnings` à l'ouverture (ticket
   `2026-10-10-echecs-de-chargement-au-rejeu-que-personne-ne-lit.md`). L'accesseur n'est
   pas exposé côté Rust : passage par `tools/rust_api`. Le refus d'ouverture est au cœur,
   pas à moi.
5. La commande en fond, au signe de fusion de la recherche (`execution-asynchrone-2`).

## La batterie du basculement, rendue (10 octobre, 19 h 34)

**74 suites vertes, aucun rouge** : c'est la confirmation produit de la condition 2 (le COPY
journalisé par défaut, `ff9bad960`).
- Conditions : lib de 17 h 27, moteur `aeec6888f` (`ff9bad960` dedans, sans `d10b92306`),
  `RAG3WEAVER_MOTEUR_ANCIEN=1` dit ; hors carte locale, régime doux.
- Deux parties : 61 suites avant le nettoyage du disque ; les 13 dernières après, à partir de
  `e2e_sparse_dump`, sur la même lib (101 tests). Le journal de la première partie a été
  effacé avec `~/.cache`, et son compte est plus haut dans ce rapport.
- Écartées : `e2e_postgres` (pas de conteneur), les cinq suites `openai-llm`
  (`e2e_avis_du_modele`, `e2e_cloud_code_agent`, `e2e_cloud_schema_probe`,
  `e2e_conversation_a_plusieurs`, `e2e_lecture_mermaid`) et la carte locale (`e2e_burn_*`,
  `e2e_demon_embeddings`, `e2e_mesure_ingestion_code`, `e2e_banc_bge_m3`).

Depuis : les compilations passent par un seul target, `~/.cache/rag3weaver-build/target-A`,
avec `CARGO_INCREMENTAL=0`.
