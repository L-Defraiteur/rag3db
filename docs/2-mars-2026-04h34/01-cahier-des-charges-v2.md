# 01 — Cahier des charges v2 : Tests rag3weaver + état du projet

Mise à jour du cahier des charges original (28 février, doc 09).

---

## Inventaire complet des fonctionnalités

### A. Cycle de vie des entités — ✅ TOUT FAIT

| # | Fonctionnalité | Impl | Testé E2E | Notes |
|---|----------------|:---:|:---:|------|
| A1 | `create()` — UUID random | ✅ | ✅ | phase0_create_drain_all_field_types |
| A2 | `create()` — UUID hashsafe (dédup) | ✅ | ✅ | phase0_hashsafe_dedup |
| A3 | `drain()` — pipeline complet | ✅ | ✅ | chunk → insert → link → embed |
| A4 | `get()` / `get_many()` | ✅ | ✅ | phase0_create_drain_all_field_types |
| A5 | `exists()` | ✅ | ✅ | phase0_create_drain_all_field_types |
| A6 | `count()` | ✅ | ✅ | phase0_create_drain_all_field_types |
| A7 | `link()` — relations | ✅ | ✅ | phase0_link_relations (avec propriétés) |
| A8 | `update()` — détection changement (hash) | ✅ | ✅ | phase0_update_and_delete |
| A9 | `delete()` — cascade chunks + relations | ✅ | ✅ | phase0_update_and_delete |
| A10 | Error cases | ✅ | ✅ | phase0_error_cases |

### B. Knowledge Bases & Ingestion — ✅ TOUT FAIT

| # | Fonctionnalité | Impl | Testé E2E | Notes |
|---|----------------|:---:|:---:|------|
| B1 | KB avec `titleFor` + `contentFor` | ✅ | ✅ | phase0_initialize_with_kb_config |
| B2 | Drain avec KB → embeddings calculés | ✅ | ✅ | phase2 (MiniLM, Multilingual, BGE-M3) |
| B3 | Drain avec KB → FTS indexé | ✅ | ✅ | phase1_bm25_* |
| B4 | Chunking (text long → chunks) | ✅ | ✅ | phase2_raw_vector_pipeline vérifie chunks |
| B5 | Chunk cascade (delete parent → delete chunks) | ✅ | ✅ | `catalog.delete()` — DETACH DELETE chunks |
| B6 | Multi-KB (même entité, 2 KBs) | ✅ | ✅ | config "main" + "authors", phase0 valide metadata |
| B7 | Sparse embeddings (BM42/BGE-M3) | ✅ | ✅ | phase3, phase5 (DualEmbedder) |
| B8 | Update → re-embed si contenu changé | ✅ | ✅ | phase0_update_and_delete |
| B9 | Filter fields (String/Int64/Double dans Lucivy) | ✅ | ❌ | FilterCompiler implémenté + 25 unit tests, pas de test E2E |

### C. Search — ✅ TOUT FAIT (sauf filtres E2E)

| # | Fonctionnalité | Impl | Testé E2E | Notes |
|---|----------------|:---:|:---:|------|
| C1 | BM25 seul | ✅ | ✅ | phase1 (6 tests) |
| C2 | Vector seul | ✅ | ✅ | phase2 (12 tests, 3 embedders) |
| C3 | Hybrid dense+BM25 | ✅ | ✅ | phase4_bm25_vector |
| C4 | Sparse seul | ✅ | ✅ | phase4_sparse_only |
| C5 | 3-way hybrid (dense+BM25+sparse) | ✅ | ✅ | phase3_hybrid_3way, phase4_all_three |
| C6 | BM25 fuzzy (distance > 0) | ✅ | ✅ | phase1_bm25_split_distant_words |
| C7 | Filtres natifs Lucivy (pre-filtering) | ✅ | ❌ | `to_lucivy_json()` + unit tests, pas E2E |
| C8 | Filtres Cypher (pre-resolution IDs) | ✅ | ❌ | `FilterCompiler::split()` + allowed_ids, pas E2E |
| C9 | Filtres combinés (Lucivy + Cypher) | ✅ | ❌ | Split architecture OK, pas E2E |
| C10 | Search sur chunks → ChunkInfo | ✅ | ✅ | `search_bm25_chunked()`, `resolve_vector_chunks()` |
| C11 | Search avec `consistency: immediate` | ✅ | ✅ | Tous les tests E2E utilisent Immediate |
| C12 | Explore (search + graph traversal) | ✅ | ⚠️ | `search_with_explore()` + `explore_bfs()` implémentés, 2 tests mineurs |

### D. Fusion — ✅ TOUT FAIT

