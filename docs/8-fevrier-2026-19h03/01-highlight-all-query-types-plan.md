# Highlight pour tous les types de queries — Plan d'implémentation

## Contexte

Le highlighting v1 (session du 6 février) couvre uniquement les **contains queries** :
- `NgramContainsQuery` → offsets capturés pendant la vérification du texte stocké (gratuit)
- `AutomatonPhraseQuery` fallback → idem via `ContainsScorer`

Les queries **term, fuzzy, regex, phrase, parse** ne retournent pas de highlights.

## État des champs indexés

| Champ | IndexRecordOption | Offsets dans les posting lists ? |
|-------|-------------------|----------------------------------|
| `{name}` (stemmed) | `WithFreqsAndPositionsAndOffsets` | **Oui** (upgradé Phase B1) |
| `{name}._raw` | `WithFreqsAndPositionsAndOffsets` | **Oui** |
| `{name}._ngram` | `Basic` | Non |

**Fait clé** : les deux champs texte (stemmed et `._raw`) stockent désormais les byte offsets `(from, to)` dans les posting lists. Ces offsets pointent vers le texte original (pas le token transformé). Il suffit de les lire via `SegmentPostings::append_offsets()`.

---

## Phase A — term / fuzzy / regex (queries sur `._raw`) — FAIT

### A1. TermQuery — FAIT

**Fichiers modifiés** :
- `ld-tantivy/src/query/term_query/term_query.rs` — `highlight_sink: Option<Arc<HighlightSink>>`, `with_highlight_sink()`, propagation vers TermWeight
- `ld-tantivy/src/query/term_query/term_weight.rs` — sink field, force `WithFreqsAndPositionsAndOffsets` quand sink présent, `next_segment()` dans `specialized_scorer()`
- `ld-tantivy/src/query/term_query/term_scorer.rs` — sink + segment_ord, `capture_offsets()` dans `advance()`, `seek()`, et `with_highlight_sink()` (initial doc)
- `tantivy_fts/rust/src/query.rs` — `build_term_query()` passe le sink

**Statut** : Code compilé, 1015 tests ld-tantivy passent. **FFI test FAIL** — les offsets ne sont pas retrouvés dans le sink après la recherche. Debug en cours (voir section "Debug").

### A2. FuzzyQuery + RegexQuery (via AutomatonWeight) — FAIT

**Fichiers modifiés** :
- `ld-tantivy/src/query/automaton_weight.rs` — sink field, `with_highlight_sink()`. Quand sink présent : pour chaque terme FST, lit postings avec `WithFreqsAndPositionsAndOffsets`, itère tous les docs et capture les offsets. Sinon : chemin block postings original (Basic)
- `ld-tantivy/src/query/fuzzy_query.rs` — sink field, `with_highlight_sink()`, custom `impl Debug` (exclut le sink pour éviter casser les tests de debug output), propagation vers AutomatonWeight
- `ld-tantivy/src/query/regex_query.rs` — sink field, `with_highlight_sink()`, propagation vers AutomatonWeight
- `tantivy_fts/rust/src/query.rs` — `build_fuzzy_query()` et `build_regex_query()` passent le sink

**Statut** : Code compilé, 1015 tests ld-tantivy passent. **FFI test PASS** — fuzzy highlight fonctionne correctement (`"rrust"~1` → `[0,4]`).

### A3. Pourquoi fuzzy marche mais term non

**Chemin fuzzy (MARCHE)** :
1. `AutomatonWeight` ne surcharge PAS `for_each_pruning`
2. → le défaut `Weight::for_each_pruning()` appelle `self.scorer(reader, boost)`
3. → `AutomatonWeight::scorer()` capture TOUS les offsets pendant la construction du scorer (dans la boucle term_stream)
4. → retourne `ConstScorer(BitSetDocSet)` — les offsets sont déjà dans le sink

**Chemin term (NE MARCHE PAS)** :
1. `TermWeight` SURCHARGE `for_each_pruning` avec `block_wand_single_scorer`
2. → `specialized_scorer()` crée un `TermScorer` avec highlight_sink
3. → `block_wand_single_scorer` itère avec `seek()` et `advance()` qui appellent `capture_offsets()`
4. → En théorie ça devrait marcher, mais les offsets ne sont pas retrouvés

**Hypothèse principale** : problème de matching segment_ord entre le sink et le DocAddress retourné par TopDocs. Ou bien `append_offsets` retourne un buffer vide (offsets_reader absent ou position_offset incorrect). Debug en cours.

---

## Phase B — phrase (query sur le champ stemmed) — FAIT (code)

### B1. PhraseQuery avec offsets — FAIT

**Approche choisie** : Option B1 — upgrader le champ stemmed à `WithFreqsAndPositionsAndOffsets`.

**Fichiers modifiés** :
- `tantivy_fts/rust/src/handle.rs` — champ stemmed upgradé de `WithFreqsAndPositions` → `WithFreqsAndPositionsAndOffsets`
- `ld-tantivy/src/query/phrase_query/phrase_scorer.rs` — sink + segment_ord, refactoring constructeurs (extracted `build()` method), `new_with_highlight()`, `drain_or_capture_offsets()` pour maintenir sync lecteurs position/offset
- `ld-tantivy/src/query/phrase_query/phrase_weight.rs` — sink field, force `WithFreqsAndPositionsAndOffsets`, crée PhraseScorer via `new_with_highlight()`
- `ld-tantivy/src/query/phrase_query/phrase_query.rs` — sink field, `with_highlight_sink()`, propagation vers PhraseWeight
- `tantivy_fts/rust/src/query.rs` — `build_phrase_query()` passe le sink

