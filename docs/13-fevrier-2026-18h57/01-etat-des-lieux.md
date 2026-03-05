# État des lieux — 13 février 2026

> Consolidation de toutes les sessions depuis le 1er février 2026.
> Dernière vérification : 13 février, 1015 tests ld-lucivy + 153 tests FFI = tout vert, build rag3db C++ OK.

---

## Vue d'ensemble

Le projet vise à intégrer un moteur de recherche full-text (Lucivy) dans un fork de Kuzu (graph DB), renommé **rag3db**, pour servir de backend à **Rag3Weaver** (framework RAG TypeScript).

### Repos

| Repo | Branche | Description |
|------|---------|-------------|
| `rag3db` | `feature/fuzzy-fts` | Fork Kuzu v0.11.2.2, renommé rag3db |
| `ld-lucivy` | `main` | Fork Lucivy v0.26.0, submodule de rag3db |

### Arborescence clé

```
packages/rag3db/                          ← Fork Kuzu, renommé rag3db
├── extension/lucivy/
│   └── ld-lucivy/                       ← Submodule git (fork Lucivy)
│       ├── src/                          ← Moteur Lucivy modifié
│       │   ├── query/
│       │   │   ├── phrase_query/
│       │   │   │   ├── automaton_phrase_query.rs
│       │   │   │   ├── automaton_phrase_weight.rs
│       │   │   │   ├── contains_scorer.rs
│       │   │   │   ├── scoring_utils.rs  ← HighlightSink
│       │   │   │   └── phrase_scorer.rs
│       │   │   ├── fuzzy_substring_automaton.rs
│       │   │   └── automaton_weight.rs
│       │   ├── schema/index_record_option.rs  ← WithFreqsAndPositionsAndOffsets
│       │   └── postings/                      ← Offsets dans les postings
│       └── lucivy_fts/                  ← Crate FFI (dans le submodule)
│           ├── rust/src/
│           │   ├── lib.rs                ← 13 fonctions extern "C"
│           │   ├── handle.rs             ← Gestion index, tri-field layout
│           │   └── query.rs              ← build_query, build_contains_query
│           ├── include/lucivy_fts.h     ← Header C (cbindgen)
│           └── test/test_ffi.c           ← 153 tests C
├── extension/lucivy_fts/
│   └── CMakeLists.txt                    ← Build CMake → submodule
├── src/                                  ← Code C++ rag3db (ex-Kuzu)
└── build/release/                        ← Build C++ vérifié OK
```

---

## Travail complété

### Phase 1 : Crate Rust FFI `lucivy_fts` (1er-6 février)

13 fonctions `extern "C"` exposant Lucivy via une API C :
- `lucivy_create_index` / `lucivy_open_index` / `lucivy_close_index`
- `lucivy_add_document` / `lucivy_commit` / `lucivy_delete_by_term`
- `lucivy_search` / `lucivy_search_filtered` (JSON in/out)
- `lucivy_num_docs` / `lucivy_free_string` / `lucivy_last_error`
- `lucivy_reload_searcher` / `lucivy_optimize`

**Tri-field layout** automatique : pour chaque champ texte `body`, l'index crée :
- `body` — stemmed (English stemmer, pour recall : "run" → "running")
- `body._raw` — lowercase only (pour precision : term exact, fuzzy, regex, contains)
- `body._ngram` — trigrams (pour NgramContainsQuery rapide)

Le routage est transparent : l'utilisateur référence `body`, le code dirige vers le bon sous-champ selon le type de query.

### Phase 2 : Build CMake (6 février)

`CMakeLists.txt` dans rag3db appelle `cargo build --release` sur le workspace ld-lucivy et link `liblucivy_fts.a`.

### Phase 3 : WithFreqsAndPositionsAndOffsets (6-7 février)

Nouveau variant `IndexRecordOption::WithFreqsAndPositionsAndOffsets` dans ld-lucivy :
- Les byte offsets (`offset_from`, `offset_to`) de chaque token sont stockés dans les postings
- 21 fichiers modifiés, 1015 tests passent
- Nécessaire pour la validation des séparateurs et le highlighting

### Phase 4 : ContainsQuery — Option B scorer custom (7 février)

Cascade **4 niveaux** par token, avec early termination :