| # | Fonctionnalité | Impl | Testé E2E | Unit tests |
|---|----------------|:---:|:---:|:---:|
| D1 | RRF fusion | ✅ | ✅ (implicite) | ✅ `fuse_rrf`, `fuse_rrf_3way` |
| D2 | Weighted fusion | ✅ | ✅ (implicite) | ✅ `fuse_weighted`, `fuse_weighted_3way_scores` |
| D3 | Boost (multiplicative/additive) | ✅ | ✅ (implicite) | ✅ `fuse_boost_multiplicative`, `fuse_boost_additive` |
| D4 | Per-signal config (role, weight, normalize) | ✅ | — | ✅ config tests |
| D5 | Per-KB fusion strategy | ✅ | — | ✅ `KBConfig::fusion_config()` |

### E. Embedders — ✅ TOUT FAIT

| # | Fonctionnalité | Impl | Testé E2E | Notes |
|---|----------------|:---:|:---:|------|
| E1 | MockEmbedder | ✅ | ✅ | Phase 0-1 |
| E2 | CandleEmbedder MiniLM-L6 (384d) | ✅ | ✅ | Phase 2 |
| E3 | CandleEmbedder Multilingual-L12 (384d) | ✅ | ✅ | Phase 2 (cross-lingual FR↔EN) |
| E4 | BgeM3Embedder (1024d, dense+sparse) | ✅ | ✅ | Phase 3-5 |
| E5 | DualEmbedder trait | ✅ | ✅ | Phase 5 (single forward pass) |

### F. Highlights — ⚠️ PARTIEL

| # | Fonctionnalité | Impl | Notes |
|---|----------------|:---:|------|
| F1 | `search_bm25_raw()` retourne per-field byte offsets | ✅ | `BM25Hit.highlights: HashMap<String, Vec<(usize, usize)>>` |
| F2 | `resolve_bm25_to_chunks()` match highlights → chunks | ✅ | Calcul d'overlap byte ranges |
| F3 | `search_bm25_chunked()` pipeline optimisé | ✅ | 2 queries, highlight→chunk matching |
| F4 | **`SearchResult.highlights` champ retourné à l'API** | **❌** | **Les highlights sont calculés puis perdus** — pas de champ dans `SearchResult` |

### G. Explore — ✅ FAIT

| # | Fonctionnalité | Impl | Testé | Notes |
|---|----------------|:---:|:---:|------|
| G1 | `search_with_explore()` | ✅ | ⚠️ | search → BFS expansion |
| G2 | `explore_bfs()` bidirectionnel | ✅ | ⚠️ | outgoing + incoming relations, depth configurable |
| G3 | `ExploreOptions` (depth, top_k, relations) | ✅ | — | |
| G4 | Pruning (top_k, priorité search results) | ✅ | — | |
| G5 | `ExploreResult` (results + graph + meta) | ✅ | — | |

Tests existants : `catalog_search_with_explore_empty()`, `explore_bfs_empty_seed()` — mineurs.

### H. Event Bus — ⚠️ PARTIEL

| # | Fonctionnalité | Impl | Notes |
|---|----------------|:---:|------|
| H1 | EventBus struct (async_broadcast) | ✅ | subscribe(), emit(), overflow mode |
| H2 | `SearchCompleted` | ✅ émis | catalog.rs:1141 |
| H3 | `EntityUpdated` | ✅ émis | catalog.rs:628 |
| H4 | `EntityDeleted` | ✅ émis | catalog.rs:698 |
| H5 | **`DrainStarted`** | **❌ défini mais jamais émis** | |
| H6 | **`DrainCompleted`** | **❌ défini mais jamais émis** | |
| H7 | **`SearchStarted`** | **❌ défini mais jamais émis** | |
| H8 | **`EntityCreated`** | **❌ défini mais jamais émis** | |
| H9 | QueueEvent système (Enqueued, BatchCompleted, etc.) | ✅ émis | OperationQueue, subscribe_queue() |
| H10 | Unit tests EventBus | ✅ | 6 tests (emit, subscribe, overflow) |

---

## Extensions rag3db (hors rag3weaver)

| Feature | Status | Détails |
|---------|--------|---------|
| **SEARCH() dans WHERE** | ✅ | `MATCH (d:Doc) WHERE SEARCH(d.body, 'rust') RETURN SEARCH_SCORE()` |
| **SPARSE_SEARCH() dans WHERE** | ✅ | `WHERE SPARSE_SEARCH(d.ID, [...], [...])` + `SPARSE_SCORE()` |
| **VECTOR_SEARCH() dans WHERE** | ✅ | `WHERE VECTOR_SEARCH(d.emb, [...], 10)` + `VECTOR_DISTANCE()` |
| **INDEX_SCAN optimizer** | ✅ | Généralisé pour les 3 types d'index, `isIndexScanPredicate` flag |
| **Bridge FTS typé** | ✅ | `search_typed_with_highlights()` — zéro JSON pour SEARCH() |
| **Geo extension** | ✅ | R-tree, 5 query modes, 19 scalar functions |
| **HNSW delete/update** | ✅ | Propagation automatique au HNSW index |
| **Build infra** | ✅ | `build.sh`, default BUILD_EXTENSIONS, `add_dependencies` |
| **38 GTests** | ✅ | 24 lucivy_fts + 10 sparse_vector + 4 vector |

