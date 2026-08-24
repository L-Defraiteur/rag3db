# Doc 11 — État des lieux au 24 août 2026

Photographie de fin de session. Tous les chiffres marqués ✅ ont été **mesurés**
ce jour-là, pas repris d'un doc antérieur — plusieurs docs de mars et de mai
étaient périmés et l'ont fait perdre du temps.

Compagnons : [12 — ambitions](12-ambitions-et-roadmap.md) ·
[13 — knowledge dump](13-knowledge-dump.md)

---

## En une phrase

La migration FTS vers lucivy v3 en Rust direct **fonctionne, KB compris** ;
l'embedder BGE-M3 tourne sur burn/Vulkan avec parité prouvée contre candle ; les
quatre cibles de build sont réparées et protégées par un CI qui n'existait pas.

## Branche et état git

| | |
|---|---|
| Branche | `fts-lucivy-v3`, **poussée** sur `origin` |
| Commits devant `master` | 14 |
| `master` | `962ce9dc2`, synchronisé local/distant |
| Non fusionné | volontairement — le repli C++ n'est pas encore débranché |

Le nom de branche décrit la *feature*, pas le dépôt : c'est bien rag3db. Le
dépôt lucivy est séparé (`~/git_workspaces/lucivy`, branche `v3-recovery`).

## Ce qui est vert ✅

```
cargo test --lib                    589 passed; 0 failed; 13 ignored

Features (cargo check --lib)        E2E (--ignored --test-threads=1)
  défaut              ✅              e2e_search                  20/20
  no-default-features ✅              e2e_idempotent_registration 21/21
  bge-m3              ✅              e2e_phase0b                 14/14
  candle-wasm         ✅              e2e_result_mode             10/10
  burn-embedder       ✅              e2e_native                  11/11
  postgres            ✅              e2e_drain_unified            6/6
  wasm-emscripten     ✅              e2e_highlight_long_text      8/8
  examples            ✅              e2e_checkpoint               3/3
                                      e2e_batch_observe            2/2
                                      e2e_simple_entity           12/13
```

## Ce qui ne l'est pas

| Sujet | Détail |
|---|---|
| `simple_register_duplicate_fails` | attend une erreur sur un `register_entity` devenu **idempotent par conception**. Test obsolète depuis mai, pas une régression. À supprimer ou réécrire. |
| 4 fichiers E2E en dérive | `e2e_dataflow_observe`, `e2e_generic_search`, `e2e_search_queue`, `e2e_undo` ne compilent pas. Vraies ruptures d'API héritées de la migration luciole (mai) : `tokio::sync::Mutex` vs `std::sync::Mutex`, variantes `PortValue::{Results,Children,Meta,Query}` disparues, `Arc<dyn Trait>` n'implémentant plus le trait. **Hors périmètre FTS**, jamais traité. |
| Repli C++ | toujours branché volontairement (voir doc 12) |

## Ce qui a été livré aujourd'hui

**Migration FTS lucivy v3** — étapes 0 à 5 de la passation (doc 04). L'indexation,
la recherche, les suppressions, les mises à jour et la réindexation passent par
`ShardedHandle` en Rust direct. Le chemin C++ subsiste en repli.

**Embedder burn** — `BurnBgeM3Embedder` derrière tes trois traits, modèle généré
par burn-onnx, poids publiés sur
[Lucie666/bge-m3-burnpack](https://huggingface.co/Lucie666/bge-m3-burnpack).
Parité contre candle : dense cosinus **1.00000000**, sparse mêmes token_ids.

**Réparations d'infrastructure**, toutes découvertes en chemin :

- build natif mort sur GCC 13+ (`<cstdint>` transitif) — réglé par un flag, pas
  613 éditions
- dépendances cxx manquantes sur 8 cibles CMake — course intermittente
- submodule `ld-lucivy` épinglé sur le v3 (il pointait 158 commits en arrière,
  dans un état incohérent)
- `postgres`, `wasm-emscripten` et les 3 exemples réparés (cassés depuis mai)
- **workflow CI créé** : il n'en existait aucun pour rag3weaver, les 29
  workflows étant hérités de kuzu. C'est la cause racine de 3 mois de dette
  invisible.

## Ce que la journée a révélé sur l'état réel du projet

**Le C++ est gelé de fait.** Dernier commit par extension : vector 2 mars, geo
1er mars, lucivy_fts et sparse_vector 15 mars. Le delta sur le cœur kuzu depuis
le rename est de **25 fichiers, +519/−14**. Tout le travail depuis mi-mars est
en Rust.

**Trois chantiers en vol laissent chacun un doublon** : migration luciole (2
moteurs de DAG), abstraction multi-backend (2 chemins de search), migration FTS
(repli C++). Voir doc 12.

**Postgres compile de nouveau mais n'a jamais été exécuté.** Zéro test
d'intégration ; `docker/docker-compose.supabase.yml` n'est utilisé par rien.

**`geo` n'a jamais tourné end-to-end.** Deux bugs vérifiés : `columns` vide passé
au bind data (`query_spatial_index.cpp:101`), et `update()` qui supprime le point
sans jamais le réinsérer (`rtree_index.cpp:126-137`). Activé dans tous les builds
natifs, utilisé par personne.

## Environnement (a changé)

Machine AMD : **2× Radeon AI PRO R9700** (Navi 48, gfx1201, RDNA4), ROCm 7.2.4,
**aucun CUDA**. `candle 0.8.4` n'a pas de backend ROCm — la feature `cuda` est
morte ici. D'où le basculement vers burn/Vulkan.

Débit mesuré de BGE-M3 sur burn : plateau à **~7 500 tokens/s** (batch 16 ×
seq 128). Cohérent avec le matériel — un A100 fait 8× mieux, ce qu'expliquent
l'écart de puissance brute, le fp16 et la flash-attention.
