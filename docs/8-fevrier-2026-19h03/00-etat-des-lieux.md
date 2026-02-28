# Etat des Lieux — 8 Fevrier 2026

> **Objectif final :** Remplacer Neo4j par Kuzu (embedded) + Tantivy (FTS fuzzy/regex/contains) dans un seul module WASM, pour alimenter le framework Rag3Weaver puis ragforge-core et community-docs.

---

## Ce qu'on a

### 1. rag3db (Fork Kuzu v0.11.2.2)

**Emplacement :** `packages/rag3db/`
**Repo :** https://github.com/L-Defraiteur/rag3db
**License :** MIT (Kuzu upstream) + nos modifs Source Available

Un fork de Kuzu avec :
- Le code source C++ complet (graph DB, Cypher, storage, extensions)
- L'extension FTS native (BM25, stemming Snowball, tokenization)
- L'extension Vector (HNSW index)
- Build WASM via Emscripten avec support **pthreads** (`-s USE_PTHREADS`, `SharedArrayBuffer`)

### 2. ld-tantivy (Fork Tantivy)

**Emplacement :** `packages/rag3db/extension/tantivy/ld-tantivy/`
**Repo :** https://github.com/L-Defraiteur/tantivy
**License :** MIT
**Lignee :** `quickwit-oss/tantivy v0.22` → `izihawa/tantivy v0.26.0` → `ld-tantivy`

Fork de Tantivy avec extensions pour la recherche de contenu :
- **ContainsQuery** — recherche multi-strategie avec auto-cascade (exact → fuzzy → substring → fuzzy substring)
- **ContainsScorer** — validation des separateurs et distance cumulative
- **NgramContainsQuery** — recherche contains acceleree par index trigram
- **WithFreqsAndPositionsAndOffsets** — byte offsets dans les posting lists (comme Lucene)
- **HighlightSink** — capture side-channel des byte offsets pour tous les types de query
- **FuzzySubstringAutomaton** — automate combine `.*{levenshtein(token,d)}.*`

**48 fichiers modifies** par rapport a izihawa/tantivy. **1015 tests** (7 ignored).

### 3. tantivy_fts (Crate FFI C)

**Emplacement :** `packages/rag3db/extension/tantivy_fts/rust/`

Crate Rust exposant ld-tantivy via une API C FFI :
- Fonctions extern "C" : lifecycle, ecriture, lecture, info
- Architecture tri-field : `{name}` (stemmed), `{name}._raw` (lowercase), `{name}._ngram` (trigrams)
- Routing transparent des queries par type
- Highlighting : byte offsets retournes dans les resultats de recherche
- `StdFsDirectory` agnostique plateforme (natif + Emscripten)

**153 tests FFI** (tous passent).

### 4. Rag3Weaver (Framework RAG TypeScript)

**Emplacement :** `packages/ragforge-core-exp-kuzu/kuzu-wasm-exp/src/`

Framework complet au-dessus de Kuzu WASM :
- L1 : QueryBuilder, SchemaBuilder (prepared statements)
- L2 : Chunker, DocumentStore, UUIDGenerator
- L3 : FilterParser, SemanticChunker, Ref, EventEmitter
- Catalog : CRUD, Search (hybrid vector+BM25, fusion RRF), Schema
- Queue : OperationQueue (priority PERSIST > INSERT > LINK > EMBED)

**Status : fonctionnel, mais search sans fuzzy/contains**

### 5. Kuzu WASM (bindings existants)

**Emplacement :** `packages/ragforge-core-exp-kuzu/kuzu-wasm-exp/`

Build Emscripten de Kuzu avec :
- `emsdk/` installe localement
- CMakeLists.txt avec `-s USE_PTHREADS`
- Tests browser avec verification SharedArrayBuffer
- Headers COOP/COEP documentes pour le deploiement

---

## Architecture cible

