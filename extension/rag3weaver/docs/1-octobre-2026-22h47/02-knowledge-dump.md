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

### 3.7 Backend déclaratif, harnais, chat

*Rédigé par la session qui a construit la partie du 27 septembre (Products
Experiments), sur la base de Codex du 19-20 septembre (`ab95c3a2d`).*

**Le manifeste** (`backend.rs::BackendManifest`, `deny_unknown_fields`
partout) : `version`, `name`, `database`, `embeddings`
(`{address, model, dimensions, provider: daemon|compatible, api_key_env}` :
le démon rag3weaver ou un service compatible OpenAI), `vector_extension`,
`scripts` (nom → fichier `.rhai`, **choisis par l'hôte, jamais par
l'appelant**), `entities` (`{schema: JSON Schema, config: EntityConfig,
writes: WritePolicy}`), `relations`, `tools`, `search_graphs` (outils de
recherche Mermaid ad hoc, en option), `fts_positions` (option lucivy à la
création). Chemins relatifs au dossier du manifeste.

**Un outil** (`ToolAttachment`) : `graph` (`.mmd`), `bindings` (paramètres
fixés par le manifeste, **non surchargeables par l'appelant**),
`input_payloads` (`{entity, view: identity|editable}` : le schéma d'entrée est
dérivé de celui de l'entité, sans les champs que possède le serveur),
`metadata` (ports terminaux `{node, port}` rendus à côté du résultat — c'est
par là que sort le port `meta` de `SelectRecordsNode`), `harness`.

**`WritePolicy`** (par entité) : `created_at`, `updated_at`, `revision`
(noms de champs entiers, dates en millisecondes posées **par le serveur**),
`immutable` (une ligne s'insère ou se rejoue à l'identique, jamais ne
s'écrase), `transition_dates` (`{field, to, timestamp_field}` : date posée
quand un champ passe à une valeur). Lue par les nœuds d'écriture via le
service `backend_write_policies`.

**Les nœuds d'écriture** (`backend_nodes.rs`) : `EntityRecordNode` pour un
enregistrement métier géré (identité calculée, WritePolicy) ;
`EntityBatchNode` pour les **snapshots externes** — au plus 512 objets par
appel, tout le lot validé par le JSON Schema avant d'écrire, identités
explicites, doublon dans le lot refusé ; `RelationBatchNode` — au plus 512
paires `{from, to}` d'identités stables (jamais d'uuid interne fourni par
l'appelant), relations sans propriétés. Les outils `ingest_*` / `link_*` du
manifeste MTG sont ces deux nœuds ; le pont MCP les masque (`--hide`).

**Le protocole** (`bin/rag3weaver-backend.rs`) : une requête JSON par ligne
sur stdin, une réponse par ligne sur stdout, `{ok:true, result}` ou
`{ok:false, error}`.

| op | entrée | rend |
|---|---|---|
| `describe` | — | `{name, tools:[{name, description, inputSchema}], …}` ; aussi `rag3weaver-backend backend.json --describe` hors boucle |
| `call` | `name`, `arguments` | le résultat de l'outil, voir ci-dessous |
| `journal` | `events:[…]` | écrit des événements de conversation (hôte seulement) |
| `journal_read` | `conversation`, `since_ms` | relit une conversation depuis un instant |
| `shutdown` | — | checkpoint et destruction de la base **avant** l'accusé `{closed:true}` ; EOF fait pareil |

Un `call` sur un outil harnaché rend `{stage, executed, validation:{accepted,
errors[], warnings[]}, delivery:{ok, results[], error}, result,
presentation}`. **`presentation` est le texte que l'agent doit voir**
(`backend.rs::harness_presentation`, `a59ca01de` → `2ef3017fe`) : une
livraison acceptée est rendue telle quelle (ici le texte d'import Arena),
un refus devient la liste des erreurs, les avertissements suivent. Une
présentation déjà posée par le graphe n'est pas écrasée.

**Les schémas d'entrée sont générés par le moteur** (`json_schema.rs`) :
pour `SelectRecordsNode.filter` et `SearchSourceNode.options`, l'entité est
résolue par les bindings et le schéma porte la forme du filtre, la
description des champs groupés par type avec leurs opérateurs, le
vocabulaire (`enum`, `items.enum`, `examples` du JSON Schema de l'entité) et
un exemple. Un champ inconnu est refusé avec la liste des champs filtrables
(`check_field_names`, seulement si l'entité n'est pas dérivée). Un filtre
peut combiner `must` / `should` / `must_not` dans un même objet ; `{}` vaut
« tout ». `SelectRecordsNode` : `limit` (0 = sans limite, absent = non
donné), `unfiltered_limit` (20 par défaut dans le gabarit
`select_structured.mmd`), port `meta` « N lignes affichées sur T ».

**Le harnais** (`harness.rs`, `dataflow/validation_nodes.rs`) :
`input_schema` vérifié d'abord, puis `before` (refuse : l'outil ne s'exécute
pas), `after` (valide le résultat, n'annule pas les effets), `on_accept`
(transforme et livre ; un échec est un échec de **livraison**, pas un
retour arrière). Chaque hook est un graphe `{graph, data}` ; `data` nomme
des fichiers JSON de faits, lus **au chargement du backend**. Le hook reçoit
un seul paramètre `context = {tool, arguments, result, data}`. Une règle par
`ValidationRuleNode` (`script_id`, `code`, `message`, `path`, `severity`) ;
le script rend `#{valid, params}` ou `#{checks:[…]}` ; le message est rendu
par substitution de scalaires. `ValidationMergeNode` combine `left`/`right`.
Un script en échec donne `hook_failed` sans faire taire les règles
indépendantes. Contrat de référence :
`templates/backends/validated-result/README.md`.

