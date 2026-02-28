# 09 — Highlighting : architecture et plan d'implémentation

## Objectif

Retourner les **byte ranges** des tokens matchés dans le texte stocké, pour permettre au caller de faire du highlighting.

Output attendu :
```json
[{
  "score": 1.23,
  "doc": {"body": ["Rust programming is great"]},
  "highlights": {"body": [[5, 16]]}
}]
```

`[5, 16]` = byte range dans le texte original stocké.

---

## Deux chemins de recherche, deux sources d'offsets

### 1. Chemin ngram (principal, tantivy_fts → à déplacer dans ld-tantivy)

`NgramContainsQuery` : trigram lookup + vérification stored text.

```
Query token "program"
  → trigrams ["pro","rog","ogr","gra","ram"]
  → lookup ngram field → candidate doc_ids
  → intersect candidates entre tokens
  → verify() : tokenize_raw(stored_text) → byte offsets des tokens
  → token_match_distance() → match ?
  → [résultat : match confirmé + byte offsets DÉJÀ DISPONIBLES]
```

**Les byte offsets sont un sous-produit gratuit de la vérification.** `tokenize_raw()` n'est pas du travail en double — c'est la logique de vérification elle-même. Il suffit de **stocker les offsets au lieu de les jeter**.

### 2. Chemin cascade (fallback, ld-tantivy)

`AutomatonPhraseQuery` : cascade exact→fuzzy→substring→fuzzy_substring sur le FST.

```
Query token "program"
  → cascade_term_infos() → TermInfos des termes matchés
  → postings intersection → positions consécutives ?
  → ContainsScorer.validate_separators()
    → positions_and_offsets() → (position, byte_from, byte_to) depuis les postings
    → [résultat : match confirmé + byte offsets DÉJÀ DISPONIBLES]
```

**Les byte offsets viennent des postings** (`WithFreqsAndPositionsAndOffsets` sur le raw field). Le `ContainsScorer` les lit déjà pour la validation des séparateurs. Même chose : stocker au lieu de jeter.

---

## Pourquoi pas de re-tokenisation en post-processing

Une première tentative (erronée) a implémenté le highlighting comme post-processing dans `collect_results` : après la recherche, pour chaque résultat, re-tokeniser le texte stocké.

**C'est faux parce que :**
1. Le raw field a `WithFreqsAndPositionsAndOffsets` — les offsets sont dans l'index
2. Les scorers lisent déjà ces offsets (ContainsScorer) ou les calculent pendant la vérification (NgramContainsScorer)
3. Re-tokeniser refait le travail que les scorers ont déjà fait
4. Ça ne donne pas la même qualité : le post-processing ne sait pas quel niveau de cascade a matché (exact, fuzzy, substring), donc il peut highlighter des tokens qui n'ont pas réellement contribué au match

---

## Plan d'implémentation

### Étape 1 : Déplacer NgramContainsQuery dans ld-tantivy

Actuellement dans `tantivy_fts/rust/src/ngram_query.rs`. À déplacer dans `ld-tantivy/src/query/ngram_contains/` (ou `src/query/phrase_query/ngram_contains_scorer.rs`).

Fichiers à créer/déplacer dans ld-tantivy :
- `ngram_contains_query.rs` — NgramContainsQuery (impl Query)
- `ngram_contains_weight.rs` — NgramContainsWeight (impl Weight) + NgramContainsScorer (impl DocSet + Scorer)

Fonctions utilitaires à inclure :
- `generate_trigrams(token) -> Vec<String>`
- `ngram_threshold(num_trigrams, fuzzy_distance) -> usize`
- `tokenize_raw(text) -> Vec<(usize, usize)>` (existe déjà dans contains_scorer.rs)
- `edit_distance(a, b) -> u32` (existe déjà dans contains_scorer.rs)
- `token_match_distance(doc, query, fuzzy_d) -> Option<u32>`
- `contains_fuzzy_substring(text, pattern, max_d) -> bool`

