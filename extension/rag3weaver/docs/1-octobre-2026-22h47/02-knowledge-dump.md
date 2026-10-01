# Knowledge dump — tout ce qu'une session doit savoir pour reprendre

**1er octobre 2026.** Écrit par la session qui a porté rag3weaver du 23 août
au 18 septembre (ingestion, catalogue, repli des KB, orchestration des
sessions), juste avant une compression de son contexte. La session « Products
Experiments » (backend, harnais, chat, deck builder MTG, nouvelle machine)
l'enrichit dans les sections marquées **[À enrichir]**.

Ce doc complète les précédents sans les remplacer :
[`6-septembre-2026-21h44/03`](../6-septembre-2026-21h44/03-knowledge-dump.md)
(mesurer, le démon, les pièges du chemin d'indexation) et
[`6-septembre-2026-21h44/02`](../6-septembre-2026-21h44/02-l-architecture-actuelle.md).
Le compagnon de celui-ci est
[01 — la réconciliation des objectifs](01-reconciliation-des-objectifs.md).

**Ce qui est vérifié et ce qui ne l'est pas.** Les tailles, noms de fichiers,
features et listes de suites ont été relevés le 1er octobre sur
`mtg-experiments`. Les chiffres de mesure datent du jour indiqué à côté
d'eux, sur l'ancienne machine sauf mention. Tout ce qui concerne les cartes
graphiques de l'ancienne machine est périmé (§7).

## 1. Le projet en dix lignes

- **rag3db** : un fork de Kuzu (base de graphe embarquée, C++), avec nos
  correctifs (HNSW, lecteurs concurrents, `COPY` à sauts de ligne échappés,
  lambdas sur structs). Remotes : `origin` (le dépôt public de Lucie) et
  `vela` (l'amont qu'on suit pour le stockage et la concurrence).
- **rag3weaver** (`extension/rag3weaver`, Rust, ~90 000 lignes) : l'orchestrateur.
  Un `Catalog` au-dessus d'une base, des entités déclarées, un dataflow de
  nœuds pour ingérer et chercher, des gabarits Mermaid, un agent, et depuis
  septembre un backend déclaratif et un chat.
- **lucivy** : notre moteur plein texte (fork de Tantivy), publié sur crates.io.
- **codeparsers** : sous-module git, 12 langages, produit scopes et arêtes.
- **Forks burn / cubecl / cubek** : l'inférence GPU par wgpu (Vulkan), sans
  CUDA ni ROCm. Embedders, rerankers, OCR.
- **La vision** (Lucie, 25 août) : « un chaos contrôlé » — un agent qui vit
  dans une base, la gère, et construit des backends avec la technologie dont
  il est fait. Tout est un graphe, et un graphe est une donnée.

## 2. Carte du dépôt

| Chemin | Ce que c'est |
|---|---|
| `src/`, `extension/vector`, `test/` | le moteur C++ et ses extensions |
| `extension/rag3weaver/src/` | le crate Rust |
| `extension/rag3weaver/src/dataflow/` | runtime, nœuds, gabarits, checkpoints |
| `extension/rag3weaver/templates/` | gabarits `.mmd` : `tools/`, `backends/`, `apps/`, `entities/`, `patterns/`, `queries/`, `render/` |
| `extension/rag3weaver/tests/` | 56 fichiers, presque tous des e2e `#[ignore]` |
| `extension/rag3weaver/scripts/` | pont MCP, chat web/TUI, tests Python du backend |
| `extension/rag3weaver/docs/<date>/` | docs de session du crate Rust |
| `extension/rag3weaver/docs/optimiseur/` | docs de la session moteur burn |
| `extension/rag3weaver/docs/vision_roadmap_09_2026/` | les 16 docs de vision |
| `docs/<date>/` (racine) | docs du fork et des extensions C++ |
| `docs/journal-des-chantiers.md` | **le registre de ce qui est ouvert** — à lire en arrivant |
| `docs/builds-et-tests.md` | builds natif, Node, WASM (ancien, de février-mars) |
| `experiments/mtga/` | l'expérience deck builder ; `data/` n'est pas dans git |
| `tools/rust_api` | le crate `rag3db` (pont cxx vers le moteur) |

Binaires (`src/bin/`) : `rag3weaver-embeddings` (le démon), `rag3weaver-backend`,
`rag3weaver-chat`, `rag3daemon`.

## 3. L'architecture de rag3weaver

### 3.1 Le catalogue et les backends

`Catalog` (`catalog.rs`, ~9 900 lignes) est la façade : `initialize`,
`register_entity` / `register_relation` / `register_kb`, `create` / `link` /
`update` / `delete`, `ingest_entities`, `drain`, `rechercher`.

Un **backend** fournit cinq organes : connexion (`DbConnection`), dialecte
(`SchemaDialect`, `dialect.rs` : tout le Cypher et le SQL vivent là),
recherche (`SearchBackend`), magasin de blobs, magasin de checkpoints. Deux
existent, entiers : rag3db natif et PostgreSQL/pgvector. Neo4j est voulu en
dernier, exprès (il parle Cypher et masquerait les fuites).

**Aucune requête écrite hors du dialecte.** Un besoin nouveau = une méthode
de `SchemaDialect`, avec sa version Cypher (défaut du trait) et sa version SQL.

### 3.2 Les entités

`EntityConfig` (`config.rs`) déclare champs, `is_title` / `is_content`,
`signals` (bm25 / vector / sparse), découpe (`ChunkingConfig`), `hashsafe`
(les champs qui font l'identité : uuid déterministe), `lifecycle`, `group_by`,
`derived`, `fusion`, `checkpoint`, et depuis fin septembre `contentKind` et
`sourceLines`.

- **Entité simple** : `{E}`, `{E}_Chunk`, relation `{E}_CHUNKED_FROM`
  (chunk → parent). Les vecteurs vivent sur les chunks.
- **Entité de données seules** : ni titre ni contenu ; pas de table de
  chunks. Les nœuds ne touchent aux `_Chunk` que des entités qui en ont
  (`record_nodes::a_des_chunks`).
- **Entité dérivée** (`DerivedConfig { from, gather, render }`) : une ligne
  par racine, rendue par gabarit minijinja depuis la racine et ses voisines ;
  colonnes `_source_entity`, `_source_uuid`, `_render_hash` ; relation
  `{E}_DERIVED_FROM` (dérivée → racine) ; ensuite découpée, embarquée,
  indexée comme toute entité. `DeriveNode` (`dataflow/derive_nodes.rs`).
- **Une base de connaissances est une entité dérivée** depuis le 18 septembre :
  `derived_kb.rs` traduit `knowledge_bases` + `title_for` / `content_for`.
  Plus de `KBMetadata`, de tables `_Index`, `_IN_`, `_SOURCED_`. Schéma v7.
  Doc : [`7-septembre-2026-21h30/01`](../7-septembre-2026-21h30/01-replier-les-kb-en-entites-derivees.md).
- **Généricité** (règle de Lucie) : une organisation nouvelle se décrit dans
  `EntityConfig`, la découpe et les relations, jamais en dur pour un cas.

### 3.3 L'écriture : file, drain, disponibilités

- `create` / `link` / `update` / `delete` mettent en file dans `PendingWork`
  (`records.rs` : `entities`, `relations`, `updates`, `deletes`, `derivations`).
- `drain` bâtit un graphe (`build_ingestion_graph`) et l'exécute :
  suppressions → mises à jour → insertions → liens → dérivées.
- **Régime d'écriture** (`RegimeEcriture`) : au tick (défaut) ou par lot.
- **Disponibilités** (`disponibilite.rs`) : `DONNEE`, `PLEIN_TEXTE`, `SPARSE`,
  `DENSE`. Une écriture ou une lecture dit jusqu'où elle exige
  (`create_jusqu_a`, `Consistency`). Ce qui n'est pas fait devient une
  **dette dans la base**, pas en mémoire.
- **Fermeture** (`Catalog::fermeture`) : les tables qu'un drain d'une cible
  doit emporter, et pas une de plus. Invariant de Lucie : jamais deux
  ressources sans lien bloquées l'une par l'autre.
- **Les quatre dettes et leurs rattrapages**, toutes bornées, toutes
  retrouvées par requête sans rien garder en mémoire : découpage
  (`_chunked_hash <> _content_hash`, `rattraper_le_decoupage`), embarquement
  (marqueur par modèle, `embarquer_le_retard`, avec réclamation
  multi-processus), rendu des dérivées (`_render_hash = ''`,
  `rendre_le_retard`), et la marque d'écriture publiée par table.
- `ingest_entities` : le chemin des lots. `split_unchanged` court-circuite ce
  qui n'a pas changé et rend l'état d'avant (pour `Lifecycle`).
- **Première ingestion en masse** : table vide → `COPY` CSV des nœuds,
  vecteurs posés avec la ligne (`InsertMode::Copy`, `EmbedMode::Enrich`).
  `RAG3WEAVER_INGESTION_LIGNE_A_LIGNE=1` pour comparer.
- **Checkpoints** : fichiers MessagePack écrits par un fil de fond
  (`checkpoint_store::Spiller`), trois modes (`Full`, `Operations`, `Off`),
  par catalogue ou par entité. Undo par nœud.
- **`Lifecycle`** : machine à états déclarée par entité ; les transitions sont
  gardées à `update` (`UpdateRecordNode`) et à l'ingestion ; une naissance
  prend l'état initial ou un état déclaré.

### 3.4 La découpe et les embarquements

- `chunker.rs` : `Semantic`, `Lines` (code : 30 lignes / 1 500 caractères / 5
  de recouvrement), `Fixed`, `Markdown`. Chaque **champ de contenu** est
  découpé séparément ; `_text` seul est embarqué (le titre non).
- **Le contrat décalage → chunk** : le plein texte rend des décalages dans le
  champ, qui se résolvent en chunk. Ne pas le casser.
- **Un index, plusieurs modèles** (`embedding_storage.rs`) : colonnes
  `embedding__{slug}`, marqueur `_embed_hash__{slug}`, index
  `{table}_vec__{slug}`. Migrer de modèle = un rattrapage, sans redécouper.
- **Modèles** : granite-278m par défaut ; 107m au premier index si plus de
  ~50 000 documents ou carte faible (`embedding_choice`, pas encore
  d'appelant) ; BGE-M3 sur demande. `RAG3WEAVER_EMBED_MODEL` gagne toujours.
- **Le démon** `rag3weaver-embeddings` (`127.0.0.1:7878`) sert un modèle à
  toutes les sessions. `HttpEmbedder` pour un endpoint compatible OpenAI.

### 3.5 La recherche

- **Un seul chemin** depuis le 18 septembre : `Catalog::rechercher`, le
  lanceur composable. Le monolithe `Catalog::search` est retiré.
- Le graphe par défaut est `templates/tools/search_base.mmd` : signaux
  étiquetés (BM25 lucivy, vecteur HNSW, sparse), fusion N-aire
  (`FuseResultsNode`, RRF k = 60), rerank comme nœud, résolution au parent,
  rendu. Le graphe de stratégie instancie `search_expansion.mmd`.
- **Les poids** : une fusion déclarée sur l'entité prime ; sinon le gabarit
  (`search_base` : BM25 0,6 / vecteur 0,4). Avant : 0,3 / 0,7 en dur.
- `resolve_search_target(name)` rend la cible (tables, champs BM25, champs
  d'enrichissement). Une dérivée cherche `content` **et** `title`.
- **Plein texte branchable** : trait `MoteurTexte` (lucivy, trigramme
  PostgreSQL ; Elasticsearch et Tantivy voulus).
- **Verbes de recherche** (`dataflow/search_chain.rs`) : `search`, `select`,
  `from`, `follow`, `where`, `within`, `fuse`, `page`, `render`, compilés vers
  le même DAG. Pas encore exposés en MCP.
- **Filtres** (`filter.rs`) : `FilterCondition`, chemins imbriqués, forme
  booléenne `must` / `should` / `must_not`, champ inconnu refusé avec la liste.

### 3.6 Le dataflow

- `DataflowRuntime` exécute un `DataflowGraph` de nœuds typés par ports
  (`PortType`). 39 nœuds intégrés, 49 avec la feature `code`
  (`BUILTIN_NODE_COUNT`, à tenir à jour à chaque ajout).
- Un fan-in attend **tous** ses producteurs ; une branche conditionnelle qui
  ne termine jamais bloque l'aval.
- **Gabarits Mermaid** (`mermaid.rs`, `template.rs`, `graph_tool.rs`) : un
  graphe-outil est un `.mmd` plus des paramètres typés. `$var` nu = typé par
  inférence ; `'$var'` quoté = chaîne.
- **Outils offerts à un agent** (feature `code`) : `adopt`, `edit`, `grep`,
  `list`, `place`, `read`, `run`, `run_bg`, `schema`, `search`, `wait`.
- **Aucun Cypher pour les agents** (décision de Lucie) : une capacité qui
  manque s'ajoute à l'abstraction.

### 3.7 Backend déclaratif, harnais, chat — **[À enrichir]**

Ce que j'en sais par lecture, à corriger et compléter par la session qui l'a
construit :

- `rag3weaver-backend <backend.json>` : JSONL sur stdio, ops `describe`,
  `call`, `shutdown`, `journal`. Le manifeste déclare base, entités, scripts
  Rhai et outils ; chaque outil est un graphe `.mmd` avec bindings.
- **Harnais** (`harness.rs`, `dataflow/validation_nodes.rs`) : `input_schema`,
  puis `before` (peut refuser), `after` (valide sans annuler), `on_accept`
  (livre). Une règle par `ValidationRuleNode`, script Rhai borné, message par
  substitution. Contrat : `templates/backends/validated-result/README.md`.
- Pont MCP : `scripts/serve_backend_mcp.py` (`--hide`, `--fixed-format`).
- Chat : `rag3weaver-chat` + `scripts/chat_app.py` + `ui/chat`.
- **Un seul hôte par base** : ne jamais ouvrir deux processus sur le même
  fichier.

*À ajouter ici : le détail du protocole, les `WritePolicy`, `EntityBatchNode`,
le journal des conversations, les limites Rhai, la préparation des faits.*

## 4. Construire et tester

```bash
# Le moteur (C++). single_file_header est requis par le pont Rust.
cmake -S . -B build/lecteurs-csv -G Ninja -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE -DBUILD_EXTENSIONS="vector"
cmake --build build/lecteurs-csv --target rag3db_shared rag3db_vector_extension single_file_header -j<N>

# Le crate, depuis extension/rag3weaver
cargo test --lib --features rag3db-native,burn-embedder,burn-ocr,code,daemon -j16   # unitaires
./run_e2e.sh --test e2e_search                                                       # une suite e2e
./run_e2e.sh --test e2e_search nom_du_test                                           # un test
```

- **`run_e2e.sh` ne joue que les tests `#[ignore]`.** Un e2e sans l'attribut
  compile et ne tourne nulle part.
- **Un `cargo test` direct ne charge pas l'extension vector** : passer par
  `run_e2e.sh`. Il lie `build/lecteurs-csv` (`RAG3DB_BUILD` pour changer,
  `RAG3DB_ROOT` = l'arbre où le C++ est bâti, **jamais un worktree**).
- `TMPDIR=/var/tmp` pour les tests qui ouvrent des sockets ou supposent un
  répertoire temporaire hors git.
- Depuis le 18 septembre, `--test` s'accumule ; avant il écrasait.
- **Ne jamais forcer `RAG3WEAVER_REGIME=plein`** dans les tests.
- Jamais `-j$(nproc)` : laisser deux cœurs. `-j6` pour une reconstruction
  complète de burn (une à `-j16` a été tuée pour la mémoire).
- Tests Python du backend : `scripts/test_backend_harness.py`,
  `test_chat_app.py`, `test_backend_persistence.py`.
- PostgreSQL : `e2e_postgres` demande un serveur sur `localhost:5433`
  (`docker start rag3weaver-pg`) ; sans lui la suite s'annonce non jouée.

**Les suites qui gardent quoi** (comptes au 18 septembre, tous verts) :

| Suite | Garde |
|---|---|
| `e2e_search` (39) | la recherche hybride sur des bases traduites en dérivées |
| `e2e_simple_entity` (22) | entité simple, `Lifecycle`, court-circuit de l'inchangé |
| `e2e_code` (24), `e2e_agent_loop` (8), `e2e_symbol_search` (12) | l'agent de code, ses outils, la recherche de symboles |
| `e2e_entites_derivees` (3), `e2e_phase0b` (14), `e2e_result_mode` (10), `e2e_idempotent_registration` (22) | dérivées, bases traduites, modes de résultat, enregistrements idempotents |
| `e2e_chemin_de_masse` (4), `e2e_undo` (4), `e2e_checkpoint` | première ingestion, undo, reprise |
| `e2e_prise_atomique` (12), `e2e_scope` (9), `e2e_hnsw_scale` (13) | lecteurs concurrents, cellules org × projet, HNSW à l'échelle |
| `e2e_generic_search` (18), `e2e_catalogue_gabarits` (12), `e2e_rerank` (3), `e2e_highlight_long_text` (8) | le lanceur, les gabarits, le rerank, les surlignages |
| `e2e_burn_*` | les six modèles et l'OCR ; demandent la carte |
| `e2e_banc_*`, `e2e_mesure_ingestion_code` | des **mesures**, pas des tests : elles n'échouent pas |

**Tests à corpus vivant** : `e2e_code` indexe `src/dataflow/port.rs`, les
bancs indexent `src/`. Alourdir ces fichiers change les résultats de tests
qui n'ont rien à voir. Un type nouveau va dans un module neuf.

## 5. Les variables d'environnement

| Variable | Rôle |
|---|---|
| `RAG3DB_BUILD`, `RAG3DB_ROOT` | la bibliothèque moteur liée par les e2e |
| `RAG3WEAVER_EMBED_MODEL` | le modèle du démon ; gagne sur l'heuristique |
| `RAG3WEAVER_REGIME` | `confort` (défaut du script) ou `plein` |
| `RAG3WEAVER_BURN_DEVICE_EMBEDDER` | forcer la carte (`igpu:0` sur la nouvelle machine) |
| `RAG3WEAVER_BURN_FLOAT` | `f16` / `bf16` / `f32` ; Flex32 est le défaut, f16 pur rend de faux vecteurs |
| `RAG3WEAVER_INGEST_PROFILE=1` | profils par nœud : `[ingest-profile]`, `[copy-profile]`, `[link-profile]`, `[drain-profile]`, `[runtime-profile]` |
| `RAG3WEAVER_INGESTION_LIGNE_A_LIGNE=1` | désactive le chemin de masse |
| `RAG3WEAVER_COPY_NAISSANCES=1` | chemin de masse des lots de naissances (désactivé par défaut) |
| `RAG3WEAVER_BLOB_RETENTION` | générations de blobs d'index gardées (2) |
| `RAG3WEAVER_SANS_DEMON=1` | modèle en processus, sans le démon |
| `RAG3WEAVER_BANC_MODELE`, `RAG3WEAVER_MESURE_*` | paramètres des bancs |
| `RAG3DB_BUFFER_POOL_SIZE`, `RAG3DB_MAX_DB_SIZE` | mémoire du moteur (pool de 8 à 15 Gio pour MTG) |
| `RAG3DB_EXTENSION_REPO` | dépôt d'extensions ; sans lui `INSTALL` refuse en le nommant |

## 6. Les pièges, par famille

**Des silences qui ne coûtaient qu'une économie.** La leçon la plus répétée
du projet : un défaut dont le seul effet est une optimisation manquée n'a pas
de symptôme, jusqu'au jour où quelqu'un s'appuie dessus pour décider.
Exemples : `split_unchanged` qui sortait en silence sur une relecture ratée ;
la fusion aplatie qui éteignait les poids d'un gabarit ; `run_e2e.sh` qui ne
jouait qu'une suite sur quatre. **Règle : quand on branche une décision sur
une information existante, relire ce qu'elle fait en cas d'échec.**

**Un seuil ne se relâche pas sans preuve.** Avant d'y toucher, prouver ce
qu'il mesurait, champ à champ contre la référence.

**Le moteur.**
- Un arrêt brutal (kill -9, gel, fin de session) peut laisser un **WAL
  illisible** (`wal_record.cpp:79`). Arrêter par SIGTERM ou EOF, copie
  reflink avant une longue écriture, ingestions détachées, un seul processus.
- `COPY` refuse tout le fichier dès qu'une clé manque ou existe déjà.
- `""` est lu comme NULL par le lecteur CSV : `null_strings` dédié.
- Un `COPY` sur une table **non vide** coûte en proportion de la table
  (mesuré le 27 septembre, cause non trouvée).
- La fenêtre de refus d'un lecteur pendant un checkpoint : 18 ms au repos,
  567 ms sous charge, contre un budget de reprise de 250 ms.
- L'extension `vector` doit être reliée après un rebuild de `librag3db.so`.

**Les mesures.**
- Le banc de qualité (cosinus nu, 0,84 de MRR) mesure **l'embarqueur sans
  distracteurs**, pas la recherche. Sur le vrai corpus (4 820 scopes), la
  recherche fait 0,33 ; l'écart est la taille du corpus.
- Un HNSW reconstruit réordonne les quasi-ex-æquo : ±1 question sur 45.
- Une optimisation GPU se vérifie par l'écart absolu maximal, pas par un
  cosinus arrondi.
- Une mesure se fait seul sur la carte.

**L'outillage.**
- `pidof rag3weaver-embeddings`, jamais `pgrep -f` (il attrape son propre shell).
- Rediriger vers un fichier puis le lire ; pas de pipe filtrant en bout de
  chaîne sur une commande longue.
- Le shell est fish : pas de `$(…)` exotique ni de `--include=*.rs` nu.
- Les scripts de remplacement par ancre : toujours afficher le texte exact
  avant de remplacer, et compter les occurrences.

## 7. La machine — **[À enrichir]**

Depuis le 25-26 septembre : ROG Flow Z13, Ryzen AI MAX+ 395 (Strix Halo),
iGPU Radeon 8060S, 128 Go unifiés, CachyOS. Les modèles de langage tournent
en local sous `llama-server` Vulkan (`llm-serve` : gpt-oss-120b, Qwen3.5).
Le démon d'embarquement tourne sur `igpu:0`, environ 8 500 jetons/s.

**Périmé** : tout ce que les docs d'avant le 25 septembre disent des deux
Radeon R9700, de « la carte TV » (`07:00.0`) et de « la carte du bureau »
(`04:00.0`), et les chiffres d'indexation mesurés dessus (24,8 s pour le cœur
C++ en granite-107m).

*À ajouter ici : les services à lancer, les ports, `llm-serve`, ce qui a dû
être refait après la migration.*

## 8. Où regarder

| La question | L'endroit |
|---|---|
| ce qu'une KB devient | `derived_kb.rs::derived_config_for_kb` |
| le rendu d'une dérivée | `dataflow/derive_nodes.rs` (`render_context`, `render_fields`) |
| le graphe d'un drain | `catalog.rs::build_ingestion_graph` |
| ce qu'un drain emporte | `catalog.rs::fermeture`, `PendingWork::extraire_les_tables` |
| le court-circuit de l'inchangé | `catalog.rs::split_unchanged` |
| le chemin de masse | `catalog.rs::premiere_ingestion_possible`, `record_nodes.rs::copier_les_noeuds` |
| les rattrapages | `embarquer_le_retard`, `rattraper_le_decoupage`, `rendre_le_retard` |
| la cible d'une recherche | `catalog.rs::resolve_search_target` |
| le lanceur | `Catalog::rechercher`, `dataflow/generic_search_nodes.rs`, `templates/tools/search_base.mmd` |
| une requête SQL ou Cypher | `dialect.rs`, toujours |
| le stockage d'un modèle | `embedding_storage.rs::VectorStorage::resolve` |
| le choix du modèle | `embedding_choice`, `regime::card_class` |
| ce qu'embarque un scope | `code.rs::own_texts`, `scope_config` |
| les outils de l'agent | `code_tools.rs`, `templates/tools/*.mmd`, `agent.rs` |
| le manifeste d'un backend | `backend.rs`, `backend_nodes.rs`, `json_schema.rs` |
| les migrations de schéma | `scope.rs::SCHEMA_VERSION` (7), `catalog.rs::migrate_scope_columns`, `migrer_les_kb_v7` |

## 9. Comment on travaille

- **Langue** : commentaires, commits et docs en français ; identifiants en anglais.
- **Commits** : pas de trailer d'attribution à une IA. `git commit -- <chemins>`
  puis `git show --stat` : dans un arbre partagé, l'index est partagé aussi.
- **Docs de session** : `extension/rag3weaver/docs/<jour-mois-année-heure>/NN-titre.md`
  pour le crate ; `docs/` à la racine pour le moteur C++.
- **Le journal** `docs/journal-des-chantiers.md` : y ajouter sa ligne en
  ouvrant un chantier, la mettre à jour en finissant ou en s'arrêtant.
- **MTG** : on pousse tout sauf les données.
- **Un symptôme signalé** = enquêter et proposer des options ; un changement
  de conception attend le choix de Lucie.
- **Plusieurs sessions** : une seule orchestre ; tout `cargo` s'annonce d'une
  ligne ; la carte se prête à une session à la fois ; une branche est
  « prête » quand les suites voisines sont jouées aussi ; on fusionne en
  local, on vérifie sur le tronc fusionné, on pousse si vert.
- **Repérage** : déléguer la cartographie à des agents, ne pas lire les gros
  fichiers par tranches.
- **La mémoire des sessions** vit dans
  `~/.claude/projects/-home-lucied-git-workspaces-rag3db/memory/` ; ce qui
  doit survivre à un changement de machine va dans le dépôt.
- **Identifiants** : dans `.vault` (Vertex, Hugging Face). Une suite cloud à
  « 0 passed » est un saut à corriger, jamais une ligne verte.

## 10. Les chiffres de référence

| Mesure | Valeur | Date, conditions |
|---|---|---|
| Indexation du cœur C++ (1 642 fichiers, 20 136 chunks) | 51,5 → 24,8 s | 6-7 sept., granite-107m, ancienne machine |
| Recherche, vecteur seul, 45 questions sur `src/` | MRR 0,328, R@1 9, R@5 24 | 18 sept., granite-278m |
| Même chose, scopes filtrés sur fonctions et méthodes | MRR 0,385, R@1 12 | 18 sept. |
| Même texte que le cosinus nu, sur 4 820 scopes | MRR 0,385 | 18 sept. |
| Cosinus nu sur 67 scopes (banc de qualité) | MRR 0,84 | pas un objectif pour la recherche |
| Ingestion du catalogue MTG, ancien chemin | ~63 cartes/s, 286 s pour 17 920 | 27 sept., Strix Halo |
| Blobs d'index après purge | 29 Mo au lieu de 355 Mo | 27 sept. |
| Démon BGE-M3 | ~8 500 jetons/s (15 000 sur les R9700) | 27 sept. |
