# Plan : Ngram sans stemmer + BM25 scoring pour contains

## Objectif

Deux changements dans tantivy_fts :

1. **Ngram toujours actif** : les champs `._raw` et `._ngram` doivent etre crees meme sans stemmer, pour que le contains utilise toujours le fast path (trigram lookup + verification) au lieu du fallback AutomatonPhraseQuery.

2. **BM25 scoring sur contains** : remplacer le score constant (`self.boost = 1.0`) par un vrai score BM25(k1=1.2, b=0.75) qui prend en compte la frequence du terme dans le document et la longueur du champ.

---

## Etat actuel

### Schema sans stemmer (le probleme)

```
CREATE_TANTIVY_INDEX('docs', ['body'])
→ 1 champ : body (tokenizer "default", TEXT | STORED)
→ raw_field_pairs = []
→ ngram_field_pairs = []
→ contains → fallback AutomatonPhraseQuery (lent, FST walks)
```

### Schema avec stemmer (ce qu'on veut toujours)

```
CREATE_TANTIVY_INDEX('docs', ['body'], stemmer := 'french')
→ 3 champs : body (stemmed), body._raw (lowercase), body._ngram (trigrams)
→ raw_field_pairs = [("body", "body._raw")]
→ ngram_field_pairs = [("body", "body._ngram")]
→ contains → NgramContainsQuery (fast, trigram lookup)
```

### Scoring actuel

```rust
// ngram_contains_query.rs:558-562
impl Scorer for NgramContainsScorer {
    fn score(&mut self) -> Score {
        self.boost  // toujours 1.0
    }
}
```

---

## Changement 1 : Ngram sans stemmer

### Fichier : `ld-tantivy/tantivy_fts/rust/src/handle.rs`

#### 1a. `build_schema()` — toujours creer raw + ngram

**Avant** (lignes 195-243) : le `if has_stemmer` cree 3 champs, le `else` en cree 1.

**Apres** : toujours creer les 3 champs. Le champ principal utilise le tokenizer "default" quand il n'y a pas de stemmer (au lieu de "stemmed").

```rust
"text" => {
    // Champ principal : stemmed si stemmer, sinon default
    let main_tokenizer = if has_stemmer { STEMMED_TOKENIZER } else { "default" };
    let indexing = TextFieldIndexing::default()
        .set_tokenizer(main_tokenizer)
        .set_index_option(IndexRecordOption::WithFreqsAndPositionsAndOffsets);
    let mut opts = TextOptions::default().set_indexing_options(indexing);
    if field_def.stored.unwrap_or(true) {
        opts = opts.set_stored();
    }
    let field = builder.add_text_field(&field_def.name, opts);
    field_map.push((field_def.name.clone(), field));

    // Raw counterpart : TOUJOURS (lowercase only)
    let raw_indexing = TextFieldIndexing::default()
        .set_tokenizer("default")
        .set_index_option(IndexRecordOption::WithFreqsAndPositionsAndOffsets);
    let raw_opts = TextOptions::default().set_indexing_options(raw_indexing);
    let raw_name = format!("{}{RAW_SUFFIX}", field_def.name);
    let raw_field = builder.add_text_field(&raw_name, raw_opts);
    field_map.push((raw_name.clone(), raw_field));
    raw_field_pairs.push((field_def.name.clone(), raw_name));

    // Ngram counterpart : TOUJOURS (trigrams)
    let ngram_indexing = TextFieldIndexing::default()
        .set_tokenizer(NGRAM_TOKENIZER)
        .set_index_option(IndexRecordOption::Basic);
    let ngram_opts = TextOptions::default().set_indexing_options(ngram_indexing);
    let ngram_name = format!("{}{NGRAM_SUFFIX}", field_def.name);
    let ngram_field = builder.add_text_field(&ngram_name, ngram_opts);
    field_map.push((ngram_name.clone(), ngram_field));
    ngram_field_pairs.push((field_def.name.clone(), ngram_name));
}
```

**Note** : sans stemmer, le champ principal et le champ `._raw` utilisent tous les deux le tokenizer "default". C'est redondant en stockage mais ca evite de casser la logique de routing dans `query.rs` qui attend `raw_field_pairs` non-vide pour les queries contains/fuzzy/term.

#### 1b. `configure_tokenizers()` — toujours enregistrer ngram

**Avant** (lignes 307-340) : le tokenizer ngram n'est enregistre que dans le `if let Some(ref stemmer_lang)`.

**Apres** : enregistrer le tokenizer ngram inconditionnellement.

