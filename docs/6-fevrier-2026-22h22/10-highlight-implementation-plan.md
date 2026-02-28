# 10 — Plan d'implémentation du Highlighting

Ref: [09-highlighting-architecture.md](./09-highlighting-architecture.md)

## Principe fondamental

**Les byte offsets sont un sous-produit gratuit de la vérification.**

Les scorers calculent _deja_ les byte offsets pendant le scoring :
- `NgramContainsScorer` appelle `tokenize_raw(stored_text)` qui retourne des `Vec<(usize, usize)>` — ce sont les byte ranges dans le texte original
- `ContainsScorer` lit `positions_and_offsets()` depuis les postings (`WithFreqsAndPositionsAndOffsets`) qui donnent `(position, byte_from, byte_to)`
- `ContainsSingleScorer` appelle `tokenize_raw()` et itere les `(start, end)`

Il suffit de **stocker ces offsets au lieu de les jeter** quand un match est confirme. Zero re-tokenisation. Zero travail supplementaire.

---

## Etat actuel

### Code en place
- `NgramContainsQuery` dans `ld-tantivy/src/query/phrase_query/ngram_contains_query.rs` (chemin ngram, principal)
- `AutomatonPhraseQuery` + `ContainsScorer` + `ContainsSingleScorer` dans `ld-tantivy/src/query/phrase_query/` (chemin cascade, fallback)
- `scoring_utils.rs` dans `ld-tantivy/src/query/phrase_query/` (fonctions partagees)
- `tantivy_fts/rust/src/query.rs` : `QueryConfig.highlight: Option<bool>` et `SearchResult.highlights: Option<HashMap<String, Vec<[usize;2]>>>` deja en place (mais `highlights` toujours `None`)
- 1015 tests ld-tantivy, 129 tests FFI — tout passe

### Architecture des scorers

```
tantivy_fts/query.rs::build_contains_query()
  ├── ngram_field disponible → NgramContainsQuery (chemin rapide)
  │     └─ NgramContainsWeight::scorer()
  │          └─ NgramContainsScorer (verify via stored text + tokenize_raw)
  │               ├─ verify_single_token() : (start, end) par token
  │               └─ verify_at_position() : doc_tokens[start_idx + q_idx] = (start, end)
  │
  └── pas de ngram_field → AutomatonPhraseQuery (fallback FST cascade)
        └─ AutomatonPhraseWeight::scorer()
             ├─ multi-token → ContainsScorer (postings intersection + stored text)
             │    └─ validate_separators() : token_offsets Vec<(usize,usize)> depuis postings ou tokenize_raw
             └─ single-token → ContainsSingleScorer (BitSet + stored text)
                  └─ validate_current() : tokenize_raw → (start, end) par token
```

---

## Architecture du highlighting

### Side-channel : `HighlightSink`

Un `Arc<HighlightSink>` partage entre le code appelant et les scorers :

```rust
// Dans scoring_utils.rs
pub struct HighlightSink {
    data: Mutex<HashMap<(u32, DocId), Vec<[usize; 2]>>>,
    segment_counter: AtomicU32,
}
```

- **Cle** : `(segment_ord, doc_id)` — identifie un document de facon unique
- **Valeur** : `Vec<[usize; 2]>` — byte ranges des tokens matches dans le texte original
- `segment_counter` : compteur atomique incremente a chaque appel a `Weight::scorer()` (1 appel = 1 segment, sequentiel)
- Quand `highlight` est `false` ou absent : le sink n'est pas cree, zero overhead

### Pourquoi `(segment_ord, doc_id)` et pas `DocAddress` ?