```
                        Build time (Emscripten)
                        =======================

  ┌──────────────────────────────────────────────────────────┐
  │                                                          │
  │  ld-tantivy + tantivy_fts (Rust)                         │
  │  - Full-text search (BM25)                               │
  │  - Fuzzy search (Levenshtein automaton)                  │
  │  - Contains search (multi-cascade + ngram)               │
  │  - Regex, phrase, parse queries                          │
  │  - Highlighting (byte offsets) pour tous les types       │
  │                                                          │
  │  Compile: cargo build --target wasm32-unknown-emscripten │
  │  Output: libtantivy_fts.a (static lib C)                 │
  │                                                          │
  └───────────────────────────┬──────────────────────────────┘
                              │ C FFI link
                              ▼
  ┌──────────────────────────────────────────────────────────┐
  │                                                          │
  │  rag3db (Kuzu fork, C++)                                 │
  │  - Graph database (Cypher)                               │
  │  - Storage engine (columnar, CSR)                        │
  │  - Vector index (HNSW)                                   │
  │  - Extension FTS → appelle Tantivy via C FFI             │
  │                                                          │
  │  Compile: emcc + CMake                                   │
  │  Output: rag3db.wasm + rag3db.js + worker.js             │
  │                                                          │
  └───────────────────────────┬──────────────────────────────┘
                              │
                              ▼
                        Runtime (Browser/Node)
                        =====================

  ┌──────────────────────────────────────────────────────────┐
  │                                                          │
  │  Rag3Weaver (TypeScript)                                 │
  │  - L1-L3 abstractions                                    │
  │  - Catalog (CRUD, Search, Schema, Queue)                 │
  │  - Hybrid search: Vector + BM25 + Fuzzy + Contains       │
  │                                                          │
  └───────────────────────────┬──────────────────────────────┘
                              │
                              ▼
  ┌──────────────────────────────────────────────────────────┐
  │                                                          │
  │  ragforge-core → community-docs                          │
  │  - Ingestion, search, entity extraction                  │
  │  - Remplace Neo4j (zero Docker, embedded)                │
  │                                                          │
  └──────────────────────────────────────────────────────────┘
```

---

## Travail accompli — Phases Rust

### Phase 1 : Crate FFI Rust — FAIT (6 fevrier)

- [x] API C FFI (create/open/close, add/delete/commit/rollback, search/search_filtered, etc.)
- [x] `StdFsDirectory` agnostique plateforme
- [x] Architecture dual-field : stemmed + `._raw` avec routing transparent
- [x] Compilation native + Emscripten
- [x] Header C via cbindgen
- [x] Persistence config `_config.json`

### Phase 2 : Integration CMake — FAIT (6 fevrier)

- [x] CMakeLists.txt liant la static lib Rust (cargo via add_custom_command)
- [x] Build natif : `libtantivy_fts.kuzu_extension` (15 KB)
- [x] Build WASM : `libkuzu.a` (36 MB) avec json+vector+algo+tantivy_fts

### Phase 3 : WithFreqsAndPositionsAndOffsets — FAIT (6-7 fevrier)

Nouveau variant `IndexRecordOption` qui stocke les byte offsets `(offset_from, offset_to)` de chaque token directement dans les postings, comme Lucene.

- [x] Fichier `.offsets` (SegmentComponent::Offsets) avec encoding bitpacked
- [x] Write pipeline : Token → PostingsWriter → FieldSerializer → PositionSerializer
- [x] Read pipeline : InvertedIndexReader → PositionReader → SegmentPostings
- [x] TermInfo etendu (40 bytes, +offsets_range)
- [x] Propagation unions : LoadedPostings, SimpleUnion, BitSetPostingUnion, PostingsWithOffset
- [x] Methode jointe `append_positions_and_offsets()` sur le trait Postings

**21 fichiers modifies dans ld-tantivy.**

### Phase 4 : ContainsQuery — FAIT (6-7 fevrier)

Recherche de sous-chaines dans les termes indexes, avec cascade automatique par position :