---

## Ce qui reste réellement à faire

### Bugs / lacunes dans le code

| # | Problème | Impact | Effort |
|---|----------|--------|--------|
| **L1** | `SearchResult` n'a pas de champ `highlights` | Les highlights BM25 sont calculés (`search_bm25_raw`) puis perdus avant d'être retournés à l'API | Petit — ajouter champ + propagation |
| **L2** | `DrainStarted`, `DrainCompleted` jamais émis | Event bus incomplet pour le drain | Petit — 2 emit() dans `drain()` |
| **L3** | `SearchStarted` jamais émis | Event bus incomplet pour la recherche | Trivial — 1 emit() dans `search()` |
| **L4** | `EntityCreated` jamais émis | Event bus incomplet pour le CRUD | Trivial — 1 emit() dans `create()` |

### Tests E2E manquants (natif)

| # | Test | Ce qu'il valide | Effort |
|---|------|----------------|--------|
| **E1** | Filtres Lucivy natifs | `filter_condition` avec category="programming" → pre-filtering Lucivy | Moyen |
| **E2** | Filtres Cypher (WHERE) | Filtres sur champs non-Lucivy (relations, NULL) → allowed_ids | Moyen |
| **E3** | Filtres combinés | Lucivy + Cypher simultanés | Petit (dépend de E1+E2) |
| **E4** | Explore avec données réelles | search + BFS depth=1,2, vérifie nœuds + arêtes | Moyen |
| **E5** | Highlights retournés | Après fix L1 : vérifier que `SearchResult.highlights` contient des byte offsets valides | Petit |
| **E6** | Chunking explicit | body > maxSize → vérifier chunks créés, search retourne ChunkInfo, delete cascade | Moyen |

### Tests WASM

| # | Test | Priorité | Notes |
|---|------|----------|-------|
| **W1** | BM25 search E2E | Haute | Playwright, config correcte (contentFor) |
| **W2** | Vector search E2E | Haute | CandleEmbedder MiniLM en WASM |
| **W3** | Hybrid search E2E | Haute | BM25 + vector fusion |
| **W4** | Sparse search E2E | Moyenne | Dépend de sparse_vector en WASM statique |
| **W5** | Persistance IDBFS + search | Moyenne | create → close → reopen → search |

### Infrastructure / modernisation (basse priorité)

| # | Tâche | Notes |
|---|-------|-------|
| **I1** | Schema JSON → typed bridge | `create_index(path, schemaJson)` — appelé 1 fois, faible impact |
| **I2** | Test fixture léger GTest | `EmptyApiTest` sans tinysnb (~800ms/test) |

---

## Architecture actuelle — résumé

```
Utilisateur Cypher (end-user)
│
├── WHERE SEARCH(d.body, 'rust')        → INDEX_SCAN optimizer → typed bridge (zéro JSON)
├── WHERE SPARSE_SEARCH(d.ID, [...])    → INDEX_SCAN optimizer → Rust FFI
├── WHERE VECTOR_SEARCH(d.emb, [...])   → INDEX_SCAN optimizer → HNSW (bind-time)
│
└── CALL QUERY_LUCIVY_INDEX(...)       → Table function → JSON bridge (flexible)
    CALL QUERY_VECTOR_INDEX(...)        → Table function → C++ HNSW
    CALL QUERY_SPARSE_VECTOR_INDEX(...) → Table function → Rust FFI

Rag3weaver (orchestrateur Rust)
│
├── search_bm25()    → CALL QUERY_LUCIVY_INDEX (JSON, allowed_ids, lucivy_filters)
├── search_vector()  → CALL QUERY_VECTOR_INDEX (projected graphs, SemiMask)
├── search_sparse()  → CALL QUERY_SPARSE_VECTOR_INDEX
│
├── fusion (RRF/weighted/boost) → per-signal config, per-KB
├── chunk resolution → parent enrichment + ChunkInfo
├── filter compiler → split Lucivy-native / Kuzu-only
├── explore → search + BFS graph expansion
└── event bus → CatalogEvent + QueueEvent (async_broadcast)
```

Deux API coexistent :
- **Table functions** (`CALL ...`) — pour l'orchestrateur, flexible, advanced features
- **WHERE clause** (`SEARCH()`, `VECTOR_SEARCH()`, `SPARSE_SEARCH()`) — pour l'utilisateur Cypher, composable, zéro JSON

