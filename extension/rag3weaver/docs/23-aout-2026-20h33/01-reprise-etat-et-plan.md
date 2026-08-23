# Doc 01 — Reprise du projet : état vérifié, environnement, plan

Date : 23 août 2026
Dernier commit du repo : **17 mai 2026** (`c60e41868`) — 3 mois d'interruption.

Tout ce qui est marqué ✅ **vérifié** dans ce doc a été confirmé par exécution ou lecture
directe du code aujourd'hui, pas repris d'un doc antérieur (plusieurs docs de mars sont périmés).

---

## 0. Ordre d'exécution proposé

| # | Chantier | Coût estimé | Pourquoi d'abord |
|---|---|---|---|
| **A** | Réparer les builds cassés (postgres, wasm, exemples) | ~1 j | Sans ça, aucun filet : chaque session recasse silencieusement |
| **B** | Remettre l'E2E en marche sur cette machine (AMD) | ~0,5 j | Prérequis de tout le reste |
| **C** | #237 — FTS `ShardedHandle` en Rust direct | ~3-5 j | Débloque le dernier E2E rouge + tue la dépendance C++ |
| **D** | Unifier les deux chemins de search | ~2 j | Rend le pipeline DAG portable Postgres |
| **E** | geo : E2E dédié + 2 bugs | ~1 j | Sortir du risque gratuit dans chaque build |

A et B sont non négociables avant de toucher au reste.

---

## 1. Où on en était — résumé factuel

### Ce qui est sain

| Check | Résultat |
|---|---|
| `cargo check --lib` (features par défaut) | ✅ **vérifié** — compile, 8 warnings |
| `cargo test --lib` | ✅ **vérifié** — **581 passed, 0 failed, 13 ignored** |
| Git | propre, seul `.gitmodules` modifié |
| TODO dans tout `src/` | **1 seul** (`catalog.rs:680`, la migration FTS) |

Le cœur — Catalog + search + ingestion sur rag3db natif — est mature et testé.

### Les trois chantiers en vol, chacun laissant un doublon

1. **Migration luciole** — phases 0→5 faites, 6-7 restantes. Bloquée sur l'absence de
   checkpoint dans luciole → **2 moteurs de DAG coexistent** :
   - Search + `GraphNode` → `execute_via_luciole()` (`catalog.rs:3004`, `graph_node.rs:210`)
   - Ingestion / migrations / `drain()` → ancien `DataflowRuntime` (checkpoint requis)
2. **Abstraction multi-backend** — `SchemaDialect` (**50 méthodes**, pas 32 comme le dit
   `docs/24-mars-2026-22h41/01`) et `SearchBackend` sont complets des deux côtés. Mais le
   backend Postgres est **bit-rotté et n'a jamais été exécuté une seule fois**.
3. **Migration FTS C++ → Rust** — **pas commencée**. `lucivy-core 2.0.0` est déjà en
   dépendance mais n'est utilisé que pour le trait `BlobStore` : aucune occurrence de
   `ShardedHandle` ni `LucivyHandle` dans `src/`.

### Le compteur qui résume la situation

Appels Cypher vers les extensions C++ restant dans `rag3weaver/src` — ✅ **vérifié par grep** :

| Extension | Call sites | Statut |
|---|---|---|
| **sparse_vector** | **0** | ✅ migration Rust terminée — **c'est le patron à suivre** |
| **lucivy_fts** | **25** | ❌ chantier C |
| vector | 14 | ⚙️ normal — passe par `SearchBackend` (pgvector côté Postgres) |

La migration sparse (`dbcf494ca`, « Phase 1 ») est le modèle exact de ce qu'il faut
refaire pour la FTS : `HashMap<String, Arc<Handle>>` dans le Catalog, handle injecté
comme service dataflow, colonnes sorties du schéma.

---

## 2. Changement d'environnement : CUDA → AMD ROCm

**C'est le point neuf de cette reprise, et il faut le traiter avant l'E2E.**

### Ce que dit la machine — ✅ vérifié