1. **Exact** — lookup direct dans le dictionnaire de termes
2. **Fuzzy** — automate de Levenshtein (distance configurable)
3. **Substring** — regex `.*token.*` sur le dictionnaire
4. **Fuzzy substring** — automate combine `.*{levenshtein(token,d)}.*`

Pour les requetes multi-token (`"std::collections"`, `"c++"`), le `ContainsScorer` verifie :
- Positions consecutives via PhraseScorer
- Separateurs exacts entre tokens (comparaison edit distance)
- Budget de distance cumulative global

**Fichiers :** `automaton_phrase_query.rs`, `automaton_phrase_weight.rs`, `contains_scorer.rs`, `fuzzy_substring_automaton.rs`

### Phase 5 : NgramContainsQuery — FAIT (7 fevrier)

Alternative rapide a ContainsQuery utilisant un index trigram pour le filtrage des candidats :

1. Extraire trigrams de la query → lookup dans l'index ngram → doc IDs candidats
2. Pour chaque candidat, lire le texte stocke et verifier (exact, fuzzy, substring)
3. Meme validation de separateurs et distance cumulative que ContainsScorer

Necessite un champ `._ngram` (troisieme champ dans le layout tri-field).

**Fichier :** `ngram_contains_query.rs`

### Phase 6 : Highlighting v1 (contains) — FAIT (7 fevrier)

`HighlightSink` — side-channel thread-safe pour capturer les byte offsets pendant le scoring :

```rust
pub struct HighlightSink {
    data: Mutex<HashMap<(u32, DocId), Vec<[usize; 2]>>>,
    segment_counter: AtomicU32,
}
```

Pour les contains queries, les offsets sont un sous-produit gratuit de la verification du texte stocke.

**Fichier :** `scoring_utils.rs`

### Phase 7 : Highlighting v2 (tous les types) — FAIT (8 fevrier)

Extension du highlighting a tous les types de query en lisant les byte offsets directement depuis les posting lists :

| Query type | Scorer | Source des offsets |
|------------|--------|-------------------|
| **contains** | ContainsScorer, ContainsSingleScorer | Verification texte stocke (gratuit) |
| **ngram contains** | NgramContainsScorer | Verification texte stocke (gratuit) |
| **term** | TermScorer | Postings via `append_offsets()` |
| **fuzzy** | AutomatonWeight | Postings (capture pendant construction scorer) |
| **regex** | AutomatonWeight | Postings (capture pendant construction scorer) |
| **phrase** | PhraseScorer | Postings via `drain_or_capture_offsets()` |

**Bug corrige :** desynchronisation `segment_ord` — `next_segment()` doit etre appele meme pour les segments vides (terme absent) pour rester en sync avec les ordinals reels de TopDocs.

**9 fichiers modifies dans ld-tantivy.**

---

## Architecture tri-field (stemming actif)

Quand `"stemmer": "english"` est specifie, chaque champ "text" genere trois champs Tantivy :

| Champ | Tokenizer | IndexRecordOption | Stocke | Usage |
|-------|-----------|-------------------|--------|-------|
| `{name}` | stemmed (SimpleTokenizer + LowerCaser + Stemmer) | WithFreqsAndPositionsAndOffsets | Oui | phrase, parse |
| `{name}._raw` | default (SimpleTokenizer + LowerCaser) | WithFreqsAndPositionsAndOffsets | Non | term, fuzzy, regex, contains |
| `{name}._ngram` | ngram (SimpleTokenizer + LowerCaser + NgramFilter) | Basic | Non | NgramContainsQuery candidats |

Le routing est transparent — l'utilisateur reference toujours le nom de base.

---

## Format query_json (API FFI)

```json
{"type": "term", "field": "body", "value": "function"}
{"type": "fuzzy", "field": "body", "value": "fonctoin", "distance": 2}
{"type": "phrase", "field": "body", "terms": ["hello", "world"]}
{"type": "regex", "field": "body", "pattern": "func.*ion"}
{"type": "contains", "field": "body", "value": "std::collections", "fuzzy_distance": 1}
{"type": "boolean", "must": [...], "should": [...], "must_not": [...]}
{"type": "parse", "field": "body", "value": "hello world"}
```