```rust
fn configure_tokenizers(index: &Index, config: &SchemaConfig) {
    use ld_tantivy::tokenizer::{LowerCaser, SimpleTokenizer, TextAnalyzer};
    use crate::tokenizer::NgramFilter;

    // N-gram tokenizer : TOUJOURS enregistre
    let ngram_tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(LowerCaser)
        .filter(NgramFilter)
        .build();
    index.tokenizers().register(NGRAM_TOKENIZER, ngram_tokenizer);

    // Stemmer : seulement si demande
    if let Some(ref stemmer_lang) = config.stemmer {
        use ld_tantivy::tokenizer::Stemmer;

        let lang = match stemmer_lang.as_str() {
            "english" => ld_tantivy::tokenizer::Language::English,
            "french" => ld_tantivy::tokenizer::Language::French,
            // ... autres langues ...
            _ => return,
        };

        let tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(LowerCaser)
            .filter(Stemmer::new(lang))
            .build();
        index.tokenizers().register(STEMMED_TOKENIZER, tokenizer);
    }
}
```

#### 1c. Commentaires du struct `TantivyHandle`

Mettre a jour les commentaires des champs `raw_field_pairs` et `ngram_field_pairs` :
- Avant : "Only populated when a stemmer is active"
- Apres : "Always populated for text fields"

#### 1d. Commentaire du module

Mettre a jour le header `handle.rs` ligne 6-9 :
- Avant : "When a stemmer is configured, every text field gets a dual-field layout"
- Apres : "Every text field gets a triple-field layout"

---

## Changement 2 : BM25 scoring pour NgramContainsQuery

### Approche

Suivre le pattern de `TermQuery` / `TermWeight` / `TermScorer` de Tantivy :

1. Dans `NgramContainsQuery::weight()` : calculer `Bm25Weight` via `EnableScoring`
2. Passer le `Bm25Weight` a `NgramContainsWeight`
3. Dans `NgramContainsWeight::scorer()` : lire le `FieldNormReader` du segment
4. Dans `NgramContainsScorer` : compter les occurrences pendant `verify()`, scorer via BM25

### Fichier : `ld-tantivy/src/query/phrase_query/ngram_contains_query.rs`

#### 2a. Imports

Ajouter :

```rust
use crate::fieldnorm::FieldNormReader;
use crate::query::bm25::Bm25Weight;
use crate::query::EnableScoring;
```

#### 2b. `NgramContainsQuery::weight()` — calculer BM25Weight

**Avant** :

```rust
impl Query for NgramContainsQuery {
    fn weight(&self, _enable_scoring: EnableScoring) -> crate::Result<Box<dyn Weight>> {
        Ok(Box::new(NgramContainsWeight { ... }))
    }
}
```

**Apres** :

```rust
impl Query for NgramContainsQuery {
    fn weight(&self, enable_scoring: EnableScoring) -> crate::Result<Box<dyn Weight>> {
        // Construire un Term sur le raw_field pour chaque query token,
        // puis calculer le BM25Weight a partir des stats de l'index.
        let bm25_weight = match enable_scoring {
            EnableScoring::Enabled { statistics_provider, .. } => {
                let terms: Vec<Term> = self.tokens.iter()
                    .map(|t| Term::from_field_text(self.raw_field, t))
                    .collect();
                if terms.is_empty() {
                    Bm25Weight::for_one_term(0, 1, 1.0)
                } else {
                    Bm25Weight::for_terms(statistics_provider, &terms)?
                }
            }
            EnableScoring::Disabled { .. } => {
                Bm25Weight::for_one_term(0, 1, 1.0)
            }
        };

        Ok(Box::new(NgramContainsWeight {
            raw_field: self.raw_field,
            ngram_field: self.ngram_field,
            stored_field: self.stored_field,
            tokens: self.tokens.clone(),
            separators: self.separators.clone(),
            prefix: self.prefix.clone(),
            suffix: self.suffix.clone(),
            fuzzy_distance: self.fuzzy_distance,
            distance_budget: self.distance_budget,
            strict_separators: self.strict_separators,
            highlight_sink: self.highlight_sink.clone(),
            bm25_weight,
        }))
    }
}
```

#### 2c. `NgramContainsWeight` — ajouter bm25_weight + passer fieldnorm au scorer

```rust
struct NgramContainsWeight {
    // ... champs existants ...
    bm25_weight: Bm25Weight,
}
```

Dans `scorer()`, lire le `FieldNormReader` et le passer au scorer :

```rust
fn scorer(&self, reader: &SegmentReader, boost: Score) -> crate::Result<Box<dyn Scorer>> {
    // ... collecte des candidats (inchange) ...

    let fieldnorm_reader = reader.get_fieldnorms_reader(self.raw_field)?;

    Ok(Box::new(NgramContainsScorer::new(
        final_candidates,
        store_reader,
        text_field,
        self.tokens.clone(),
        self.separators.clone(),
        self.prefix.clone(),
        self.suffix.clone(),
        self.fuzzy_distance,
        self.distance_budget,
        self.strict_separators,
        self.bm25_weight.boost_by(boost),
        fieldnorm_reader,
        self.highlight_sink.clone(),
        segment_ord,
    )))
}
```

