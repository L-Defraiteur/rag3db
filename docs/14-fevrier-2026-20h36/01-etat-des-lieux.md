# Etat des lieux — 14 fevrier 2026, 20h36

> Consolidation de tout le travail effectue le 14 fevrier 2026 (sessions 00h58 a 20h36).

---

## Vue d'ensemble

Le projet integre un moteur de recherche full-text fuzzy (Lucivy) dans **rag3db** (fork de Kuzu v0.11.2.2), compile en natif, Node.js et WASM browser, pour servir de backend a **Rag3Weaver** (framework RAG TypeScript).

### Repos

| Repo | Branche | Dernier commit | Description |
|------|---------|----------------|-------------|
| `ld-lucivy` | `main` | `77c4ca6` | Fork Lucivy v0.26.0, submodule de rag3db |
| `rag3db` | `feature/fuzzy-fts` | `e0186a809` | Fork Kuzu v0.11.2.2, renomme rag3db |

---

## Strategie de recherche — quel type de query pour quoi

### `"contains"` = le type principal pour Rag3Weaver (recherche hybride BM25)

Le type `"contains"` utilise **NgramContainsQuery**, qui est le coeur du moteur. C'est lui qui sera expose comme composant BM25 dans la recherche hybride de Rag3Weaver.

**Ce qu'il fait en interne** — 3 modes de verification via `VerificationMode` enum :

**Mode Fuzzy** (defaut, `regex: false`) :
1. **Exact lookup** sur `._raw` + **trigram intersection** sur `._ngram` → candidats
2. **Verification fuzzy** — `token_match_distance()` (exact, substring, Levenshtein)
3. **BM25 scoring** — `score = idf * (1+k1) * tf / (tf + k1*(1-b+b*dl/avgdl))`

**Mode Regex** (`regex: true`) :
1. **Parse HIR** → `regex_syntax::Extractor` → litteraux obligatoires
2. **Trigram union** sur `._ngram` depuis les litteraux (ou full-scan si litteraux < 3 chars)
3. **Verification regex** — `regex::find_iter()` sur texte stocke → count tf
4. **BM25 scoring** identique

**Mode Hybride** (`regex: true, distance > 0`) :
- Verification regex exact OU fuzzy sur les litteraux extraits
- `tf = max(tf_regex, tf_fuzzy)` — le meilleur des deux

**Parametres cles** :
- `distance` (defaut: **1**) — distance de Levenshtein max par token. Tolere 1 faute de frappe par defaut.
- `distance_budget` — budget cumulatif de distance sur tous les tokens + separateurs
- `strict_separators` (defaut: **true**) — les separateurs (`::`, `++`, `.`) doivent matcher dans le budget de distance

**Pourquoi c'est le bon choix pour le RAG** :
- Tolerant aux fautes (fuzzy par defaut, distance=1)
- Tolerant aux substrings ("program" trouve "programming")
- Multi-token ("std::collections" fonctionne)
- Score BM25 differencie (pas un boost constant)
- Fast path via trigram index (pas un FST walk)
- Highlights inclus gratuitement

### Les autres types de query — secondaires

| Type | Moteur interne | Usage | Scoring |
|------|---------------|-------|---------|
| **`contains`** | **NgramContainsQuery** | **Recherche principale** — fuzzy/regex/hybride + BM25 | **BM25** |
| `term` | TermQuery | Lookup exact d'un seul mot | BM25 (natif Lucivy) |
| `fuzzy` | FuzzyTermQuery | Fuzzy whole-word (PAS substring) d'un seul mot | BM25 (natif Lucivy) |
| `phrase` | PhraseQuery | Sequence exacte de mots adjacents | BM25 (natif Lucivy) |
| `regex` | RegexQuery | Pattern regex sur le term dict | BM25 (natif Lucivy) |
| `parse` | QueryParser | Syntaxe Lucene-like (AND, OR, champs, etc.) | BM25 (natif Lucivy) |
| `boolean` | BooleanQuery | Combinaison de sous-queries | Combine |