**Détail PhraseScorer** :
- Le PhraseScorer évalue `phrase_match()` pour chaque doc candidat (en lisant les positions)
- Les lecteurs position et offset sont des `PositionReader` séquentiels dans la même `SegmentPostings`
- `drain_or_capture_offsets(matched)` : pour CHAQUE doc candidat (matché ou non), consomme les offsets via `append_offsets()` pour garder le lecteur sync. Si matched, insère dans le sink.
- Appelé dans `build()` (initial doc), `advance()`, `seek()`, `seek_into_the_danger_zone()`

**Statut** : Code compilé, 1015 tests ld-tantivy passent. **FFI test FAIL** — même symptôme que term query (pas de highlights dans la réponse).

**Note** : En fait, le `PositionReader` supporte le random access (pas séquentiel pur), donc le draining n'est peut-être pas nécessaire. Mais ça ne devrait pas empêcher de marcher.

---

## Phase C — ParseQuery — DIFFÉRÉ

`ParseQuery` utilise le `QueryParser` de tantivy qui produit un arbre de queries. On ne peut pas facilement injecter un `HighlightSink` dans ces queries générées.

**Recommandation** : différer. Le highlight pour parse query peut venir plus tard.

---

## Debug en cours — 3 tests FFI FAIL

### Tests qui PASSENT
- `highlight on fuzzy query: has highlights` ✅
- `highlight on fuzzy query: rrust~1 → [0,4]` ✅
- Tous les tests contains highlight ✅

### Tests qui FAIL
- `highlight on term query: has highlights` ❌ — pas de clé "highlights" dans la réponse
- `highlight on term query: rust → [0,4]` ❌
- `highlight on phrase query: has highlights` ❌ — pas de clé "highlights" dans la réponse

### Analyse du flux

Le flux `collect_results()` fait :
```rust
let offsets = sink.get(doc_address.segment_ord, doc_address.doc_id)?;
```

Le `HighlightSink` stocke `(segment_ord, doc_id) → Vec<[usize; 2]>` où `segment_ord` vient de `sink.next_segment()` (AtomicU32 incrémenté).

**Question clé** : est-ce que `segment_ord` dans le sink correspond bien au `doc_address.segment_ord` retourné par TopDocs ?

**Différences de chemin Weight** :
- Fuzzy : `Weight::for_each_pruning()` (défaut) → `self.scorer()` → offsets capturés dans `scorer()`
- Term : `TermWeight::for_each_pruning()` (override) → `specialized_scorer()` → `block_wand_single_scorer` → offsets capturés pendant l'itération
- Phrase : `Weight::for_each_pruning()` (défaut) → `PhraseWeight::scorer()` → PhraseScorer itère et capture

**Prochaine étape** : ajouter du logging debug dans `capture_offsets()` et `collect_results()` pour vérifier :
1. Si `capture_offsets()` est appelé du tout
2. Si `append_offsets()` retourne des données non-vides
3. Si le segment_ord dans le sink matche celui de `doc_address`

---

## Résumé des phases

| Phase | Queries | Champ | Méthode | Statut |
|-------|---------|-------|---------|--------|
| **A1** | term | `._raw` | Lire offsets posting list | Code FAIT, **FFI FAIL** |
| **A2** | fuzzy, regex | `._raw` | Lire offsets dans AutomatonWeight | **FAIT + FFI PASS** |
| **B1** | phrase | stemmed (upgradé) | Lire offsets PhraseScorer | Code FAIT, **FFI FAIL** |
| **C** | parse | — | Différé | — |

## Fichiers modifiés (récapitulatif)

### ld-tantivy
- `src/query/term_query/term_query.rs` — sink + propagation
- `src/query/term_query/term_weight.rs` — sink + force record option + segment_ord
- `src/query/term_query/term_scorer.rs` — sink + capture_offsets dans advance/seek
- `src/query/automaton_weight.rs` — sink + capture offsets dans scorer()
- `src/query/fuzzy_query.rs` — sink + custom Debug
- `src/query/regex_query.rs` — sink + propagation
- `src/query/phrase_query/phrase_query.rs` — sink + propagation
- `src/query/phrase_query/phrase_weight.rs` — sink + force record option
- `src/query/phrase_query/phrase_scorer.rs` — sink + drain_or_capture_offsets + refactored constructors

### tantivy_fts
- `rust/src/query.rs` — sink passé à tous les builders (term, fuzzy, regex, phrase, contains)
- `rust/src/handle.rs` — stemmed field upgradé à WithFreqsAndPositionsAndOffsets
- `test/test_ffi.c` — 3 tests highlight ajoutés (term, fuzzy, phrase)

### Compilation
- 1015 tests ld-tantivy : tous passent
- 150/153 tests FFI : 3 FAIL (term + phrase highlights)