| Niveau | Méthode | Coût distance |
|--------|---------|---------------|
| 1. Exact | Lookup direct term dict | 0 |
| 2. Fuzzy | Levenshtein DFA (d ≤ 2) | d |
| 3. Substring | Regex `.*{escaped}.*` | 0 |
| 4. Fuzzy Substring | NFA `.*{levenshtein(token,d)}.*` | d |

**FuzzySubstringAutomaton** (`src/query/fuzzy_substring_automaton.rs`) :
- Simulation NFA : à chaque byte du FST, maintient un ensemble d'états Levenshtein actifs
- Nouveau walk démarre à chaque byte (= prefix `.*`)
- Une fois un état accepting atteint, match permanent (= suffix `.*`)
- Résout le cas "progam" → "programming" (fuzzy d=1 de "program" qui est substring)

**ContainsScorer** (`src/query/phrase_query/contains_scorer.rs`) — multi-token :
1. Intersection des posting lists (positions consécutives, slop=0)
2. Pour chaque match candidat : charge le texte stocké, extrait les séparateurs réels via byte offsets
3. Compare séparateurs query vs doc (edit distance), accumule dans un budget
4. Valide prefix/suffix aux bords
5. Si distance totale ≤ budget → match confirmé

**ContainsSingleScorer** — single-token (cas "c++") :
- Tokenize "c++" → ["c"] avec suffix "++"
- Le scorer vérifie que "++" apparaît après "c" dans le document
- Résout le problème : "c++" ne matche plus "custom", "music", etc.

### Phase 5 : NgramContainsQuery (7 février)

Alternative rapide quand un champ `._ngram` est disponible :
- Lookup des trigrams dans l'index ngram (très rapide, pas de FST walk)
- Puis vérification dans le texte stocké (même logique ContainsScorer)

### Phase 6 : HighlightSink (7-8 février)

Side-channel pour capturer les byte offsets pendant le scoring :
- `HighlightSink` = `Mutex<HashMap<(segment_ord, DocId), Vec<[usize; 2]>>>` + `AtomicU32`
- Supporté pour **tous** les types de query : term, fuzzy, regex, phrase, contains

| Query type | Scorer | Source des offsets |
|------------|--------|--------------------|
| Term | TermScorer | `capture_offsets()` dans advance()/seek() |
| Fuzzy | AutomatonWeight | Collecte upfront dans scorer construction |
| Regex | AutomatonWeight | Idem fuzzy |
| Phrase | PhraseScorer | `drain_or_capture_offsets()` |
| Contains | ContainsScorer | Byte offsets des postings + texte stocké |

**Bug fix critique** : `next_segment()` doit être appelé pour CHAQUE segment (même vides) pour rester synchronisé avec les ordinals de TopDocs. Fix dans `term_weight.rs` et `phrase_weight.rs`.

### Phase 7 : Réorganisation repos (8 février)

1. `lucivy_fts/` (crate FFI) déplacé dans le repo `ld-lucivy` (workspace member)
2. `ld-lucivy` ajouté comme submodule git de `rag3db`
3. Remote upstream `kuzudb` supprimé de rag3db

### Phase 8 : Rename kuzu → rag3db (8-13 février)

- Remplacement de toutes les occurrences `kuzu`/`Kuzu`/`KUZU` → `rag3db`/`Rag3db`/`RAG3DB`
- ~2538 fichiers modifiés, 29 fichiers/dossiers renommés
- Fix `kuzudb` → `rag3db` (pas `rag3dbdb`)
- Fix fichiers `.cc`/`.tcc` manqués par le premier sed (snappy, thrift)
- Fix fichiers `.test`, `.benchmark`, `.mjs`, `.html`, etc.
- **Build vérifié** : compile à 100% sans erreur
- **Committé et pushé** sur `feature/fuzzy-fts` le 13 février

---

## Tests actuels

| Suite | Résultat | Commande |
|-------|----------|----------|
| ld-lucivy lib | 1015 pass, 0 fail | `cd ld-lucivy && cargo test --lib` |
| lucivy_fts FFI | 153 pass, 0 fail | `cc test_ffi.c ... && ./test_ffi` |
| rag3db C++ build | 100% OK | `cmake --build build/release` |

---

## Ce qui reste à faire

### Phase A : Extension C++ dans rag3db