**Distinction importante** : `"fuzzy"` (FuzzyTermQuery) est **strictement inferieur** a `"contains"` pour le use case RAG :
- `"fuzzy"` = fuzzy whole-word seulement, pas de substring, pas de multi-token
- `"contains"` = fuzzy + substring + multi-token + separateurs + BM25

Le type `"fuzzy"` existe pour compatibilite et cas specifiques (match exact d'un seul mot avec tolerance), mais **`"contains"` est le type a exposer dans Rag3Weaver**.

### Pipeline hybride Rag3Weaver (cible)

```
                     Requete utilisateur
                            |
              +-------------+-------------+
              |                           |
     QUERY_LUCIVY_INDEX           QUERY_VECTOR_INDEX
     type: "contains"              metric: cosine
     fuzzy distance=1              top_k=100
     BM25 scoring                  embedding similarity
              |                           |
              +-------------+-------------+
                            |
                   Fusion RRF / weighted
                   (cote TypeScript)
                            |
                     Resultats finaux
```

---

## Tout ce qui a ete fait (chronologique)

### Phases 1-8 (1er-13 fevrier) — Moteur Lucivy

Details dans `docs/13-fevrier-2026-18h57/01-etat-des-lieux.md`.

- **Crate Rust FFI** `lucivy_fts` — bridge cxx type (9 structs, 15 fonctions)
- **Triple-field layout** automatique : `body` -> stemmed + `._raw` + `._ngram`
- **Cascade 4 niveaux** : exact -> fuzzy -> substring -> fuzzy substring
- **ContainsScorer** (multi-token) + **ContainsSingleScorer** (resout "c++")
- **FuzzySubstringAutomaton** — NFA `.*{levenshtein(token,d)}.*`
- **HighlightSink** — byte offsets pour tous les types de query
- **WithFreqsAndPositionsAndOffsets** — offsets dans les postings (21 fichiers)
- **Rename kuzu -> rag3db** (2538 fichiers)
- **Tests** : 1025 ld-lucivy = tout vert

### Phase A (13-14 fevrier) — Extension C++ `lucivy_fts`

3 fonctions Cypher implementees :

```cypher
CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body']);
CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body'], stemmer := 'french');
CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body'], filter_fields := ['category', 'score']);

CALL QUERY_LUCIVY_INDEX('doc', '{"type":"contains","field":"body","value":"c++"}', 10)
RETURN node_id, score, highlights;

CALL DROP_LUCIVY_INDEX('doc');
```

Architecture :
- `LucivyIndex` herite de `storage::Index` — hooks automatiques (`insert`/`delete_`/`update`)
- `QUERY` = `TableFunc` (necessite RETURN), `CREATE`/`DROP` = `StandaloneTableFunc`
- Bridge cxx type (zero JSON sur hot path)

### Phase B (14 fevrier) — Tests E2E GTest

9 tests initiaux ecrits et valides (fichier + in-memory).

### DELETE/UPDATE + Lazy Commit (14 fevrier)

- **Probleme identifie** : writer != reader dans Lucivy (les mutations ne sont visibles qu'apres commit+reload)
- **Solution implementee** : lazy commit via flag `dirty_`
  - `insert()`, `delete_()`, `update()` -> `dirty_ = true`
  - `QUERY_LUCIVY_INDEX` appelle `flushIfDirty()` avant la recherche (commit + reload une seule fois)
- **Tests E2E** : `LucivyDeleteTest2` et `LucivyUpdateTest` — passent

### Node.js natif (14 fevrier)

- Build `rag3dbjs.node` + extension `.rag3db_extension`
- **139 tests mocha** : tous passent
- lucivy_fts teste manuellement (contains, fuzzy, phrase, parse) — OK
- Extension chargee dynamiquement via `LOAD EXTENSION`

### Build WASM (14 fevrier)

5 bugs corriges pendant le build :
1. `DOC_FREQUENCY_PROP_NAME` forward declaration (ext fts)
2. cxx bridge `-fexceptions` pour emscripten
3. Rust atomics manquants -> nightly + `-Z build-std`
4. `__cpp_exception` conflit -> `-C panic=abort`
5. `libfuzzy_fst.a` natif -> retire `fts` du build WASM

Sortie : `rag3db_wasm.js` 17MB (WASM inline, single file)
Extensions statiquement linkees : json, vector, algo, lucivy_fts

### Tests WASM complets (14 fevrier)

- **WASM NODEFS** : 94 tests mocha passent
- **WASM standard** : lucivy_fts 3/3 + vector HNSW 5/5
- **Playwright browser (IDBFS)** : 2 tests (8 sub-tests) — persistence validee
  - Phase 1 : create DB -> tables -> index lucivy + vector -> query -> syncfs -> IndexedDB
  - Phase 2 : reload depuis IndexedDB -> re-query -> memes resultats

### Ngram sans stemmer + BM25 contains (14 fevrier)

**Changement 1 — handle.rs** : les 3 sous-champs (principal, `._raw`, `._ngram`) sont TOUJOURS crees pour chaque colonne texte, meme sans stemmer. Le contains utilise toujours le fast path (trigram lookup) au lieu du fallback AutomatonPhraseQuery.

**Changement 2 — ngram_contains_query.rs** : le scoring du contains passe de boost constant (1.0) a BM25(k1=1.2, b=0.75) :
- `NgramContainsQuery::weight()` calcule `Bm25Weight` via `EnableScoring`
- `verify()` compte TOUTES les occurrences (tf) au lieu de s'arreter au premier match
- `score()` utilise `bm25_weight.score(fieldnorm_id, last_tf)` — vrai BM25

**Bug debug** : les scores restaient a 1.0 apres implementation. Cause : cmake ne detecte pas les changements Rust (`add_custom_command` sans `DEPENDS`). L'extension `.rag3db_extension` n'etait pas re-linkee. Fix : rebuild manuel (cargo + cmake extension re-link). Documente dans BUILD.md.

### Contains unifie : regex + fuzzy + BM25 (14 fevrier)

NgramContainsQuery unifie avec `VerificationMode` enum (Fuzzy/Regex/Hybride) :

- **Etape 1** : Refactoring — `VerificationMode::Fuzzy(FuzzyParams)`, fonctions libres pour eviter les conflits de borrow
- **Etape 2+3** : `VerificationMode::Regex(RegexParams)` — regex pur + hybride (regex OR fuzzy, `tf = max`)
- **Etape 4** : Routing `regex: true` dans `QueryConfig`, `build_contains_regex()` avec `regex-syntax::Extractor`
- **Etape 5** : Full-scan fallback quand litteraux < 3 chars (tous les docs candidats, regex verification filtre, BM25 score)
- **Etape 6** : 10 tests unitaires Rust + 7 sub-tests GTest E2E (trigram accel, BM25 var, hybride, no match, highlights, regression, short-literal)

API : `{"type":"contains", "value":"program[a-z]+", "regex":true}` et `{"regex":true, "distance":1}` pour hybride.

### Documentation (14 fevrier)

3 documents de build crees :
- `rag3db/BUILD.md` — guide complet tous targets
- `extension/lucivy_fts/BUILD.md` — architecture 3 couches, piege cmake/cargo
- `ld-lucivy/README.md` — restructure (NgramContainsQuery + BM25 en avant)

---

## Etat actuel des tests

| Suite | Resultat | Commande |
|-------|----------|----------|
| Rust (ld-lucivy) | **1025 pass** | `cargo test --lib` |
| Natif GTest E2E | **11 pass** | `./lucivy_fts_test` |
| Node.js natif mocha | **139 pass** | `npm test` (nodejs_api) |
| WASM NODEFS mocha | **94 pass** | `npm test` (tools/wasm) |
| WASM browser Playwright | **2 pass** (8 sub-tests) | `npx playwright test` |

---

## Fonctionnalites validees

### Recherche contains (NgramContainsQuery) — composant BM25 du RAG hybride

| Fonctionnalite | Statut |
|----------------|--------|
| Fuzzy substring (distance=1 par defaut, configurable) | OK |
| Multi-token avec separateurs ("std::collections", "c++", "os.path.join") | OK |
| BM25 scoring (k1=1.2, b=0.75, tf + fieldnorm) | OK |
| Trigram fast path (pas de FST walk) | OK |
| Fonctionne avec ou sans stemmer | OK |
| Triple-field layout automatique (principal + `._raw` + `._ngram`) | OK |
| Highlights (byte offsets des matchs) | OK |
| Regex accelere par trigrams (`regex: true`) | OK |
| Hybride regex + fuzzy (`regex: true, distance > 0`) | OK |
| Full-scan fallback pour regex short-literal (< 3 chars) + BM25 | OK |

### Autres types de query (secondaires)

| Fonctionnalite | Statut |
|----------------|--------|
| term (exact single word) | OK |
| fuzzy (FuzzyTermQuery, whole-word seulement) | OK |
| phrase (sequence exacte de mots) | OK |
| regex (pattern sur term dict) | OK |
| parse (syntaxe Lucene-like) | OK |
| boolean (combinaison de sous-queries) | OK |

### Infrastructure

| Fonctionnalite | Statut |
|----------------|--------|
| CREATE_LUCIVY_INDEX (bulk scan + indexation) | OK |
| DROP_LUCIVY_INDEX (cleanup complet) | OK |
| Indexation incrementale (INSERT -> index auto) | OK |
| DELETE hook (suppression auto de l'index) | OK |
| UPDATE hook (delete + re-insert auto) | OK |
| Lazy commit (dirty flag, commit+reload avant QUERY) | OK |
| Mode fichier + in-memory | OK |
| Persistance (close DB -> reopen -> query) | OK |
| Persistance IDBFS browser (IndexedDB) | OK |
| Filtrage par node IDs (`allowed_ids`) | OK |
| Filter fields natifs (INT64, DOUBLE, etc.) | OK |
| 7 operateurs de filtre (eq, ne, lt, lte, gt, gte, in) | OK |
| Stemmer configurable (french, english, etc.) | OK |
| cxx bridge type (zero JSON sur hot path) | OK |

### Targets de build

| Fonctionnalite | Statut |
|----------------|--------|
| Natif Linux x86_64 | OK |
| Node.js natif (NAPI addon) | OK |
| WASM browser (Emscripten, pthreads, IDBFS) | OK |
| Vector HNSW (cosine, L2) en WASM | OK |
| Hybrid search (lucivy_fts contains + vector cosine) | OK |

---

## 3 builds disponibles

| Build | Output | Taille | Extensions lucivy_fts | Tests |
|-------|--------|--------|----------------------|-------|
| Natif (Linux x86_64) | `lucivy_fts_test` + `.rag3db_extension` | — | Dynamique (LOAD EXTENSION) | 11 GTest E2E |
| Node.js natif (NAPI) | `rag3dbjs.node` + `.rag3db_extension` | ~50MB | Dynamique (LOAD EXTENSION) | 139 mocha |
| WASM browser | `rag3db_wasm.js` (single file) | 17MB | Statique (auto-chargee) | 94 mocha + 2 Playwright |

---

## Arborescence actuelle

```
packages/rag3db/
├── BUILD.md                            <- Guide complet des builds
├── README.md                           <- Presentation du projet
├── extension/lucivy/
│   └── ld-lucivy/                     <- Submodule (fork Lucivy v0.26.0)
│       ├── README.md                   <- NgramContainsQuery + BM25 en avant
│       ├── src/                        <- Moteur Lucivy modifie (1025 tests)
│       │   └── query/phrase_query/
│       │       └── ngram_contains_query.rs  <- BM25 scoring
│       └── lucivy_fts/rust/src/
│           ├── bridge.rs               <- cxx bridge (9 structs, 15 fonctions)
│           ├── handle.rs               <- LucivyHandle, triple-field layout
│           └── query.rs                <- build_query, FilterClause, routing
├── extension/lucivy_fts/
│   ├── BUILD.md                        <- Architecture 3 couches, piege cmake/cargo
│   ├── CMakeLists.txt                  <- Build Rust + C++ + cxx (natif + WASM)
│   ├── src/
│   │   ├── main/lucivy_fts_extension.cpp
│   │   ├── index/lucivy_index.cpp     <- LucivyIndex, hooks, lazy commit
│   │   ├── catalog/
│   │   └── function/
│   │       ├── create_lucivy_index.cpp  <- filter_fields, add_document_mixed
│   │       ├── query_lucivy_index.cpp   <- allowed_ids, flushIfDirty, highlights
│   │       └── drop_lucivy_index.cpp
│   └── test/
│       └── lucivy_fts_test.cpp        <- 11 tests GTest E2E
├── extension/extension_config.cmake    <- WASM: json, vector, algo, lucivy_fts
├── tools/
│   ├── nodejs_api/                     <- Build + tests Node.js natif (139 mocha)
│   └── wasm/                           <- Build + tests WASM
│       ├── test/browser/               <- Tests Playwright IDBFS (2 tests)
│       └── build/rag3db/               <- Sortie WASM (17MB single file)
└── build/
    ├── release/                        <- Build natif
    ├── nodejs/                         <- Build Node.js
    └── wasm/                           <- Build WASM browser
```

---

## Commits pushes (session 14 fevrier)

| Repo | Branche | Commit | Contenu |
|------|---------|--------|---------|
| ld-lucivy | `main` | `4c4e7ad` | handle.rs + ngram_contains_query.rs (ngram sans stemmer + BM25) |
| ld-lucivy | `main` | `76ed60f` | README restructure (NgramContainsQuery + BM25 en avant) |
| ld-lucivy | `main` | `80159c1` | Contains unifie regex (VerificationMode, trigram accel, hybrid, full-scan) |
| ld-lucivy | `main` | `77c4ca6` | README mis a jour (regex mode, hybrid, full-scan fallback) |
| rag3db | `feature/fuzzy-fts` | `7ed62275e` | Extension C++ complete + 10 tests + BUILD.md + submodule update |
| rag3db | `feature/fuzzy-fts` | `e0186a809` | Regex contains E2E tests (7 sub-tests) + submodule update |

---

## Bugs corriges importants

| Bug | Cause | Fix |
|-----|-------|-----|
| cmake ne detecte pas les changements Rust | `add_custom_command` sans `DEPENDS` | Rebuild manuel : `cargo build --release` puis `cmake --build . --target rag3db_lucivy_fts_extension` |
| Buffer overflow magicBytes | MAGIC_BYTES="RAG3DB" (6 chars) mais buffer [4] | Buffer [8] + assert |
| getDatabasePath() retourne le fichier | Utilise pour le path index | `parent_path()` |
| miniconda LD_LIBRARY_PATH | Vieux libstdc++ injecte | `LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu` |
| f64 manquant dans handle.rs | build_schema() sans branche "f64" | Ajout du cas "f64" |
| Writer != reader Lucivy | Mutations invisibles sans commit | Lazy commit via `dirty_` flag |
| WASM cxx exceptions | build.rs sans `-fexceptions` | `-fexceptions -sDISABLE_EXCEPTION_CATCHING=0` pour emscripten |
| WASM atomics | Rust WASM sans support pthreads | nightly + `-Z build-std` + `+atomics,+bulk-memory` + `-C panic=abort` |
| BM25 scores tous a 1.0 | Extension pas re-linkee (piege cmake) | Rebuild cargo + cmake extension re-link |

---

## Commandes de build

```bash
# Tests ld-lucivy (1025 tests)
cd packages/rag3db/extension/lucivy/ld-lucivy && cargo test --lib

# Build natif + tests E2E (11 tests)
cd packages/rag3db/build/release
cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSION_TESTS=TRUE \
  -DBUILD_EXTENSIONS="lucivy_fts" -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE
cmake --build . --target lucivy_fts_test -j$(nproc)
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu ./extension/lucivy_fts/test/lucivy_fts_test

# Build Node.js natif (139 mocha)
cd packages/rag3db/build/nodejs
cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSIONS="lucivy_fts" \
  -DBUILD_NODEJS=TRUE -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE
cmake --build . --target rag3dbjs -j$(nproc)
cmake --build . --target rag3db_lucivy_fts_extension -j$(nproc)

# Build WASM browser (94 mocha + 2 Playwright)
cd packages/rag3db/build/wasm
source ~/emsdk/emsdk_env.sh
emcmake cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_WASM=TRUE \
  -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE -DBUILD_BENCHMARK=FALSE
emmake cmake --build . -j$(nproc)

# Apres modif Rust : rebuild manuel obligatoire
cd packages/rag3db/extension/lucivy/ld-lucivy && \
  cargo build --release -p ld-lucivy -p lucivy-fts && \
  cd ../../../build/release && \
  cmake --build . --target rag3db_lucivy_fts_extension -j$(nproc)
```

---

## Ce qui reste a faire

### Court terme

1. **Phase C : Integration Rag3Weaver** — wrapper Node.js/TypeScript :
   - Exposer `"contains"` (NgramContainsQuery) comme methode de recherche BM25 par defaut
   - Pipeline hybride : contains BM25 + vector cosine -> fusion RRF cote TypeScript
   - Les autres types (term, fuzzy, phrase, parse, regex) disponibles en option avancee
2. **npm publish** — packaging du WASM build pour distribution npm
3. **Stemming multi-langues** — le code est la mais pas teste en WASM pour toutes les langues

### Moyen terme

4. **CI/CD** — GitHub Actions pour build + tests auto (natif, Node.js, WASM, Playwright)
5. **Benchmarks** — perfs FTS sur datasets realistes
6. **Thread pool dynamique** — actuellement 16 Workers pre-crees au demarrage, a optimiser

### Limitations connues

- Le WASM single-file fait 17MB (compressible avec gzip/brotli)
- Lucivy utilise 1 seul writer thread en WASM (vs 8 en natif) pour eviter l'epuisement du pool de pthreads
- Les headers COOP/COEP sont obligatoires pour le navigateur (SharedArrayBuffer)
- L'ancienne extension `fts` (BM25 natif de Kuzu) n'est pas disponible en WASM

---

## Historique des docs

| Dossier | Docs | Contenu principal |
|---------|------|-------------------|
| `13-fevrier-2026-18h57/` | 9 docs | Phases 1-8, architecture, plan implementation, fix mode fichier |
| `14-fevrier-2026-00h58/` | 11 docs | Phase A/B, Node.js, WASM, Playwright, ngram+BM25, builds |
| **`14-fevrier-2026-20h36/`** | **7 docs** | Etat des lieux consolide + contains unifie regex |

### Detail des 11 docs de la session 00h58

| Doc | Contenu |
|-----|---------|
| 01-etat-des-lieux | Etat consolide (persistance, filtres, 7 tests) |
| 02-rapport-fin-de-session | DELETE/UPDATE en cours, lazy commit recommande |
| 03-plan-integration-rag3db-wasm | Decouverte : tout existe deja dans rag3db |
| 04-rapport-nodejs-natif-ok | Node.js natif valide (contains, fuzzy, phrase, parse) |
| 05-rapport-wasm-ok | WASM build valide, 5 bugs corriges |
| 06-point-etape-wasm-validations | Checkpoint : vector non teste, plan validation |
| 07-rapport-tests-complets | 139 mocha + 94 WASM + vector HNSW valide |
| 08-progression-tests-playwright-idbfs | Infrastructure Playwright, bug serve.js |
| 09-guide-builds-et-tests | Guide complet builds + tests + IDBFS + COOP/COEP |
| 10-plan-ngram-sans-stemmer-et-bm25-contains | Plan detaille des 2 changements |
| 11-progression-ngram-bm25 | TERMINE : ngram + BM25 implementes, testes, pushes |
