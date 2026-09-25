# Reprise de la branche `mtg-experiments` : ce qu'a fait Codex (19 → 23/09)

*Rédigé le 25/09/2026 lors de la migration vers la nouvelle machine, pour les prochaines sessions de code.
Le travail a été fait par Codex, donc l'historique Claude Code n'en contient rien. Ce document en tient lieu.*

## TL;DR

- Codex a mené de bout en bout un cas d'usage **Magic: The Gathering Arena**. Il a ingéré la collection et le catalogue complet, fait de la recherche composée (plein texte, vecteurs et graphe), construit des decks, puis un agent avec validation. Le cas d'usage **a bien fonctionné** et il a servi de banc d'essai réaliste.
- Ce travail a fait apparaître **4 correctifs du moteur C++**, plusieurs **correctifs dans rag3weaver** et de **nouvelles briques génériques** : payloads typés imbriqués, backends déclaratifs, search chain, validation Rhai, chat. Le code Rust est **presque entièrement générique** ; le vocabulaire MTG se limite à quelques tests, templates et scripts (voir la section Dette).
- **Tout est dans un seul commit** `ab95c3a2d` (140 fichiers, +11 376/−221) sur la branche `mtg-experiments`, qui **n'est pas poussée**.
- `master` a aussi **2 commits locaux non poussés** (mesure de la fenêtre de refus du lecteur).

## État git au 25/09