```
04:00.0 VGA  AMD/ATI Navi 48 [Radeon AI PRO R9700]
07:00.0 VGA  AMD/ATI Navi 48 [Radeon AI PRO R9700]
/opt/rocm  →  présent (amdgcn, bin, hiprand)
/usr/local/cuda  →  absent
```

Deux R9700, ROCm installé, aucun CUDA.

### Le blocage réel — ✅ vérifié

`candle-core 0.8.4`, features disponibles :

```
accelerate · cuda · cudnn · metal · mkl
```

**Il n'y a pas de backend ROCm/HIP dans candle 0.8.** Donc :

- `--features cuda` (`Cargo.toml:55`) est mort sur cette machine.
- `run_e2e.sh` active **cuda par défaut** et exporte `/usr/local/cuda/{bin,lib64}` +
  `CUDA_ROOT`. Il faut passer `--no-cuda`, ou mieux : inverser le défaut.

### Précision importante sur « rendre le sparse compatible »

La distinction compte pour ne pas chercher au mauvais endroit :

| Composant | Nature | AMD ? |
|---|---|---|
| **Index sparse** (`sparse_vector/rust`, portage Qdrant) | Rust pur, CPU, pruning WAND | ✅ **aucun problème** — rien à faire |
| **Embedder sparse** (`bm42_embedder`, `bge_m3_embedder`) | candle → GPU | ❌ **c'est ici que ça bloque** |
| Index HNSW (`vector`) | C++ + simsimd, dispatch SIMD runtime | ✅ AVX2/AVX512 OK sur AMD |
| Suffix FST lucivy | Rust, AVX2 dans `bitpacker` | ✅ OK sur AMD |

Autrement dit : **l'index sparse n'a rien à voir avec CUDA**. Ce qui doit être revu, ce
sont les *embedders* — le chemin `CandleDualEmbedder` / `BgeM3Embedder` / `Bm42Embedder`.

### Trois options pour l'inférence

| Option | Effort | Verdict |
|---|---|---|
| **CPU candle** (`--no-cuda`) | nul | ✅ **à faire tout de suite** pour débloquer l'E2E. Lent mais correct — les E2E utilisent de petits corpus |
| **llama.cpp server ROCm + `CallbackEmbedder`** | ~0,5 j | ✅ **la vraie cible.** llama.cpp a un backend HIP/ROCm et Vulkan, supporte `--embedding`, et **est déjà cloné dans `git_workspaces/llama.cpp`**. Le trait `Embedder` est déjà abstrait, et les exemples `tei_reqwest` / `tei_openai` donnent le patron HTTP |
| Attendre un backend ROCm dans candle | — | ❌ pas de visibilité, ne pas parier dessus |

**Décision proposée** : `--no-cuda` maintenant pour l'E2E, puis brancher llama.cpp en
serveur d'embeddings via le `CallbackEmbedder` comme chemin normal sur cette machine.
Ça a un bénéfice de bord : ça découple définitivement le projet du couple candle+CUDA.

### Tâches

- [ ] `run_e2e.sh` : inverser le défaut CUDA → `--cuda` opt-in au lieu de `--no-cuda` opt-out ;
      ne plus exporter les chemins CUDA inconditionnellement
- [ ] `Cargo.toml:55` : la feature `cuda` est **mal formée** — elle n'active pas
      `candle-transformers` ni `tokenizers`, donc `--features cuda` seul ne compile pas.
      À corriger ou à documenter comme non-standalone