Créer les fonctions Cypher pour piloter Lucivy depuis rag3db :

```cypher
-- Créer un index FTS sur une table de noeuds
CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body'], {fuzzy := true});

-- Rechercher avec contains (auto-cascade + séparateurs)
CALL QUERY_LUCIVY_INDEX('doc', '{"type":"contains","field":"body","value":"c++"}')
RETURN node_id, score, highlights;

-- Supprimer l'index
CALL DROP_LUCIVY_INDEX('doc');
```

**Fichiers à créer** dans `extension/lucivy_fts/` :
- `src/lucivy_fts_extension.cpp` — Point d'entrée extension
- `src/lucivy_fts_functions.cpp` — Implémentation des fonctions Cypher
- `src/include/lucivy_fts_functions.h` — Déclarations

**Lien avec le FFI** : les fonctions C++ appellent les 13 fonctions `extern "C"` de `liblucivy_fts.a`.

### Phase B : Tests end-to-end

Pipeline complet : Cypher → C++ extension → FFI C → Lucivy Rust
- Tests d'intégration dans `extension/lucivy_fts/test/`
- Cas : create index, add docs, search (term/fuzzy/contains/phrase), highlights, delete

### Phase C : Intégration Rag3Weaver

Adapter le framework RAG TypeScript pour utiliser rag3db comme backend :
- Remplacer/compléter le CatalogSearch existant
- Utiliser les fonctions Cypher de Phase A
- Pipeline : ingest → index → search avec highlights

---

## Approches abandonnées

| Approche | Statut | Raison d'abandon |
|----------|--------|------------------|
| fuzzy-fst (lib standalone) | Code complet | Lucivy fait déjà tout ça nativement, redondant |
| Summa/Lucivy WASM | Bloqué | writer_threads issue, architecture trop lourde |
| Option A (cascade sans fuzzy substring) | Remplacée | Option B (fuzzy substring) plus puissante, implémentée |
| Séparateurs comme tokens dans l'index | Abandonnée | Validation post-hoc via byte offsets plus propre |

---

## Commandes de build

```bash
# Tests ld-lucivy (1015 tests)
cd packages/rag3db/extension/lucivy/ld-lucivy
cargo test --lib

# Build + tests FFI lucivy_fts (153 tests)
cd packages/rag3db/extension/lucivy/ld-lucivy
cargo build --release -p lucivy-fts
cd lucivy_fts/test
cc -o test_ffi test_ffi.c -I../include -L../../target/release \
   -llucivy_fts -lpthread -lm -ldl
./test_ffi

# Build rag3db C++ (release, sans extensions)
cd packages/rag3db
mkdir -p build/release && cd build/release
cmake ../.. -DCMAKE_BUILD_TYPE=Release \
    -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE -DBUILD_EXTENSIONS=""
cmake --build . -j$(nproc)
```

---

## Historique des sessions documentées

| Date | Dossier/Fichier | Contenu principal |
|------|-----------------|-------------------|
| 1er fév | `kuzu-wasm-exp/docs/2026-02-01-08h38/rag3db-fuzzy-fst/` | Design initial (00-08), fuzzy-fst, Summa, roadmap |
| 6 fév | `docs/6-fevrier-2026-22h22/` | Architecture storage, API FFI, offsets plan |
| 7 fév 14h | `SAVE_CONTEXT_7_fevrier_14h23.md` | ContainsScorer phase 2 |
| 7 fév 16h | `SAVE_CONTEXT_7_fevrier_16h50.md` | FuzzySubstringAutomaton, Option A vs B |
| 7 fév 18h | `SAVE_CONTEXT_7_fevrier_18h01.md` | NgramFilter, tri-field |
| 7 fév 19h | `SAVE_CONTEXT_7_fevrier_19h50.md` | Fix highlighting (mauvaise approche → HighlightSink) |
| 8 fév 19h | `docs/8-fevrier-2026-19h03/` | État des lieux + highlight plan |
| 8 fév 19h | `SAVE_CONTEXT_8_fevrier_19h44.md` | HighlightSink A1+A2+B1, 3 tests FAIL |
| 8 fév 21h | `SAVE_CONTEXT_8_fevrier_21h36.md` | Fix segment_ord, 153/153, rename kuzu→rag3db |
| 13 fév | **Ce document** | Consolidation complète, build vérifié |