#### 2d. `NgramContainsScorer` — remplacer boost par BM25

```rust
struct NgramContainsScorer {
    // ... champs existants ...
    // REMPLACER :
    // boost: Score,
    // PAR :
    bm25_weight: Bm25Weight,
    fieldnorm_reader: FieldNormReader,
    last_tf: u32,  // cache du term frequency du doc courant
}
```

#### 2e. `verify()` — compter les occurrences (term frequency)

Modifier `verify()` pour retourner le nombre de matchs au lieu d'un bool, et le cacher dans `last_tf`.

**Avant** :

```rust
fn verify(&self) -> bool {
    // ... retourne true au premier match
}
```

**Apres** :

```rust
fn verify(&mut self) -> bool {
    let tf = self.count_matches();
    self.last_tf = tf;
    tf > 0
}
```

La fonction `count_matches()` reprend la logique de `verify_single_token` / `verify_multi_token` mais **compte tous les matchs** au lieu de s'arreter au premier. Ca implique :

- `verify_single_token` : boucler sur tous les doc_tokens, incrementer un compteur a chaque match (au lieu de `return true`)
- `verify_multi_token` : boucler sur toutes les positions, incrementer un compteur a chaque fenetre qui matche
- Les highlights sont collectes pour TOUS les matchs (pas seulement le premier)

#### 2f. `score()` — BM25

```rust
impl Scorer for NgramContainsScorer {
    fn score(&mut self) -> Score {
        let doc = self.doc();
        let fieldnorm_id = self.fieldnorm_reader.fieldnorm_id(doc);
        self.bm25_weight.score(fieldnorm_id, self.last_tf)
    }
}
```

---

## Tests

### Tests Rust unitaires (ld-tantivy)

Ajouter des tests dans `ngram_contains_query.rs` ou un fichier de test dedie :

1. **Ngram sans stemmer** : creer un index sans stemmer, verifier que contains utilise NgramContainsQuery (pas AutomatonPhraseQuery)
2. **BM25 score ordering** : inserer des docs avec des frequences differentes du meme terme, verifier que le score BM25 est plus eleve pour le doc avec plus d'occurrences
3. **BM25 vs boost constant** : verifier que le score n'est plus constant (score doc_A != score doc_B quand les longueurs ou frequences different)

### Tests GTest E2E (tantivy_fts_test.cpp)

Les 9 tests existants doivent continuer de passer sans modification :
- Les tests sans stemmer vont maintenant creer des champs `._raw` et `._ngram` — plus de donnees indexees mais meme comportement
- Les scores vont changer (de 1.0 a des valeurs BM25) — verifier que les tests ne comparent pas les scores a 1.0

**A verifier** : si les tests E2E font des assertions sur les valeurs de `score`, il faudra les adapter.

### Tests WASM (Playwright)

Les tests browser (contains, fuzzy) doivent passer sans modification. Ils ne testent que le nombre de resultats, pas les valeurs de score.

---

## Impact sur la taille de l'index

Sans stemmer, on passe de **1 champ** a **3 champs** par colonne texte. Impact :
- Le champ `._raw` est identique au champ principal (meme tokenizer "default") → ~2x les postings
- Le champ `._ngram` ajoute les trigrams → ~3-5x le nombre de termes, mais en `Basic` (doc IDs seulement, pas de positions/offsets)

Pour un index typique, l'augmentation de taille est d'environ **3-4x** sur les champs texte. C'est acceptable vu le gain de performance sur les recherches contains.

---

## Ordre d'implementation

1. **Changement 1** (handle.rs) — ngram sans stemmer (~20 min)
2. Lancer `cargo test --lib` pour verifier les 1015 tests Rust
3. **Changement 2** (ngram_contains_query.rs) — BM25 scoring (~1h)
4. Lancer `cargo test --lib` pour verifier
5. Build natif + tests GTest E2E (adapter si assertions sur score)
6. Build WASM + tests Playwright (verification)

---

## Fichiers modifies

| Fichier | Changement |
|---------|------------|
| `ld-tantivy/tantivy_fts/rust/src/handle.rs` | Triple-field layout toujours, tokenizer ngram toujours |
| `ld-tantivy/src/query/phrase_query/ngram_contains_query.rs` | BM25Weight, FieldNormReader, count_matches, score BM25 |
| `ld-tantivy/tantivy_fts/test/tantivy_fts_test.cpp` | Adapter assertions score si necessaire |