Note : `tokenize_raw` et `edit_distance` sont dupliqués entre contains_scorer.rs et ngram_query.rs. Factoriser dans un module utilitaire commun (`src/query/contains_utils.rs` ou similaire).

tantivy_fts gardera juste le routing dans `build_contains_query()` :
```rust
if ngram_field disponible {
    NgramContainsQuery::new(raw_field, ngram_field, stored_field, ...)
} else {
    AutomatonPhraseQuery::new_with_separators(...)
}
```

### Étape 2 : Ajouter le stockage d'offsets dans les scorers

**Pattern commun** : side-channel via `Option<Arc<Mutex<HashMap<DocId, Vec<[usize; 2]>>>>>`.

- Injecté dans le scorer à la construction (None = pas de highlighting, zéro overhead)
- Rempli pendant le scoring quand un match est confirmé
- Lu après le search dans `collect_results`

#### NgramContainsScorer (déplacé dans ld-tantivy)

Dans `verify_single_token()` — quand `token_match_distance()` retourne Some :
```rust
// Avant : return true;
// Après :
if let Some(ref sink) = self.highlight_sink {
    sink.lock().unwrap().insert(self.doc(), vec![[start, end]]);
}
return true;
```

Dans `verify_at_position()` — quand tous les tokens matchent et les séparateurs sont validés :
```rust
if let Some(ref sink) = self.highlight_sink {
    let offsets: Vec<[usize; 2]> = (0..self.tokens.len())
        .map(|i| {
            let (s, e) = doc_tokens[start_idx + i];
            [s, e]
        })
        .collect();
    sink.lock().unwrap().insert(self.doc(), offsets);
}
return true;
```

#### ContainsScorer (ld-tantivy, fallback multi-token)

Dans `validate_separators()` — quand `count += 1` (ligne 395) :
```rust
if let Some(ref sink) = self.highlight_sink {
    let offsets: Vec<[usize; 2]> = token_offsets.iter()
        .map(|&(from, to)| [from, to])
        .collect();
    sink.lock().unwrap()
        .entry(self.intersection_docset.doc())
        .or_default()
        .extend(offsets);
}
count += 1;
```

#### ContainsSingleScorer (ld-tantivy, fallback single-token)

Dans `validate_current()` — quand match trouvé (avant `return true`) :
```rust
if let Some(ref sink) = self.highlight_sink {
    sink.lock().unwrap().insert(self.bitset_docset.doc(), vec![[start, end]]);
}
```

### Étape 3 : Propager le sink depuis Query → Weight → Scorer

```
AutomatonPhraseQuery                  NgramContainsQuery
  └─ highlight_sink: Option<...>        └─ highlight_sink: Option<...>
       ↓                                     ↓
  AutomatonPhraseWeight               NgramContainsWeight
  └─ highlight_sink: Option<...>        └─ highlight_sink: Option<...>
       ↓                                     ↓
  ContainsScorer / ContainsSingle      NgramContainsScorer
  └─ highlight_sink: Option<...>        └─ highlight_sink: Option<...>
```

Méthode sur chaque Query pour injecter le sink :
```rust
pub fn with_highlight_sink(mut self, sink: Arc<Mutex<HashMap<DocId, Vec<[usize; 2]>>>>) -> Self {
    self.highlight_sink = Some(sink);
    self
}
```

### Étape 4 : Wiring dans tantivy_fts

**query.rs :**
- `build_query()` accepte un `highlight_sink: Option<Arc<...>>`
- Le passe au NgramContainsQuery ou AutomatonPhraseQuery via `with_highlight_sink()`

**query.rs — collect_results :**
- Après le search, si highlight_sink existe, le lire pour remplir `SearchResult.highlights`
- Le sink mappe `DocId` → offsets, mais on a des `DocAddress` (segment_id + doc_id). Il faudra adapter le mapping.