`DocAddress { segment_ord, doc_id }` est le type Tantivy mais il derive `Hash + Eq`. On pourrait l'utiliser directement. **Mais** le scorer ne connait pas son `segment_ord` — `SegmentReader` ne l'expose pas. La solution : un compteur atomique dans le sink, incremente a chaque `Weight::scorer()` (appele exactement 1 fois par segment, dans l'ordre). Le scorer recoit son `segment_ord` a la construction.

### Flux

```
1. tantivy_fts : config.highlight == true
   → creer Arc<HighlightSink>
   → passer au Query via with_highlight_sink()

2. Query::weight() → Weight avec le sink

3. Weight::scorer(reader)
   → segment_ord = sink.next_segment()
   → scorer recoit (sink, segment_ord)

4. Scorer pendant le scoring
   → quand un match est confirme, insere dans le sink :
     sink.insert(segment_ord, doc_id, offsets)

5. Apres la recherche
   → tantivy_fts lit le sink
   → pour chaque (score, DocAddress) dans les resultats :
     offsets = sink.get(doc_address.segment_ord, doc_address.doc_id)
   → remplir SearchResult.highlights
```

---

## Plan fichier par fichier

### Etape 1 : `scoring_utils.rs` — ajouter HighlightSink

**Fichier** : `ld-tantivy/src/query/phrase_query/scoring_utils.rs`

Ajouter :
```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};
use crate::DocId;

/// Side-channel for highlight byte offsets, shared between caller and scorers.
pub struct HighlightSink {
    data: Mutex<HashMap<(u32, DocId), Vec<[usize; 2]>>>,
    segment_counter: AtomicU32,
}

impl HighlightSink {
    pub fn new() -> Self { ... }

    /// Called by Weight::scorer() — returns the segment_ord for this segment.
    pub fn next_segment(&self) -> u32 {
        self.segment_counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Called by scorers when a match is confirmed.
    pub fn insert(&self, segment_ord: u32, doc_id: DocId, offsets: Vec<[usize; 2]>) {
        self.data.lock().unwrap()
            .insert((segment_ord, doc_id), offsets);
    }

    /// Called after search to retrieve offsets for a result.
    pub fn get(&self, segment_ord: u32, doc_id: DocId) -> Option<Vec<[usize; 2]>> {
        self.data.lock().unwrap()
            .get(&(segment_ord, doc_id))
            .cloned()
    }
}
```

**Visibilite** : `pub` (utilise depuis tantivy_fts).

### Etape 2 : `NgramContainsQuery` — ajouter capture d'offsets

**Fichier** : `ld-tantivy/src/query/phrase_query/ngram_contains_query.rs`

Modifications :

1. **NgramContainsQuery** : ajouter champ `highlight_sink: Option<Arc<HighlightSink>>`
   - Nouveau constructeur `with_highlight_sink(mut self, sink: Arc<HighlightSink>) -> Self`
   - Propager dans `Query::weight()` vers `NgramContainsWeight`

2. **NgramContainsWeight** : ajouter champ `highlight_sink: Option<Arc<HighlightSink>>`
   - Dans `scorer()` : `let segment_ord = sink.as_ref().map(|s| s.next_segment());`
   - Propager vers `NgramContainsScorer`

3. **NgramContainsScorer** : ajouter champs `highlight_sink: Option<Arc<HighlightSink>>` + `segment_ord: u32`
   - **`verify_single_token()`** ligne 376 — avant `return true` :
     ```rust
     if let Some(ref sink) = self.highlight_sink {
         sink.insert(self.segment_ord, self.doc(), vec![[start, end]]);
     }
     return true;
     ```
   - **`verify_at_position()`** ligne 484 — avant `true` :
     ```rust
     if let Some(ref sink) = self.highlight_sink {
         let offsets: Vec<[usize; 2]> = (0..self.tokens.len())
             .map(|i| {
                 let (s, e) = doc_tokens[start_idx + i];
                 [s, e]
             })
             .collect();
         sink.insert(self.segment_ord, self.doc(), offsets);
     }
     true
     ```
   - `verify()` change `&self` → `&mut self` ? Non — `verify` est appele dans `advance()` qui est `&mut self`. Mais `verify` est `&self` actuellement. Pas de probleme : `HighlightSink::insert` prend `&self` (Mutex interne).

### Etape 3 : `AutomatonPhraseQuery` + Weight + Scorers — ajouter capture d'offsets

**Fichier** : `ld-tantivy/src/query/phrase_query/automaton_phrase_query.rs`

1. **AutomatonPhraseQuery** : ajouter champ `highlight_sink: Option<Arc<HighlightSink>>`
   - Methode `with_highlight_sink(mut self, sink: Arc<HighlightSink>) -> Self`
   - Propager dans `automaton_phrase_weight()` vers `AutomatonPhraseWeight::new()`

**Fichier** : `ld-tantivy/src/query/phrase_query/automaton_phrase_weight.rs`

2. **AutomatonPhraseWeight** : ajouter champ `highlight_sink: Option<Arc<HighlightSink>>`
   - `new()` : ajouter parametre
   - `phrase_scorer()` : `let segment_ord = self.highlight_sink.as_ref().map(|s| s.next_segment()).unwrap_or(0);`
     - Passer `(highlight_sink.clone(), segment_ord)` au `ContainsScorer::new()`
   - `single_token_scorer()` : meme chose pour `ContainsSingleScorer::new()`
   - **Attention** : `Weight::scorer()` dispatch vers `phrase_scorer` ou `single_token_scorer`. Le `next_segment()` doit etre appele exactement 1 fois par appel a `scorer()`. Donc l'appeler dans `scorer()` avant le dispatch :
     ```rust
     fn scorer(&self, reader, boost) -> ... {
         let segment_ord = self.highlight_sink.as_ref().map(|s| s.next_segment()).unwrap_or(0);
         if self.phrase_terms.len() <= 1 {
             self.single_token_scorer(reader, boost, segment_ord)
         } else {
             self.phrase_scorer(reader, boost, segment_ord)
         }
     }
     ```

**Fichier** : `ld-tantivy/src/query/phrase_query/contains_scorer.rs`

3. **ContainsScorer** : ajouter champs `highlight_sink: Option<Arc<HighlightSink>>` + `segment_ord: u32`
   - `new()` : ajouter parametres
   - `validate_separators()` ligne 351 — quand `count += 1` :
     ```rust
     if let Some(ref sink) = self.highlight_sink {
         let offsets: Vec<[usize; 2]> = token_offsets.iter()
             .map(|&(from, to)| [from, to])
             .collect();
         sink.insert(self.segment_ord, self.intersection_docset.doc(), offsets);
     }
     count += 1;
     ```
   - Cas sans validation (ligne 142, `count += 1` dans la boucle simple) : pas de byte offsets disponibles a ce point (juste positions, pas offsets). **Deux options** :
     - **Option A (simple, v1)** : ne pas highlighter les matchs sans validation (cas rare — pas de separateurs/prefix/suffix = query simple sans ponctuation)
     - **Option B** : lire les offsets depuis les postings meme dans le cas sans validation. Plus complet mais plus de code.
     - **Choix v1 : Option A.** La grande majorite des contains queries ont des separateurs (sinon c'est juste un term/fuzzy match).

4. **ContainsSingleScorer** : ajouter champs `highlight_sink: Option<Arc<HighlightSink>>` + `segment_ord: u32`
   - `new()` : ajouter parametres
   - `validate_current()` ligne 525 — avant `return true` :
     ```rust
     if let Some(ref sink) = self.highlight_sink {
         sink.insert(self.segment_ord, self.bitset_docset.doc(), vec![[start, end]]);
     }
     return true;
     ```

### Etape 4 : Exports et re-exports

**Fichier** : `ld-tantivy/src/query/phrase_query/mod.rs`

- `scoring_utils` passe de `pub(crate)` a `pub` (pour que tantivy_fts puisse importer `HighlightSink`)

**Fichier** : `ld-tantivy/src/query/mod.rs`

- Ajouter : `pub use self::phrase_query::scoring_utils::HighlightSink;`

### Etape 5 : Wiring dans tantivy_fts

**Fichier** : `tantivy_fts/rust/src/query.rs`

1. **Import** : `use ld_tantivy::query::HighlightSink;` + `use std::sync::Arc;`

2. **`build_query()`** : ajouter parametre `highlight_sink: Option<Arc<HighlightSink>>`
   - Propager aux sous-fonctions `build_contains_query`, `build_boolean_query`

3. **`build_contains_query()`** : si `highlight_sink` est Some :
   ```rust
   let mut query = NgramContainsQuery::new(...);
   if let Some(sink) = highlight_sink {
       query = query.with_highlight_sink(sink);
   }
   // idem pour AutomatonPhraseQuery
   ```

4. **`collect_results()`** : ajouter parametre `highlight_sink: Option<&HighlightSink>`, `field_name: Option<&str>`
   - Pour chaque resultat `(score, doc_address)` :
     ```rust
     let highlights = highlight_sink.and_then(|sink| {
         let offsets = sink.get(doc_address.segment_ord, doc_address.doc_id)?;
         let mut map = HashMap::new();
         map.insert(field_name.unwrap().to_string(), offsets);
         Some(map)
     });
     ```
   - `field_name` = le champ user original (ex: `"body"` et non `"body._raw"`)

5. **`execute_search()` et `execute_search_filtered()`** : ajouter parametres et propager

**Fichier** : `tantivy_fts/rust/src/lib.rs`

6. **`tantivy_search()` et `tantivy_search_filtered()`** :
   ```rust
   let highlight_sink = if config.highlight.unwrap_or(false) {
       Some(Arc::new(HighlightSink::new()))
   } else {
       None
   };
   let query = build_query(&config, &h.schema, &h.index, &h.raw_field_pairs, &h.ngram_field_pairs, highlight_sink.clone())?;
   // ...
   let results = execute_search(&searcher, query.as_ref(), limit, &h.schema, highlight_sink.as_deref(), field_name)?;
   ```

---

## Scope v1 : contains seulement

Le highlighting v1 couvre uniquement les queries `contains` (NgramContainsQuery + AutomatonPhraseQuery fallback). C'est le cas d'usage principal de ragforge.

Les autres types de queries retournent `highlights: None` :
- `term`, `fuzzy` : possible en v2 via lecture postings en post-processing
- `phrase`, `parse` : possible en v2 via re-tokenisation stemmed (seul cas justifie)
- `regex` : skip
- `boolean` : en v2, union des highlights des sous-queries

---

## Tests

### Tests unitaires ld-tantivy

Dans `ngram_contains_query.rs` (section `#[cfg(test)]`) :
1. **single token + highlight** : chercher "world" dans "hello world" → `[[6, 11]]`
2. **multi token + highlight** : chercher "hello world" → `[[0, 5], [6, 11]]`
3. **fuzzy + highlight** : chercher "wrold" (d=1) → `[[6, 11]]` (le byte range du token original "world")
4. **no highlight (sink = None)** : verifier que le scorer fonctionne normalement sans sink

Dans `automaton_phrase_weight.rs` (section `#[cfg(test)]`) :
5. **cascade exact + highlight** : "hello" exact → offsets
6. **cascade fuzzy + highlight** : "helo" → offsets de "hello"

### Tests FFI tantivy_fts

Dans `test/test_ffi.c` (~8 tests) :
7. `{"type":"contains","field":"body","value":"world","highlight":true}` → verifier `highlights.body = [[6,11]]`
8. `{"type":"contains","field":"body","value":"hello world","highlight":true}` → `[[0,5],[6,11]]`
9. `{"type":"contains","field":"body","value":"c++","highlight":true}` → `[]` (pas de match, le test "c++" existant)
10. `{"type":"contains","field":"body","value":"world","highlight":false}` → pas de champ `highlights` dans le JSON
11. `{"type":"contains","field":"body","value":"world"}` → idem (highlight absent = false)
12. `{"type":"term","field":"body","value":"world","highlight":true}` → `highlights: null` ou absent (v1 = contains only)
13. `{"type":"contains","field":"body","value":"programm","highlight":true}` → byte range de "programming" si match fuzzy/substring
14. Multi-document : verifier que les highlights sont corrects pour chaque doc individuellement

---

## Verification

```bash
# 1. Tests unitaires ld-tantivy (devrait passer ~1025+ tests)
cd packages/rag3db/extension/tantivy/ld-tantivy && cargo test --lib

# 2. Clippy
cd packages/rag3db/extension/tantivy/ld-tantivy && cargo clippy --features mmap,stopwords,lz4-compression,stemmer --no-default-features -- -D warnings -A clippy::uninlined_format_args -A clippy::identity_op -A clippy::let_and_return -A clippy::redundant_closure -A clippy::too_many_arguments -A clippy::assertions_on_constants -A dead_code -A unused_imports

# 3. Build + tests FFI
cd packages/rag3db/extension/tantivy_fts/rust && cargo build --release
cd ../test && cc -o test_ffi test_ffi.c -I../include -L../rust/target/release -ltantivy_fts -lpthread -lm -ldl && LD_LIBRARY_PATH=../rust/target/release ./test_ffi
```

---

## Resume : 5 etapes, 7 fichiers

| # | Etape | Fichiers | Lignes estimees |
|---|-------|----------|-----------------|
| 1 | HighlightSink dans scoring_utils | scoring_utils.rs | +40 |
| 2 | Capture dans NgramContainsScorer | ngram_contains_query.rs | +30 |
| 3 | Capture dans Contains + Single + propagation APQ | automaton_phrase_query.rs, automaton_phrase_weight.rs, contains_scorer.rs | +50 |
| 4 | Exports | mod.rs (x2) | +3 |
| 5 | Wiring tantivy_fts | query.rs, lib.rs | +30 |
| - | Tests unitaires + FFI | 3 fichiers | +150 |
