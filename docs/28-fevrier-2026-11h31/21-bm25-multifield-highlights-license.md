# 21 — BM25 multi-field highlights, chunk resolution BM25, license LRSL v1.2

## Ce qu'on a fait

### 1. HighlightSink per-field dans ld-lucivy (Phase B)

**Probleme** : `HighlightSink` ne supportait pas les requetes multi-champs. Pour une KB avec title + body + summary, `build_boolean_query()` construisait un `boolean should` mais passait `None` pour `highlight_sink` aux sous-requetes. Resultat : highlights vides pour le cas standard rag3weaver.

**3 bugs corriges** :
1. `insert()` ne trackait pas le nom du champ — tous les offsets dans le meme bucket
2. `insert()` faisait `HashMap::insert()` (ecrase) au lieu de `extend()` (append)
3. `build_boolean_query()` passait `None` pour `highlight_sink` aux sous-requetes

**Changements (16 fichiers)** :

| Fichier | Modification |
|---------|-------------|
| `scoring_utils.rs` | Stockage `(field_name, start, end)`, `insert()` prend `field_name: &str`, `get()` retourne `HashMap<String, Vec<[usize;2]>>` |
| `term_scorer.rs`, `term_weight.rs`, `term_query.rs` | `highlight_field_name: String` propage Query→Weight→Scorer |
| `automaton_weight.rs` | Idem |
| `fuzzy_query.rs` | Idem |
| `regex_query.rs` | Idem |
| `phrase_query.rs`, `phrase_weight.rs`, `phrase_scorer.rs` | Idem |
| `automaton_phrase_query.rs`, `automaton_phrase_weight.rs` | Idem |
| `contains_scorer.rs` | Idem (2 structs: ContainsScorer + ContainsSingleScorer) |
| `ngram_contains_query.rs` | Le plus complexe — 3 helpers + tests mis a jour pour HashMap |
| `query.rs` (lucivy_fts) | Tous les `with_highlight_sink(sink)` → `with_highlight_sink(sink, field_name)`, `build_boolean_query` propage le sink |
| `bridge.rs` (lucivy_fts) | `collect_search_results_with_highlights` utilise per-field `sink.get()`, retire param `highlight_field` |

**Tests** : 1064 lib tests OK, 15 GTest E2E OK.

### 2. BM25 chunk resolution dans rag3weaver

**Nouveau dans search.rs** :
- `BM25Hit` struct : uuid + score + highlights `HashMap<String, Vec<(usize, usize)>>`
- `search_bm25_raw()` : comme `search_bm25` mais `RETURN node_id, score, highlights` (3e colonne JSON)
- `parse_highlights_json()` : parse `{"body":[[100,200]],"title":[[5,15]]}` → HashMap
- `resolve_bm25_to_chunks()` : pour chaque hit, retourne **tous** les chunks qui intersectent un highlight (tries par overlap decroissant). Si aucun chunk ne matche (match dans title non-chunke) → `chunk: None`

**Cablage dans catalog.rs** :
- `is_chunked` detecte tot (avant le match search_type)
- Branches Hybrid et BM25Only : si chunked → `search_bm25_raw()` + `resolve_bm25_to_chunks()`, sinon → `search_bm25()` classique

**Tests** : 345 unit tests OK, 10 phase2 OK, 6 phase0 OK.

### 3. License LRSL v1.2

**Probleme** : la LRSL v1.1 dans community-docs disait "monthly revenue" au lieu de "annual revenue" pour le seuil de 100K EUR.

**Fix** : nouvelle version v1.2 avec "annual" dans les deux repos :

| Repo | LICENSE | NOTICE |
|------|---------|--------|
| ld-lucivy | LRSL v1.2 (fork Lucivy v0.26.0) | MIT originale Lucivy |
| rag3db | LRSL v1.2 (fork Kuzu v0.11.2.2) | MIT originale Kuzu + mention Lucivy |

community-docs reste en v1.1 (monthly) — a corriger separement.

### 4. README rag3db

Reecrit pour refleter l'etat actuel :
- 3 extensions documentees : lucivy_fts, vector (avec DELETE/UPDATE), sparse_vector
- Section rag3weaver complete (Catalog API, chunking, search hybride, filtrage, embedders, queue, WASM FFI)
- Architecture mise a jour
- License LRSL v1.2 referencee

### 5. Commits et push

**ld-lucivy** (2 commits) :
1. `feat: per-field highlight tracking in HighlightSink` (16 files, +215 -97)
2. `chore: LRSL v1.2 license, add NOTICE for Lucivy MIT attribution`

**rag3db** (1 commit) :
- `feat: rag3weaver chunk resolution, BM25 multi-field highlights, HNSW delete/update` (38 files, +5315 -316)

## Etat des tests

| Suite | Resultat |
|-------|----------|
| ld-lucivy `cargo test --lib` | 1064 OK |
| lucivy_fts GTest E2E | 15 OK |
| rag3weaver `cargo test` | 345 OK |
| E2E phase0 (CRUD) | 6/6 OK |
| E2E phase1 (BM25) | 6/6 OK |
| E2E phase2 (vector) | 10/10 OK |

## Prochaines etapes

1. **Explorer l'integration Lucivy dans le query planner rag3db** (niveau 2) — faire de l'extension lucivy un vrai index provider integre au Cypher, avec predicate pushdown pour les filtres
2. **Phase 3 tests** (hybrid dense+BM25) — les infra sont pretes, il manque les tests e2e
3. **Phase 5 tests** (chunking) — l'infra chunk resolution est complete, il faut des tests qui verifient `ChunkInfo` dans les resultats
4. **Simplifier les filtres** — potentiellement tout passer par Cypher (retirer les filter_fields Lucivy) si le niveau 2 se concretise

## Fichiers crees/modifies cette session

| Fichier | Action |
|---------|--------|
| `ld-lucivy/src/query/phrase_query/scoring_utils.rs` | HighlightSink per-field |
| `ld-lucivy/src/query/**/*.rs` (14 fichiers) | Propagation highlight_field_name |
| `ld-lucivy/lucivy_fts/rust/src/query.rs` | with_highlight_sink + boolean propagation |
| `ld-lucivy/lucivy_fts/rust/src/bridge.rs` | Per-field sink.get(), retire highlight_field |
| `ld-lucivy/LICENSE` | LRSL v1.2 |
| `ld-lucivy/NOTICE` | MIT Lucivy attribution |
| `rag3db/extension/rag3weaver/src/search.rs` | search_bm25_raw, resolve_bm25_to_chunks |
| `rag3db/extension/rag3weaver/src/catalog.rs` | Cablage BM25 chunk resolution |
| `rag3db/LICENSE` | LRSL v1.2 |
| `rag3db/NOTICE` | MIT Kuzu + Lucivy attribution |
| `rag3db/README.md` | Reecrit complet |