**lib.rs — tantivy_search / tantivy_search_filtered :**
- Si `config.highlight == Some(true)`, créer le sink, le passer à build_query, le lire après search.

### Étape 5 : Cas term / fuzzy / phrase / parse

| Query type | Source d'offsets | Approche |
|-----------|-----------------|----------|
| term | Postings raw field (WithFreqsAndPositionsAndOffsets) | Après search, pour chaque résultat, lire les postings du terme → offsets |
| fuzzy | Postings raw field | Idem, mais il faut trouver quel terme a matché (DFA walk sur term dict du segment) |
| contains | Scorer (ngram ou cascade) | Side-channel (étapes 2-3) |
| phrase | Champ stemmed (WithFreqsAndPositions, PAS d'offsets byte) | Re-tokeniser avec le pipeline stemmer pour corréler positions → byte offsets. C'est le seul cas justifié. |
| parse | Idem phrase | Idem |
| regex | Skip v1 | Retourner highlights vide |
| boolean | Union des sub-queries | Agréger les sinks des sous-queries |

Pour **term** et **fuzzy**, pas besoin de side-channel — on peut lire les postings directement en post-processing puisqu'on connaît le terme (ou on le retrouve via DFA). C'est un simple lookup dans l'inverted index du segment.

Pour **phrase/parse**, le champ stemmed n'a pas de byte offsets dans les postings. Option future : ajouter `WithFreqsAndPositionsAndOffsets` au champ stemmed aussi (modification schema dans handle.rs). Pour l'instant, re-tokeniser avec le pipeline stemmer est acceptable — c'est le seul cas.

---

## Mapping DocId → DocAddress

Le sink utilise `DocId` (u32, local à un segment). Mais `collect_results` travaille avec `DocAddress` (segment_ord + doc_id).

Solution : utiliser `DocAddress` comme clé du sink au lieu de `DocId`. Le scorer connaît son segment (via le `SegmentReader` passé au Weight). Passer le `segment_ord` au scorer à la construction.

Ou plus simple : utiliser un sink par segment. Le collector itère les segments séquentiellement, donc pas de conflit.

---

## Résumé des fichiers

### ld-tantivy (à modifier/créer)

| Fichier | Action |
|---------|--------|
| `src/query/phrase_query/ngram_contains_query.rs` | **NOUVEAU** — déplacé depuis tantivy_fts |
| `src/query/phrase_query/ngram_contains_weight.rs` | **NOUVEAU** — déplacé depuis tantivy_fts |
| `src/query/phrase_query/contains_scorer.rs` | **MODIFIER** — ajouter highlight_sink |
| `src/query/phrase_query/automaton_phrase_weight.rs` | **MODIFIER** — ajouter highlight_sink, propager |
| `src/query/phrase_query/automaton_phrase_query.rs` | **MODIFIER** — ajouter highlight_sink, with_highlight_sink() |
| `src/query/phrase_query/mod.rs` | **MODIFIER** — déclarer nouveaux modules |
| `src/query/mod.rs` | **MODIFIER** — re-exporter NgramContainsQuery |
| `src/query/contains_utils.rs` | **NOUVEAU** (optionnel) — factoriser tokenize_raw, edit_distance, etc. |

### tantivy_fts (à modifier/simplifier)

| Fichier | Action |
|---------|--------|
| `rust/src/ngram_query.rs` | **SUPPRIMER** — déplacé dans ld-tantivy |
| `rust/src/matching.rs` | **SUPPRIMER** — utilitaires déplacés dans ld-tantivy |
| `rust/src/highlight.rs` | **SUPPRIMER** — approche fausse |
| `rust/src/query.rs` | **MODIFIER** — build_contains_query utilise NgramContainsQuery de ld-tantivy, ajouter wiring sink |
| `rust/src/lib.rs` | **MODIFIER** — supprimer mod ngram_query/matching/highlight, ajouter wiring sink |
| `test/test_ffi.c` | **MODIFIER** — garder tests contains existants, refaire tests highlight |