- [ ] Vérifier que les E2E passent en CPU (durée à mesurer — c'est le vrai inconnu)
- [ ] Brancher llama.cpp ROCm derrière `CallbackEmbedder`, avec un exemple dédié

---

## 3. Chantier A — Réparer les builds cassés

**Sur les 4 cibles du projet, une seule compile aujourd'hui : natif rag3db.**
Les casses sont toutes des dommages collatéraux de la migration async→sync du 17 mai,
invisibles parce que ni `postgres` ni `wasm-emscripten` ne sont dans un CI.

### A.1 — feature `postgres` — ✅ vérifié, 2 erreurs structurelles

1. **`postgres_blob_store.rs` appelle `execute_with_params_sync()`** aux lignes
   **31, 46, 65, 79, 93**. Cette méthode **n'existe nulle part** :
   `grep "fn execute_with_params_sync" src/` → 0 résultat. Le trait `DbConnection`
   (`connection.rs:163`) n'expose plus que `execute` et `execute_with_params`.
   → **Fix** : remplacer les 5 appels par `execute_with_params`.

2. **`tokio/rt` n'est pas activé par la feature `postgres`**. Dans `Cargo.toml`,
   `tokio = { version = "1", features = ["sync"] }` et `rt` n'est ajouté que par
   `wasm-emscripten`. Or `postgres_connection.rs:233` fait `Handle::current().block_on(...)`.
   → **Fix** : ajouter `"tokio/rt"` (voire `rt-multi-thread`) à la feature `postgres`.

3. **Fragilité de fond** : `Handle::current().block_on()` panique s'il est appelé depuis
   un thread worker tokio, et `Handle::current()` panique s'il n'y a pas de runtime du tout.
   → À revoir : runtime dédié possédé par `PostgresConnection` plutôt que `Handle::current()`.

4. **Zéro test d'intégration Postgres** — les 14 fichiers de `tests/` sont tous
   `#![cfg(feature = "rag3db-native")]`. Le `docker/docker-compose.supabase.yml`
   (pgvector/pg17, port 5433) n'est utilisé par rien.

**État réel du backend Postgres : ~70 % écrit, 0 % exécuté.** Le design est cohérent
(50 méthodes de dialect, 6 de SearchBackend, `_row_id BIGSERIAL` comme offset, `unnest`
au lieu d'`UNWIND`, join tables pour les relations). Il n'a juste jamais tourné.

### A.2 — feature `wasm-emscripten` — ✅ vérifié, 4 `block_on` obsolètes

`futures::executor::block_on()` appelé sur des fonctions devenues **sync** le 17 mai :

| Appel | Cible | Signature réelle |
|---|---|---|
| `wasm_ffi.rs:916` | `catalog.initialize()` | `pub fn initialize` (`catalog.rs:326`) |
| `wasm_ffi.rs:1295` | `cat.search(...)` | `pub fn search` (`catalog.rs:2353`) |
| `wasm_ffi.rs:1583` | `catalog.count(...)` | `pub fn count` (`catalog.rs:1768`) |
| `catalog.rs:2128` | `self.drain()` | `pub fn drain` (`catalog.rs:2068`) |

`block_on` sur un non-`Future` ne compile pas. → **Fix** : retirer les 4 wrappers.

Au passage : la surface FFI WASM est riche (16 symboles) mais il **manque `search` sync,
`update`, `delete`, `register_entity`**.

### A.3 — `examples/` cassés

Les 3 exemples (`candle_local`, `tei_reqwest`, `tei_openai`) font `.await` sur des traits
devenus sync. → À convertir. Ils redeviendront utiles avec le chemin llama.cpp (§2).

### A.4 — CI : la vraie leçon

Ajouter au CI, même en `cargo check` seul :

```
cargo check --lib                              # déjà vert
cargo check --features postgres
cargo check --features wasm-emscripten
cargo check --examples
```

C'est l'absence de ces 3 lignes qui a laissé pourrir 3 mois de dette invisible.

---

## 4. Chantier B — Reprendre la main sur l'E2E

### B.1 — L'état du terrain

**Aucun build C++ n'existe** : pas de `build/`, pas de `librag3db.so`. La première
exécution demandera un build cmake complet (compter ~10 G de disque, `build.sh` le
vérifie explicitement).

### B.2 — `run_e2e.sh` — comment il marche

```bash
cd extension/rag3weaver

./run_e2e.sh --build-only        # build cmake isolé dans build/native-test
./run_e2e.sh --no-cuda --summary # tous les E2E, table par suite
./run_e2e.sh --no-cuda phase0    # filtre sur un nom de test
./run_e2e.sh --test e2e_search   # un seul fichier
```

Ce qu'il fait :
- build isolé dans `build/native-test` (séparé de WASM/nodejs), extensions
  `vector;lucivy_fts;sparse_vector;geo`, `BUILD_SHELL=FALSE`, `BUILD_TESTS=FALSE`
- skip le build si `librag3db.so` existe déjà (`--build` pour forcer)
- features cargo : `rag3db-native,candle-embedder,bge-m3[,cuda]`
- exporte `RAG3DB_SHARED=1`, `RAG3DB_LIBRARY_DIR`, `RAG3DB_INCLUDE_DIR`, `RAG3DB_ROOT`,
  `LD_LIBRARY_PATH`
- lance `cargo test --test e2e_* -- --ignored --nocapture` (tous les E2E sont `#[ignore]`)
- `--summary` : agrège les `test result:` en une table par suite

### B.3 — Inventaire des tests

- **152 tests E2E déclarés** dans 14 fichiers (✅ vérifié par grep), tous `#[ignore]` +
  `#![cfg(feature = "rag3db-native")]`
- Dernier score connu (17 mai) : **106/108**, avec
  - 1 échec : BM25 `contains` — dépend de champs `._ngram` non alimentés par l'extension C++,
    **sera résolu par le chantier C**
  - 1 test obsolète (`simple_register_duplicate_fails`, idempotent par design)
- **4 fichiers ne compilaient pas** au 17 mai : `e2e_dataflow_observe` (7),
  `e2e_generic_search` (8), `e2e_search_queue` (5), `e2e_undo` (4) = **24 tests**

### B.4 — `build.sh` est périmé — ✅ vérifié

```bash
EXTENSIONS="${EXTENSIONS:-tantivy_fts;sparse_vector;vector;geo}"   # ligne 17
```

`extension/tantivy_fts` **n'existe plus** — renommé `lucivy_fts` le 5 mars (`bd6691d54`).
Le script référence l'ancien nom aux **lignes 8, 10, 11 et 17**. `./build.sh` avec les
défauts ne construit donc pas la FTS.

### Tâches

- [ ] `build.sh` : `tantivy_fts` → `lucivy_fts` (4 occurrences)
- [ ] `run_e2e.sh --build-only --no-cuda` → premier build, mesurer la durée
- [ ] Faire compiler les 4 fichiers E2E restants (24 tests)
- [ ] Rejouer la suite complète en CPU, établir le nouveau score de référence
- [ ] Mesurer la durée en CPU vs l'ancien score CUDA — décider si llama.cpp devient
      prioritaire

---

## 5. Chantier C — #237 : FTS `ShardedHandle` en Rust direct

Le plan détaillé est dans `extension/rag3weaver/docs/17-mai-2026-14h13/04-rapport-final-session.md`
et l'API dans le `05-knowledge-dump.md` du même dossier. Rien à redécouvrir.

**Le patron à copier : `dbcf494ca`** (migration sparse). Étapes :

1. `HashMap<String, Arc<ShardedHandle>>` dans le `Catalog`, à côté de `sparse_handles`
   (`catalog.rs:133`), avec un `ensure_fts_handle()` calqué sur `ensure_sparse_handle()`
   (`catalog.rs:247`) — `open_with_storage` avec fallback `create_with_storage`
2. `register_entity` / `register_kb` → créer le handle
3. Ingestion : les nodes appellent `handle.add_document()` au lieu de `CREATE_LUCIVY_INDEX`
4. `FlushNode` (`record_nodes.rs:2778, 2809`) → `handle.commit()` au lieu de `FLUSH_LUCIVY_INDEX`
5. Search : `handle.search(&QueryConfig, top_k, None)` au lieu de `QUERY_LUCIVY_INDEX`
   (`search.rs:1752, 1825, 2077`)
6. Sortir les colonnes FTS du schéma (`schema.rs:447-474`), comme l'a fait sparse

**25 call sites** à traiter. Points d'attention notés au 17 mai :
- `ShardedHandle` utilise `global_scheduler()` de luciole → s'assurer qu'il est initialisé
- Le stockage passe par `ShardStorage` (`FsShardStorage` ou `BlobShardStorage`) — donc le
  même `BlobStore` que sparse, donc portable Postgres gratuitement

---

## 6. Chantier D — Unifier les deux chemins de search

**Le problème** — ✅ vérifié : il existe deux implémentations divergentes.

| Chemin | Fonctions appelées | Portable Postgres ? |
|---|---|---|
| `Catalog::search()` (`catalog.rs:2353`) | `search_vector_via_backend`, `search_sparse_via_backend`, `resolve_vector_chunks_with_dialect`, `enrich_results_with_data_via_backend` | ✅ oui |
| Generic search nodes (`generic_search_nodes.rs:21-25`) | `search_vector()`, `search_sparse()`, `resolve_vector_chunks()`, `enrich_results_with_data()` — avec `&conn` brut | ❌ **non, rag3db-only** |

Seul `Catalog::search()` a les diagnostics, `ResultMode`, la pagination et `SourceResolved`.

**Fix** : faire pointer les nodes du DAG sur les fonctions `_via_backend`, puis supprimer
les legacy. Ça rend le pipeline dataflow portable et supprime la divergence sémantique.

### Code mort à supprimer au passage — ✅ vérifié

- **`src/fusion.rs` en entier** (163 LOC) — `boost_fuse` / `weighted_fuse` / `rrf_fuse`
  ont **0 callsite** hors de leurs 11 propres tests. La vraie fusion est
  `search::fuse_results()` (`search.rs:2327`). Aucun warning parce que `lib.rs:34` le
  déclare `pub mod fusion` → il est dans l'API publique
- `search_vector_bruteforce()` (`search.rs:1572`) — 0 callsite, `#[allow(dead_code)]`
- `resolve_chunk_results_via_backend()` (`search.rs:956`) — 0 callsite
- `search_bm25_raw()` + `resolve_bm25_to_chunks()` — remplacés par `search_bm25_chunked()`
- `catalog.rs:3313` — `services` construit et jamais utilisé

### Note sur le bridge luciole

`LucioleNodeAdapter` mappe **tous les ports sur `PortType::Any`** (`luciole_bridge.rs:47-54`) :
le type-checking d'arêtes est perdu dès qu'on passe par luciole, il ne survit que dans
`DataflowGraph::connect()` en amont. À garder en tête si un bug de câblage apparaît.

---

## 7. Chantier E — geo : lui donner un E2E et fermer les 2 bugs

geo était une ambition d'une seule journée (1er mars, `9433ece3d`) — R-tree N-dim maison,
22 fonctions scalaires, quaternions/OBB/frustum, zéro dépendance externe, WASM-compatible.
La couche math et le R-tree en mémoire sont solides (**41 tests unitaires**). L'intégration
base de données ne l'a jamais été.

### Les deux bugs — ✅ vérifiés par lecture directe

1. **`query_spatial_index.cpp:101`** — `binder::expression_vector columns;` est déclaré
   **vide** puis passé au bind data ligne 104, alors que `tableFunc` écrit dans les value
   vectors 0 et 1. Le HNSW, lui, remplit explicitement ses 2 colonnes
   (`query_hnsw_index.cpp:145-147`). Signature d'un chemin jamais exécuté end-to-end.

2. **`rtree_index.cpp:126-137`** — `update()` fait `rtree_->remove(nodeOffset)` dans
   **les deux branches** du `if`, sans jamais réinsérer :

   ```cpp
   if (propertyVector.isNull(valuePos)) {
       rtree_->remove(nodeOffset);
   } else {
       // The full coords are not available here, so just remove for now.
       rtree_->remove(nodeOffset);
   }
   ```

   → **toute UPDATE sur une coordonnée efface définitivement le point de l'index.**

### Autres manques

- Pas d'intégration à l'optimiseur `INDEX_SCAN` (contrairement aux 3 autres extensions) :
  `grep "isIndexScanPredicate" extension/geo/` → 0 résultat
- `searchFrustum` (`rtree.cpp:705-725`) : scan linéaire de toutes les feuilles, aucun élagage
- `searchOBB` / `searchFrustum` hard-limités à `dims == 3` malgré le discours N-dim
- Mode bbox : `distance = 0.0` en dur (`query_spatial_index.cpp:172-175`)
- Bulk load absent : construction par `nodeTable.lookup()` un tuple à la fois

### Comment câbler l'E2E — ✅ vérifié

Le harness existe déjà, geo ne l'utilise simplement pas.

- Les fichiers `.test` sont exécutés par `test/runner/e2e_test.cpp`
  (`add_rag3db_test(e2e_test e2e_test.cpp)`), qui scanne un répertoire pilotable par la
  variable d'environnement **`E2E_TEST_FILES_DIRECTORY`** (défaut `test/test_files`).
- Le modèle est `extension/vector/test/test_files/*.test` (8 fichiers) :

  ```
  -CASE NullVecs
  -LOAD_DYNAMIC_EXTENSION vector
  -STATEMENT CREATE NODE TABLE embeddings (id int64, vec FLOAT[8], PRIMARY KEY (id));
  ---- ok
  -STATEMENT CALL QUERY_VECTOR_INDEX(...) RETURN node.id ORDER BY distance;
  -CHECK_ORDER
  ---- 5
  998
  ```

- `extension/geo/test/CMakeLists.txt` ne linke aujourd'hui que
  `$<TARGET_OBJECTS:rag3db_geo_index>` — donc **aucune couverture** des fonctions scalaires,
  du binder, du catalogue, de la persistance disque ni des table functions.

**À faire** :
- [ ] Créer `extension/geo/test/test_files/` avec au minimum : create index → insert →
      KNN → radius → bbox → OBB → update → delete → drop
- [ ] Attention : `build.sh` et `run_e2e.sh` passent `BUILD_TESTS=FALSE` → le runner
      `e2e_test` n'est pas construit. Il faut un build avec `BUILD_TESTS=TRUE` pour ces tests
- [ ] Fixer les 2 bugs (le `columns` vide tombera dès le premier test)

### Décision alternative

Si geo n'est pas une priorité : **le sortir des extensions par défaut**
(`extension/extension_config.cmake:6` l'active dans tous les builds natifs). Le laisser
activé et jamais testé, c'est du risque gratuit dans chaque build. Le garder demande un
E2E ; ne pas le garder demande une ligne. Ce qu'il ne faut pas faire, c'est le statu quo.

Note : **rag3weaver ne l'utilise nulle part** (`grep "geo_|spatial|rtree"` → 0 résultat).

---

## 8. Annexe — chiffres corrigés

Plusieurs docs antérieurs sont périmés. Valeurs mesurées le 23 août 2026 :

| | Valeur | Source périmée |
|---|---|---|
| Tests lib | **581 passed + 13 ignored** | doc 24-mars : 591 ; README : 539 |
| Tests E2E déclarés | **152** dans 14 fichiers | doc 17-mai : ~108 |
| Nodes dataflow | **26** | README : 22 |
| Méthodes `SchemaDialect` | **50** | doc 24-mars : 32 |
| Delta C++ sur le cœur kuzu depuis le rename | **25 fichiers, +519 / −14** | — |
| Commits sur `src/` core en 2026 | **6**, tous entre le 8 fév et le 5 mars | — |

**Docs à marquer périmés** : `docs/24-mars-2026-22h41/01` (annonce le multi-backend
« terminé » — c'était vrai avant la migration sync du 17 mai).

### Gel de fait du C++

Dernier commit par extension : vector **2 mars**, geo **1er mars**, lucivy_fts **15 mars**,
sparse_vector **15 mars**. Tout le travail depuis mi-mars est en Rust dans rag3weaver.
Le fork kuzu est déjà gelé dans les faits — il reste à l'acter.