**Les limites Rhai** (`harness.rs::RhaiLimits`, posées par l'hôte, non
relevables par un script) : 1 000 000 d'opérations, 64 Kio de source,
16 Mio de JSON en entrée, **131 072 éléments par collection** (tableau ou
map, imbriqués compris), 1 000 ms, 32 niveaux d'appel et de profondeur.
`eval`, `import`, `export` désactivés (donc **`export` ne peut pas servir de
nom de champ** dans un script), aucune fonction d'E/S ; deux fonctions hôtes :
`json_string`, `content_hash` (blake3). Ce n'est pas un bac à sable OS ni un
plafond mémoire global.

**La préparation des faits** : les faits d'un harnais sont des fichiers
produits hors du moteur, par un script, puis référencés par `data`. Pour
MTG, `scripts/prepare_deck_harness.py` écrit `backend/harness/cards.json`
et `wildcards.json` (ignorés par git) et enregistre les scripts et les hooks
dans `backend.json`. **Piège payé** : un objet par impression dépassait le
budget de collection (27 000 impressions × 14 champs) ; `cards.json` porte
donc **une chaîne séparée par des tabulations par impression**, décodée par
la fonction `card()` de `prepare.rhai`. Les règles MTG : `deck_size`,
`sideboard_size`, `land_count`, `known_card`, `copy_limit`, `craftable`,
`wildcard_budget`, `mana_sources`, `mana_distribution`, `land_bounds`,
`commander_card`, `legendary_copies` ; seuils dans `policy.json`.

**Le contrat de complétion** (`completion_tool` dans la config de chat,
`task_accepted` dans le résultat du run) existe toujours dans l'agent, avec
au plus 1 relance si aucun outil n'a été appelé, 2 sinon. **Retiré de la
config MTG par Lucie** : `submit_deck` est un outil de vérification que
l'agent utilise quand on lui demande un deck, pas une obligation de fin de
tour.

**Le pont MCP** (`scripts/serve_backend_mcp.py`, SDK `mcp` officiel) : lance
le backend en sous-processus, une requête à la fois (verrou), et finit
l'aller-retour même si le client annule (sinon l'appel suivant lirait la
réponse du précédent). Un refus du harnais ou une livraison ratée devient
`isError` avec des diagnostics lisibles. `--hide PREFIX` (répétable),
`--fixed-format` (le client ne choisit plus `response_format` : les petits
modèles demandaient le JSON complet), `--response-format text|json`.

**Le chat** : `rag3weaver-chat <config.json>` (`agent.rs`, `chat.rs`) pilote
un LLM compatible OpenAI sur les outils du backend lancé par
`backend_command` ; `allowed_tools`, `max_iterations`, `state_dir`.
`scripts/chat_app.py config.json --binary … [--port 8740] [--web-only]`
sert `ui/chat` en NDJSON, avec un jeton d'accès (lien affiché au
lancement ; `RAG3WEAVER_CHAT_TOKEN` pour `--attach`). Un tour **continue si
la page se ferme** ; `GET /api/attach` rattache et rejoue ses événements une
fois ; `/api/history` lit le disque. La réflexion du modèle s'affiche en
direct.