Option `"highlight": true` sur n'importe quel type → les resultats incluent un champ `"highlights"` avec les byte offsets `[[from, to], ...]`.

---

## Format des resultats

```json
[{
  "score": 1.23,
  "doc": {"body": "Rust programming is great"},
  "highlights": {"body": [[0, 4]]}
}]
```

`"highlights"` est present seulement si `"highlight": true` dans la query.

---

## Ce qui reste a faire

### Phase A : Extension C++ (wrapper rag3db)

Exposer tantivy_fts comme extension Cypher dans rag3db. Pattern identique a l'extension FTS existante.

**Fichiers a creer :**
```
extension/tantivy_fts/src/
├── function/
│   ├── create_tantivy_index.cpp     ← CREATE_TANTIVY_INDEX
│   ├── drop_tantivy_index.cpp       ← DROP_TANTIVY_INDEX
│   └── query_tantivy_index.cpp      ← QUERY_TANTIVY_INDEX (modes: parse, fuzzy, regex, exact, contains)
├── index/
│   └── tantivy_index.cpp            ← TantivyIndex (wrapper FFI, insert/delete/checkpoint)
└── catalog/
    └── tantivy_catalog_entry.cpp    ← Serialisation metadata dans catalog Kuzu
```

**Modes Cypher publics :**

| Mode | Type FFI | Champ | Description |
|------|----------|-------|-------------|
| `parse` (defaut) | `parse` | stemmed | Recherche langage naturel, stemming |
| `fuzzy` | `fuzzy` | raw | Tolerant aux typos (Levenshtein) |
| `regex` | `regex` | raw | Pattern matching |
| `exact` | `regex` (`.*{term}.*`) | raw | Match exact, reroute vers regex |
| `contains` | `contains` | raw + ngram | Recherche de sous-chaines (code, identifiers) |

**Exemples Cypher :**
```sql
CALL CREATE_TANTIVY_INDEX('Article', 'article_fts', ['title', 'body'],
    stemmer := 'english');

CALL QUERY_TANTIVY_INDEX('Article', 'article_fts', 'running programs')
RETURN node, score;

CALL QUERY_TANTIVY_INDEX('Article', 'article_fts', 'std::collections',
    mode := 'contains')
RETURN node, score;

MATCH (n:Article) WHERE n.year > 2020
WITH collect(n._node_id) AS ids
CALL QUERY_TANTIVY_INDEX('Article', 'article_fts', 'query',
    filter_ids := ids)
RETURN node, score;

CALL DROP_TANTIVY_INDEX('Article', 'article_fts');
```

### Phase B : Tests end-to-end

- [ ] Tests unitaires C++ (extension)
- [ ] Tests Cypher (create, query 5 modes, filtered, delete, reopen)
- [ ] Tests WASM (Node.js ou browser)
- [ ] Metriques : taille .wasm finale, temps creation index, temps recherche par mode

### Phase C : Integration Rag3Weaver

- [ ] Modifier `CatalogSearch` pour utiliser `QUERY_TANTIVY_INDEX`
- [ ] Ajouter options `mode`, `distance`, `highlight` dans les parametres de search
- [ ] Tests end-to-end avec pipeline complet (ingestion → search → highlights → results)

---

## Stockage

### Segments Tantivy

```
{kuzu_db_path}/tantivy/{table_id}_{index_name}/
├── _config.json          ← Config (stemmer, champs) pour reopen
├── meta.json             ← Metadata Tantivy (segment list, schema)
├── {seg_id}.idx          ← Inverted index
├── {seg_id}.pos          ← Term positions
├── {seg_id}.offsets      ← Byte offsets (NOUVEAU)
├── {seg_id}.store        ← Document store
├── {seg_id}.fast         ← Fast fields (columnar)
├── {seg_id}.fn           ← Field norms
└── {seg_id}.del          ← Alive bitset (deletions)
```