---

## Conclusion — Ce qu'il faut faire maintenant

### Décisions prises

- **Highlights BM25 dans SearchResult** : pas utile. Les highlights BM25 servent uniquement en interne pour la résolution chunk (quel chunk intersecte le match). Ce qui compte côté API c'est les coordonnées du chunk trouvé (`start_line`, `end_line`, `start_char`, `end_char`) — et ça, `ChunkInfo` le retourne déjà. → **Rien à faire.**

- **Events manquants** (`DrainStarted`, `DrainCompleted`, `SearchStarted`, `EntityCreated`) : trivial, on a déjà l'infra EventBus + les events qui marchent (`SearchCompleted`, `EntityUpdated`, `EntityDeleted`, tout le QueueEvent système). C'est juste 4 `emit()` à ajouter. → **Quick win, on le fait en début de session.**

### Priorité réelle

1. **Events manquants** — 4 emit() à ajouter, 15 minutes (L2-L4)
2. **Audit pré-filtre vs post-filtre** — Identifier tout ce qui passe encore en post-filter (Cypher WHERE après search) alors qu'on pourrait le pousser en pré-filter (Lucivy natif ou allowed_ids). Objectif : zéro post-filter, tout en pre-filter pour la perf.
3. **Tests E2E filtres** — Valider le FilterCompiler end-to-end avec des données réelles (E1-E3)
4. **Tests E2E chunking explicit** — body long > maxSize, vérifier chunks, ChunkInfo dans résultats, cascade delete (E6)
5. **Tests E2E explore** — search + BFS avec relations réelles, depth=1/2 (E4)
6. **WASM search E2E** — BM25 + vector + hybrid en browser Playwright (W1-W3)

---

## Ouverture — Semaine prochaine (usages concrets)

Le framework rag3weaver est feature-complete pour le core. La prochaine étape c'est de le brancher sur des **sources de données réelles** et valider le pipeline d'ingestion end-to-end avec du contenu varié.

### Composio — Connecteurs SaaS

- **Shopify** : ingestion produits (titre, description, tags, prix, images) → entité Product avec filter fields (price Double, category String, in_stock Boolean). Relations Product→Collection, Product→Variant. Test : search hybride "red cotton t-shirt" avec filtre price < 50.
- **Google Drive** : ingestion arborescence → entités Directory, File, avec relation Directory→File (hiérarchie). Extraction texte depuis Docs/Sheets. Mention detection : URLs dans le contenu → relations File→URL. Emails référencés → relations File→Contact.
- **Gmail / Outlook** : ingestion mails → entité Mail avec relations Mail→Contact (from/to/cc), Mail→Attachment (fichier joint), Mail→URL (liens dans le body). Chunking sur le body des longs threads.

### Ingestion documents binaires

- **PDF** : extraction texte + métadonnées (auteur, date, pages). Chunking markdown sur le contenu extrait. Relations Document→Author.
- **DOCX / PPTX** : extraction via pandoc ou libreoffice. Même pipeline que PDF.
- **Images** : OCR (tesseract) ou description (vision model) → champ `description` indexé en FTS + embedded.
- **Relations directory/file** : arborescence locale ou cloud → Directory (path, name) → HAS_FILE → File (name, extension, size, content). Permet de chercher "tous les fichiers dans le dossier X qui parlent de Y".

### Codeparsers / ingestion GitHub

- **codeparsers-transpiler** : parsing AST (tree-sitter) → entités Module, Class, Function, Variable avec relations structurelles (Module→Class→Function, Function→CALLS→Function, Class→INHERITS→Class). Chunking par fonction/méthode. FTS sur le code + embeddings sémantiques sur les docstrings.
- **GitHub repos** : ingestion via gh API → entités Repository, Issue, PR, Commit, File. Relations Commit→File (MODIFIES), PR→Issue (CLOSES), PR→Commit. Search : "quels commits ont touché l'authentification ?" → vector search sur commit messages + BFS sur les fichiers modifiés.
- **GitHub discussions / wiki** : même pipeline que documents markdown, avec relations entre pages (liens internes).

### Ce que ça teste

Ces usages réels vont stress-tester :
- **Variété de types** : Text, String, Integer, Double, Boolean, Timestamp, Json, Tags — tous les FieldType
- **Relations riches** : hiérarchies (directory/file), citations (mail→contact), structurelles (class→method)
- **Chunking à l'échelle** : milliers de documents, certains > 100KB
- **Filtres complexes** : date ranges, catégories, statuts, combinaisons AND/OR
- **Multi-KB** : un KB "code" (semantic sur docstrings), un KB "docs" (hybrid sur markdown), un KB "issues" (fulltext)
- **Explore** : naviguer le graphe depuis un résultat de recherche (fichier → commits → PRs → issues)