**Le journal des conversations** (`e32561453` → `9ab639392`) :
`rag3weaver-chat` écrit chaque événement **à l'instant où il arrive**
(`at_ms` strictement croissant) dans `state_dir/journal/<session>.jsonl`, et
en base par un fil d'écriture (canal mpsc, joint avant la fermeture du
backend) qui appelle l'op `journal` : `register_trace_schema`, puis
`record_runs_and_messages` par événement. Jetons et réflexion sont tamponnés
et vidés aux frontières (début d'outil, fin de tour). Un événement
`turn_end` clôt le tour.

**Un seul hôte par base**, et c'est un piège réel : le serveur MCP de
`llama-server` et `rag3weaver-chat` lancent chacun leur `rag3weaver-backend`
sur la même base. Le second échoue sur le verrou (vu : `llama-server`
redémarré n'avait plus **0 outil**). Choisir l'un ou l'autre, ou
`LLM_SERVE_NO_MCP=1`.

### 3.8 L'expérience MTG (`experiments/mtga`)

Aucun scraping : collection lue par `mtga-reader` (npm, GPL-3.0), decks par
`Player.log` (*Detailed Logs* activé dans Arena), textes et glossaire depuis
les SQLite locales du client. Tout ce qui est sous `data/` reste hors git.

L'ordre, de la capture au chat :

1. `scripts/refresh.sh` : `collect.cjs` (collection via `mtga-reader`, Arena
   lancé sous Proton) puis `build.py` → `data/arena.sqlite`.
2. `scripts/fetch_source.py` (cards, decks, mechanics) depuis l'API locale
   (`serve.sh`, port 8731).
3. `prepare_engine_collection.py`, `prepare_engine_catalog.py`,
   `prepare_engine_relations.py`, `prepare_engine_render.py` : écrivent les
   schémas et enregistrent entités et outils dans `backend/backend.json`. La
   fiche de carte est **une seule** pour collection et catalogue
   (`card_rows.py`) ; `OwnedCard` = le catalogue restreint aux impressions
   possédées. Faits de capacités (`ability_facts.py` : coûts, déclencheurs,
   effets, restrictions dont mana conditionnelle) et vocabulaire déclaré
   dans le schéma (`vocabulary.py`) : c'est ce qui alimente les `enum` des
   filtres générés.
4. `engine_collection.py --ingest --ingest-relations --ingest-catalog
   --ingest-catalog-links` (ou `sync_engine.sh`) : ingestion par les outils
   `ingest_*` / `link_*`. **À lancer détaché** (`setsid nohup … & disown`),
   copie reflink de la base avant.
5. `prepare_deck_harness.py` : faits et hooks du harnais.
6. Servir : `serve_engine_mcp.sh` (pour `llama-server`, via
   `~/.config/llm-serve/mcp.json`) **ou** `engine_backend.sh` (pour
   `rag3weaver-chat`, `chat/chat.json`), jamais les deux.

Tests : `scripts/test_engine_catalog.py`, `test_engine_mcp.py`,
`test_engine_render.py` (MTG) ; `extension/rag3weaver/scripts/test_backend_harness.py`,
`test_backend_persistence.py`, `test_backend_mcp_render.py`,
`test_chat_app.py` (moteur, sur une base temporaire, sans LLM ni
embarquement réel).

Le rendu des résultats : `backend/render/magic.md.jinja` (vue de carte
compacte en anglais, champs vides omis), choisi par
`RAG3WEAVER_RENDER_TEMPLATES`. Les entités MTG sont en `contentKind: record`.

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
| `RAG3DB_SHARED=1`, `RAG3DB_LIBRARY_DIR`, `RAG3DB_INCLUDE_DIR` | à la compilation du crate : lier `build/lecteurs-csv/src` (et `LD_LIBRARY_PATH` vers le même dossier à l'exécution des binaires) |
| `RAG3WEAVER_RENDER_TEMPLATES` | dossier des gabarits de rendu d'un backend (MTG : `experiments/mtga/backend/render`) |
| `RAG3WEAVER_BACKEND_BIN` | binaire `rag3weaver-backend` utilisé par les scripts MTG (défaut : `target/release` pour le service, `target/debug` pour `engine_collection.py`) |
| `RAG3WEAVER_CHAT_TOKEN` | jeton du chat web pour `chat_app.py --attach` |
| `LLM_SERVE_NO_MCP=1` | `llm-serve` sans `mcp.json`, quand le chat tient déjà la base |

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
  (mesuré le 27 septembre : `chunk_insert` passe de 781 à 4 474 ms par lot de 512 cartes ; cause non
  trouvée). Deux suspects à séparer par `RAG3WEAVER_INGEST_PROFILE=1` :
  l'index vectoriel retiré puis reconstruit à chaque lot
  (`ajuster_l_index_pour_le_retard`), la relecture `select_node_ids`.
- La fenêtre de refus d'un lecteur pendant un checkpoint : 18 ms au repos,
  567 ms sous charge, contre un budget de reprise de 250 ms.
- L'extension `vector` doit être reliée après un rebuild de `librag3db.so`.
- **Une ligne supprimée n'est jamais récupérée**, alors qu'un `SET` récupère
  sa place. Les blobs d'index lucivy (`_index_blobs`) supprimant leurs
  anciens segments à chaque sauvegarde, la base MTG a atteint ~8 Go pour
  ~150 Mo de données vivantes. Contourné côté rag3weaver
  (`cypher_blob_store.rs`, `d1aa7d296` : suppression marquée par
  `_deleted_gen`, purge par `SET _data = vide` au-delà de
  `RAG3WEAVER_BLOB_RETENTION` générations) ; la correction dans rag3db reste
  à faire, et la base MTG existante doit être reconstruite pour rendre la place.
  Pour regarder : `FSM_INFO()`, `storage_info()` ; `SIZE()` refuse un BLOB,
  prendre `octet_length`.
- **Deux processus sur une base** : le second échoue sur le verrou — ou pire,
  si l'un est tué pendant que l'autre écrit, le WAL est à refaire. Le cas
  vécu : `llama-server` (MCP) et `rag3weaver-chat` lancent chacun leur hôte.
- **Récupérer un WAL illisible** : base arrêtée, mettre de côté la base et
  son `.wal`, restaurer la dernière copie reflink (ou la base sans le WAL,
  c'est-à-dire le dernier checkpoint), refaire les écritures perdues. Les
  ingestions MTG se rejouent (upserts).

**Le côté agent.**
- Un `kill -INT` ne traverse pas un `chat_app.py` lancé en arrière-plan :
  `kill -TERM`, qui laisse le backend fermer la base proprement.
- Une description d'outil trop générique se paie : tous les `search_*`
  disaient la même chose, et la requête se disait « nom de carte » ; le
  modèle cherchait par nom ce qu'un filtre aurait trouvé. Correction
  proposée, pas faite : une description d'entité dans le manifeste, reprise
  dans les descriptions générées.
- Les sorties d'outil se paient en contexte : texte anglais seul, champs vides
  omis, pas de ligne `Metadata: {}` vide, extraits sans répétition des champs
  (`contentKind: record`). Ne rien retirer qui porte de l'information sans
  l'accord de Lucie.
- minijinja n'a pas `.endswith` (prendre `is endingwith`) et une variable
  modifiée dans une boucle demande `namespace()`.

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

## 7. La machine

Depuis le 25-26 septembre : ROG Flow Z13 (GZ302EAC), Ryzen AI MAX+ 395
(Strix Halo), iGPU Radeon 8060S (gfx1151), 128 Go unifiés dont 112 Go
accessibles au GPU (`ttm.pages_limit=29360128`), 32 fils, CachyOS (noyau 7.2),
KDE Plasma sous Wayland, shell fish, disque en btrfs (d'où les copies
`cp --reflink=always`, instantanées).

**Les services, et leurs ports** (aucun n'est un service systemd ; tous se
lancent à la main) :

| Service | Lancement | Port |
|---|---|---|
| Modèle de langage | `llm-serve gptoss` (ou `qwen`, `qwen1m`), `llm-serve stop` / `status` ; script dans `~/.local/bin`, copie dans `~/setup/llm-serve` ; journal `~/models/llama-server.log` | 8080 (UI web + API OpenAI) |
| Démon d'embarquement | `RAG3WEAVER_EMBED_MODEL=bge-m3 RAG3WEAVER_BURN_DEVICE_EMBEDDER=igpu:0 target/release/rag3weaver-embeddings --adresse 127.0.0.1:7878` | 7878 |
| Serveur MCP MTG | lancé **par** `llama-server` depuis `~/.config/llm-serve/mcp.json` (`serve_engine_mcp.sh --hide close_backend --fixed-format`) | stdio |
| Chat web | `chat_app.py experiments/mtga/chat/chat.json --binary target/release/rag3weaver-chat --web-only` | 8740 |
| API locale MTG | `experiments/mtga/scripts/serve.sh` (uvicorn) | 8731 |

L'ordre : le démon d'embarquement d'abord (le backend s'y connecte à
l'ouverture), puis `llm-serve`, puis le chat. Le démon doit tourner pour que
le serveur MCP déclare ses outils.

`llm-serve` : gpt-oss-120b MXFP4, 128k de contexte, ~50 jetons/s en
génération ; Qwen3.5-122B-A10B (UD-Q4_K_XL), 262k, ~21 jetons/s ; `-np 1`
(une conversation à la fois). Poids dans `~/models`.

**Ce qui a dû être refait après la migration** :
- prérequis `ninja` et `vulkan-headers` ; `single_file_header` ajouté à la
  cible C++ (sans `rag3db.hpp`, le pont Rust ne compile pas) ;
- les forks burn / cubecl / cubek (`rag3weaver/pre.3`) n'ont rien eu à
  changer pour gfx1151 ;
- les données MTG reconstituées : `mtga-reader` sous Proton, *Detailed Logs*
  réactivé dans Arena, puis la chaîne du §3.8 ;
- `llm-serve` et `mcp.json` écrits pour cette machine.

**Les réglages système qui touchent le travail** :
- **Pas de veille sur secteur** (`~/.config/powerdevilrc` :
  `[AC][SuspendAndShutdown] AutoSuspendAction=0`, couvercle = écran éteint).
  Le 27 septembre le poste s'est réveillé seul capot fermé puis figé en
  s2idle, et a emporté un WAL. Une longue ingestion ne se lance pas sur
  batterie.
- Chrome : `--disable-features=DbusSecretPortal` dans
  `~/.config/chrome-flags.conf` (KWallet sans portefeuille contre
  gnome-keyring : Chrome ne chargeait plus aucune page).
- `rg` de VSCodium : ouvrir le dépôt, pas tout `~` (sinon une recherche de
  `package.json` occupe 12 fils).

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

## 11. Ajouts du 1er octobre au soir — ce que la fusion et l'enquête ont appris

- **`Catalog::get` et `get_many` rendent une ligne à plat** : les champs
  déclarés et `_uuid`, sans clé `n`. Même forme sur les deux dialectes. Gardé
  par `e2e_search::get_rend_une_ligne_a_plat`.
- **`describe` d'un backend porte `capabilities`** (`journal`,
  `journal_read`). Un client n'envoie une op que si elle y est annoncée.
- **Les outils d'ingestion d'un backend passent par `Catalog::ingest_entities`**
  (`backend_nodes.rs`) : court-circuit de l'inchangé, dérivées et rattrapages
  s'appliquent aux entités d'un manifeste. Deux manques connus : un snapshot
  ne supprime pas les lignes disparues ; un lot est refusé sur une entité à
  `Lifecycle` (« bulk snapshot writes cannot bypass a lifecycle »), ce qui
  peut devenir une vérification depuis que l'ingestion garde les transitions.
- **Le harnais rend toutes les erreurs d'une étape d'un coup** : les règles
  partent en parallèle des mêmes faits et leurs rapports sont fusionnés
  (`Backend::validate_hooks` ne s'arrête pas au premier refus). Trois
  frontières seulement : un schéma d'entrée invalide ne lance pas les règles ;
  un `before` refusé n'exécute ni l'outil ni `after` ; le nœud commun de
  préparation des faits, s'il échoue, bloque tout le hook.
- **Le journal d'écriture (WAL)** : un enregistrement de plus de 4 096 octets
  était corrompu à l'écriture (`ChecksumWriter::resizeBufferIfNeeded`, tampon
  remplacé sans recopie). Un journal relu après un arrêt brutal était donc
  illisible. Correctif en cours sur `correctif-wal-enregistrements-longs`.
  Format d'un enregistrement : type (1 octet), corps, somme (8 octets) ; pas
  de longueur. Le rejeu fait une passe à blanc qui retient le dernier COMMIT.
  Rapport complet : voir le doc du correctif, `docs/` à la racine.
- **Les poids MiniLM et MiniLM multilingue** (`~/.cache/rag3weaver/minilm/`)
  sont installés depuis le 1er octobre ; sans eux, 16 tests de `e2e_search`,
  `e2e_simple_entity`, `e2e_generic_search` et `e2e_idempotent_registration`
  ne tournent pas, et les suivants tombent en « LazyLock poisoned ».
- **Fusionner tôt.** Une branche de onze jours portait une régression de
  contrat que la suite qui la gardait n'avait jamais vue. Avant de dire une
  branche prête : jouer les suites voisines **sur la branche**, et tenir sa
  ligne au journal.
- **Travailler à deux sessions sans se marcher dessus** : celle qui n'a pas
  l'arbre commite depuis un worktree détaché de `origin/master` (dans son
  scratchpad), pousse, et le retire. `git worktree add --detach`.
- **`mtg-experiments` a été réécrite le 1er octobre** pour retirer les
  trailers d'attribution : les hash d'avant ne valent plus à partir de
  `64a12b05b`. Une branche locale `sauvegarde/mtg-experiments-avant-reecriture`
  garde l'ancienne pointe.