| Élément | État |
|---|---|
| `master` | 2 commits en avance sur `origin/master` : `945cd6706` et `20a8f6ee8`. **Non poussés.** |
| `mtg-experiments` | `master` + 1 commit `ab95c3a2d`. **N'existe pas sur origin.** |
| `extension/rag3weaver/docs/23-09-2026/` | non suivi (2 docs de decks Paradox Engine / Bant) |
| `experiments/mtga/` (604 Ko de code Python, backend, rag3bridge) | **ignoré par git** (`.gitignore`). Il n'existe que sur le disque (restauré depuis la clé USB). |
| `experiments/mtga/data/`, `.venv` | **absents** (exclus de la sauvegarde) : bases, snapshots, decks générés, poids de modèles |
| `follows.csv`, `user.csv`, `user.parquet` à la racine | restes de démo sans lien avec MTG (sortie d'un `COPY TO`, 7/09). Ils peuvent être supprimés. |
| Worktrees `rag3db-embarquements`, `rag3db-recherche`, `rag3db-lifecycle` | dossiers absents (non sauvegardés). Leurs branches sont sur origin. Il faut faire un `git worktree prune`. |

## 1. Correctifs du moteur C++ (tous génériques)

Aucun n'est propre à MTG : le cas d'usage les a révélés parce qu'il filtre de grandes listes de structs (`collect({...})`). **À extraire dans un commit dédié sur master** (cherry-pick de `src/` et `test/`).

1. **Propagation de l'état aux champs de struct dans les lambdas** (`lambda_evaluator.cpp:43`, `list_slice_info.cpp:69-84`). `param->state = X` ne touchait que le vecteur parent : `x.label` lisait un enfant dont la sélection et la taille étaient périmées. On utilise maintenant `ValueVector::setState`, qui est récursif. Symptôme : résultats faux ou lectures hors bornes dans `list_filter`, `list_transform` et les quantificateurs au-delà de 2048 éléments ou après un `WHERE`. Tests : `test/test_files/function/lambda/structured_slices.test`.
2. **Quantificateurs `any/all/none/single` sur plusieurs tranches** (`quantifier_functions.cpp:17-46`). Ils suivent maintenant le protocole `ListSliceInfo` : un compte par `listPos`, accumulé dans `quantifierCounts`, et le résultat écrit à `slice->done()`. Ils gèrent aussi une liste NULL (résultat NULL) et un prédicat NULL (compté comme faux).
3. **`ParsedParameterExpression::copy()`** implémenté (avant : `KU_UNREACHABLE`). Tout lambda contenant un `$param` plantait. **Pas de test.** `serializeInternal` reste `KU_UNREACHABLE`.
4. **`StringChunkData::finalize` sûr face aux exceptions** (`string_chunk_data.cpp:240-258`). Il travaille en deux phases (on construit `newIndices`, puis on publie index et dictionnaire ensemble), ce qui supprime la corruption des chaînes si `appendString` lève `BufferManagerException` sous pression mémoire. Test : `test/storage/string_finalize_test.cpp`.

## 2. rag3weaver : correctifs du code existant (à garder)

- **`search.rs` `inline_params`** : une seule passe qui tient compte des guillemets. Elle ne remplace plus les `$` dans les littéraux ni en cascade. Les antislashs sont échappés. Le doublon de `rag3db_search_backend.rs` a été supprimé.
- **`filter.rs` et `dialect.rs`** : `HasAny/HasAll/HasNone` deviennent `list_contains` avec OR/AND, sans lambda, car les lambdas plantaient dans les graphes projetés. Les versions lambda passent en `any(v IN … WHERE …)`.
- **`catalog.rs`** : `flush_blob_store` renvoie `Result` (`CatalogError::IndexPersistence`), qui est propagé partout. Avant, l'échec était avalé. `RETURN n` sous forme de Map est déplié.
- **`dataflow/runtime.rs`** : un fan-in attend **tous** ses producteurs, avant il partait avec des données partielles. ⚠️ Une branche conditionnelle qui ne termine jamais bloque désormais le nœud en aval.
- **`FuseResultsNode`** : identité `(entity, uuid)`, dédoublonnage dans chaque branche avant le RRF, `top_k` avant la collecte des preuves, option `duplicates: merge|keep`.
- **`SearchSourceNode`** : un filtre malformé fait échouer la recherche au lieu de l'élargir silencieusement.
- **`agent.rs`** : sortie d'outil bornée (`ToolOutputLimit`). **`openai_llm.rs`** : canal `on_reasoning` (`reasoning_content`).
- **`fts_handle.rs`** : index Lucivy sans positions (`Catalog::set_fts_positions`). **`build.rs`** : `-rdynamic` en build statique. **`render_nodes.rs`** : gabarit `tree` et filtres minijinja.

## 3. rag3weaver : nouvelles briques génériques

- **Payloads typés imbriqués** : `FieldType::List/Struct`, DDL `T[]`/`STRUCT(...)`, `CypherValue::Typed`, filtres `Path` et `Nested`, et `json_schema.rs` (JSON Schema vers types).
- **Composable results** (`dataflow/composable_results.rs`) : nœuds Select, Related, Intersect et Label.
- **Search chain** (`dataflow/search_chain.rs`) : un `SearchProgram` avec les verbes `search/select/from/follow/where/within/fuse/page/render`, compilé vers un DAG dataflow. Exemple : `examples/search_chain_plan.rs <backend.json> <program.json>`. **Pas encore exposé en MCP.**
- **Validation Rhai** : `harness.rs` (`RhaiLimits`, `ValidationReport`), `RhaiNode`, `ValidationRuleNode` et `ValidationMergeNode`. L'agent relance le modèle tant que `submit_result` n'est pas accepté (`task_accepted`). `BUILTIN_NODE_COUNT` passe de 32 à 39.
- **Backends déclaratifs** (`backend.rs`, `backend_nodes.rs`) : un manifeste JSON déclare schéma, entités, relations et outils (des graphes Mermaid `.mmd`), avec un harness `before/after/on_accept` et des `WritePolicy`. Binaire `rag3weaver-backend backend.json [--describe]` (JSONL sur stdio : `describe/call/shutdown`). Pont MCP : `scripts/serve_backend_mcp.py` (`mcp==1.30.0`).
- **Embedder HTTP compatible OpenAI** (`http_embedder.rs`, `embeddings.provider: compatible`).
- **Chat** : `chat.rs` + binaire `rag3weaver-chat chat.json [--demo]` (feature `openai-llm`). `scripts/chat_app.py` fournit un TUI curses et une interface web locale (`ui/chat`).
- **Templates** : backends `notebook` et `validated-result` (avec exemple complet de règles de deck dans `examples/deck/`), app `notebook`, outils génériques `search_structured`, `select_structured`, `search_dense_related`, `ingest_snapshot` et `link_snapshot`.
- **Dépendances** : `jsonschema 0.51`, `rhai 1.26.1`. **Lucivy passe de 4.0.1 à 4.3.0.**

## 4. Le cas d'usage MTG, en bref

Voir `docs/19-09-2026/`, `docs/20-09-2026/` et `docs/23-09-2026/` pour le détail.

- **19/09** : modèle `Card → Ability → Mechanic`. Ingestion de la collection (5 380 impressions, 10 352 exemplaires). Les sélections exhaustives n'ont ni omission ni faux positif (71/71 Trésors). Outils MCP `describe_backend`, `validate_search` et `run_search`.
- **20/09** : 13 decks construits via le MCP (Eldrazi, Rakdos, grenouilles, Mycotyrant…), avec des scripts `build_*_engine.py`. Catalogue complet et planification des jokers (`plan_wildcards.py`). Lucivy 4.3.0. Agent comparé sur modèles locaux (Bonsai 2 27B, Qwen3-Coder-30B) : **aucun deck conforme produit par l'agent seul**. Le harness de validation Rhai refuse correctement ces decks.
- **23/09** (non commité) : deck Paradox Engine en 60 cartes, et variante Bant de 70 cartes tirée d'un deck utilisateur lu dans `Player.log`.
- **Licences** : `mtga-reader` est sous GPL-3.0. Les conditions de Wizards sont à clarifier avant toute diffusion.

### Reproduire

- Build : `cmake --build build/lecteurs-csv --target rag3db_shared`, puis `cargo build --features daemon,rag3db-native` (plus `openai-llm` pour le chat). `LD_LIBRARY_PATH=build/lecteurs-csv/src`. Tests avec `TMPDIR=/var/tmp`.
- Services : démon d'embeddings **BGE-M3 (1024 dimensions) sur `127.0.0.1:7878`** et LLM compatible OpenAI (llama.cpp, `127.0.0.1:8080/v1`).
- MCP : `bash experiments/mtga/scripts/serve_engine_mcp.sh`, avec `RAG3DB_BUFFER_POOL_SIZE` (15 Gio), `RAG3DB_MAX_DB_SIZE=64Gio` et `RAG3WEAVER_RENDER_TEMPLATES`.
- Données : API locale sur `:8731` (`serve.sh`), collection via `mtga-reader`, decks via `Player.log`, textes via les SQLite du client Arena. Pipeline : `prepare_engine_*.py`, puis `engine_collection.py --ingest`, puis `sync_engine.sh`.
- ⚠️ **Sur cette machine, `experiments/mtga/data/` et `.venv` n'existent pas** : il faut recréer le venv, recapturer depuis Arena et réingérer.

## 5. Bugs ouverts

1. **Persistance des `abilities` imbriquées** : après réouverture, des textes sont rattachés au mauvais élément (46 impressions sur 1 118). Seuls les exports ont été réparés, pas le moteur. **Bloquant** pour les filtres et rendus sur ce champ.
2. **SIGSEGV avec un buffer pool de 1 Gio** : il se reproduit encore après le correctif `finalize`. Contournement : pool de 8 à 15 Gio.
3. **Arrêt du serveur MCP** : le SDK le tue au bout de 2 s alors que la fermeture en prend environ 3 s, ce qui corrompt le WAL. C'est contourné par `shutdown`/`close_backend`. Ne jamais ouvrir deux hôtes sur la même base, et ne pas supprimer les WAL.
4. La synchronisation ne fait que des upserts : les cartes absentes d'un nouveau snapshot ne sont pas supprimées.
5. Hors périmètre : `list_filter.cpp:113` utilise `inputVector.isNull(i)` au lieu de `pos` (bug voisin, non corrigé).

## 6. Dette technique

Ce ne sont pas des problèmes bloquants, mais des choses à rendre génériques plus tard.

- **Un commit monolithique** mélange correctifs C++, fonctionnalités Rust, docs et gitignore. Il faudrait au minimum extraire les correctifs moteur (§1) sur master.
- **`experiments/mtga/` hors git** : c'est le code de la démonstration (backend, rag3bridge, scripts de decks), sans aucune sauvegarde versionnée. Il faut le versionner à part (dépôt privé ?) ou retirer la règle du `.gitignore` et ignorer seulement `data/`.
- **Parties propres à MTG** :
  - `templates/tools/search_related_scoped.mmd`, dont les paramètres et descriptions parlent de mechanic/ability/card/deck ;
  - `tests/structured_payloads.rs::composed_magic_snapshot` (entités `Magic*`, `RAG3WEAVER_TEST_MTGA`) ;
  - la description de l'outil `save_artifact` (`chat.rs`), qui parle de « deck export ».
- **Valeurs en dur** :
  - `127.0.0.1:7878` et `bge-m3`/1024 dans le manifeste notebook ;
  - `127.0.0.1:8080/v1` et `local-model` dans `templates/apps/notebook/chat.json` ;
  - le chemin relatif `../../../../vector/build/libvector.rag3db_extension` ;
  - `scripts/test_structured_payloads.sh`, qui code en dur `experiments/mtga/data/...`, les ports 8736/7878 et `target/debug`.
  - (Aucune clé API en dur : elles passent par `api_key_env`.)
- **Incohérence** : le dialecte Postgres mappe `List/Struct` en JSONB, mais le catalogue les **refuse** hors rag3db.
- **Quantificateurs** :
  - `quantifierCounts` est un état propre aux quantificateurs, ajouté dans la structure générique `ListLambdaBindData` ;
  - `assign(2048, 0)` s'exécute à chaque évaluation pour toutes les lambdas ;
  - `(void)dataPos` ne sert à rien ;
  - un prédicat NULL est compté comme faux (pas de logique à trois valeurs).
- **Couverture de tests** :
  - aucun test pour `ParsedParameterExpression::copy` ;
  - 3 tests d'intégration `#[ignore]` qui ont besoin d'un daemon réel ;
  - les « 1 107 tests lib OK » annoncés par le commit n'ont pas été revérifiés sur cette machine.
- `ChatConfig`, `BackendManifest` et `EmbeddingService` sont en `deny_unknown_fields` : attention aux manifestes existants lors des évolutions.

## 7. Commits `master` non poussés (hors MTG)

- `945cd6706` : le test `LeRefusSeResoutParUneNouvelleTentative` (`test/api/lecteurs_concurrents_test.cpp`) suit maintenant la cadence de reprise de l'appelant Rust (5, 10, 20, 40… ms, 60 essais) et chronomètre chaque épisode de refus.
- `20a8f6ee8` : `docs/18-septembre-2026-11h00/02-la-fenetre-de-refus-du-lecteur.md`. La fenêtre de refus va de `logAndFlushCheckpoint` à `wal->clear()`. Elle est d'environ 18 ms au repos, 38 à 44 ms sous charge, avec un pic à 567 ms qui dépasse le budget de 250 ms de `read_only`. Ce n'est pas un bug moteur. **Décision à prendre** : relever le budget (par exemple 1 s), ou tester l'invariant « refusé ou lecture juste » via `read_only_patient`.

## Actions suggérées, dans l'ordre

1. Pousser `master` et `mtg-experiments` sur origin (pour sauvegarder).
2. Versionner `experiments/mtga/` (code seulement, sans `data/`).
3. Extraire les correctifs moteur (§1) sur master, et ajouter un test pour `ParsedParameterExpression::copy`.
4. Traiter le bug de persistance des `abilities` imbriquées (§5.1).
5. Rendre génériques les éléments listés dans la Dette (§6) au fil de l'eau.