### Plateforme

| Plateforme | Directory Tantivy | Stockage | Persistence |
|------------|-------------------|----------|-------------|
| **Natif** | `StdFsDirectory` (→ `MmapDirectory`) | Vrai filesystem | Automatique |
| **WASM** | `StdFsDirectory` (→ VFS Emscripten) | MEMFS | IDBFS → IndexedDB via `FS.syncfs()` |
| **Tests** | `RamDirectory` | RAM pure | Aucune |

Le choix est fait a la compilation via `#[cfg()]`. Le code C++ n'a pas besoin de savoir.

---

## Commandes build

```bash
# ld-tantivy (1015 tests)
cd packages/rag3db/extension/tantivy/ld-tantivy && cargo test --lib

# tantivy_fts build
cd packages/rag3db/extension/tantivy_fts/rust && cargo build --release

# FFI tests (153 tests)
cd packages/rag3db/extension/tantivy_fts/test && \
  cc -o test_ffi test_ffi.c -I../include -L../rust/target/release -ltantivy_fts -lpthread -lm -ldl && \
  ./test_ffi

# rag3db natif (avec extension tantivy_fts)
cd packages/rag3db && mkdir -p build && cd build && \
  cmake .. -DBUILD_EXTENSIONS="tantivy_fts" && make -j$(nproc)

# rag3db WASM
source .../emsdk/emsdk_env.sh && cd packages/rag3db/build-wasm && \
  emcmake cmake .. -DBUILD_EXTENSIONS="json;vector;algo;tantivy_fts" -DBUILD_WASM=FALSE && \
  emmake make -j$(nproc)
```

---

## Fichiers de reference

| Sujet | Emplacement |
|-------|-------------|
| **ld-tantivy** (fork Tantivy) | `packages/rag3db/extension/tantivy/ld-tantivy/` |
| **tantivy_fts** (crate FFI) | `packages/rag3db/extension/tantivy_fts/rust/` |
| Header C genere | `packages/rag3db/extension/tantivy_fts/include/tantivy_fts.h` |
| Tests FFI | `packages/rag3db/extension/tantivy_fts/test/test_ffi.c` |
| Extension C++ (stub) | `packages/rag3db/extension/tantivy_fts/src/` |
| Extension FTS existante | `packages/rag3db/extension/fts/` |
| Build WASM Kuzu | `kuzu-wasm-exp/CMakeLists.txt` |
| Rag3Weaver search | `kuzu-wasm-exp/src/lib/catalog/modules/CatalogSearch.ts` |
| Docs session 6 fevrier | `docs/6-fevrier-2026-22h22/` |
| Plan highlight all types | `docs/8-fevrier-2026-19h03/01-highlight-all-query-types-plan.md` |

---

## Questions resolues

| Question | Reponse |
|----------|---------|
| Tantivy en WASM avec threads ? | C FFI + Emscripten. Un seul `.wasm` avec graph DB + FTS. |
| Gestion memoire Rust/C++ ? | Handles opaques (`TantivyHandle*`), lifetime par create/close. Strings par `tantivy_free_string`. |
| Index storage ? | `{db_path}/tantivy/{table_id}_{index_name}/`. Meme filesystem, dossier separe. |
| Stemming + exact match ? | Tri-field : stemmed + `._raw` + `._ngram`. Routing transparent par type de query. |
| Recherche de code (`c++`, `std::collections`) ? | ContainsQuery avec cascade 4 niveaux + validation separateurs + NgramContainsQuery. |
| Highlighting ? | HighlightSink side-channel. Byte offsets depuis les posting lists pour tous les types de query. |
| Taille WASM ? | Static lib Tantivy FFI = 17 MB. Build complet rag3db = 36 MB (libkuzu.a). Taille .wasm finale a mesurer. |
