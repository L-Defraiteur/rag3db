# Plan — Contains unifie : ajout du mode regex dans NgramContainsQuery

## Contexte

La `RegexQuery` actuelle de Lucivy fonctionne via un **FST walk** : elle parcourt le term dictionary entier du champ avec un automate, collecte les doc_ids dans un `BitSet`, et retourne un score constant (`ConstScorer`). C'est lent sur de gros index et ca ne donne pas de BM25.

L'objectif : integrer le support regex **dans** `NgramContainsQuery` (pas un nouveau type de query), en reutilisant le pipeline trigram + BM25 existant, et en ajoutant une verification hybride regex + fuzzy.

Voir `04-vision-contains-unifie.md` pour la vision complete et les decisions prises.

## Decouverte cle : `regex-syntax::hir::literal::Extractor`

L'extracteur de litteraux depuis un AST regex **existe deja** dans la crate `regex-syntax` (projet officiel `rust-lang/regex`), deja presente dans le dependency tree de ld-lucivy.

```rust
use regex_syntax::hir::literal::Extractor;
use regex_syntax::parse;

let hir = parse(r"program[a-z]+ing")?;
let seq = Extractor::new().extract(&hir);
// -> [Literal::inexact("program")]  (prefixe obligatoire)
```

L'`Extractor` gere alternations, repetitions, classes de caracteres, prefixes/suffixes.

**Dependances (toutes deja presentes, rien a ajouter au Cargo.lock)** :

| Crate | Version | Usage |
|-------|---------|-------|
| `regex-syntax` | 0.8.9 | Parser HIR + `Extractor` (extraction litteraux) |
| `regex` | 1.12.3 | `Regex::find_iter()` pour verification sur texte stocke |
| `lucivy-fst` | 0.5.0 | Automate FST (fallback quand pas de litteraux) |

Note : `regex-syntax` est une dependance transitive (via `regex` et `lucivy-fst`). Il faudra l'ajouter en dependance directe dans le `Cargo.toml` de ld-lucivy pour l'utiliser explicitement.

---

## Architecture : VerificationMode dans NgramContainsQuery

Pas de nouveau fichier. On ajoute une enum `VerificationMode` dans `ngram_contains_query.rs` :

```rust
enum VerificationMode {
    Fuzzy {
        tokens: Vec<String>,
        separators: Vec<String>,
        prefix: String,
        suffix: String,
        fuzzy_distance: u8,
        distance_budget: u32,
        strict_separators: bool,
    },
    Regex {
        compiled: regex::Regex,
        literals: Vec<String>,   // pour verification hybride fuzzy
        fuzzy_distance: u8,      // 0 = regex pur, > 0 = hybride
    },
}
```

Le `NgramContainsQuery` existant recoit un champ `verification: VerificationMode` et un champ `trigram_sources: Vec<String>` (tokens en mode fuzzy, litteraux en mode regex).

Le reste du pipeline (candidats trigram, BM25, highlights) est partage.

---

## Plan d'implementation

### Etape 1 : Refactoring de NgramContainsQuery — VerificationMode::Fuzzy

**Fichier** : `src/query/phrase_query/ngram_contains_query.rs`

Extraire les champs specifiques au mode fuzzy dans `VerificationMode::Fuzzy`. Le comportement actuel ne change pas, c'est un refactoring pur.

Avant :
```rust
struct NgramContainsQuery {
    raw_field: Field,
    ngram_field: Field,
    stored_field: Option<Field>,
    tokens: Vec<String>,
    separators: Vec<String>,
    prefix: String,
    suffix: String,
    fuzzy_distance: u8,
    distance_budget: u32,
    strict_separators: bool,
    highlight_sink: Option<Arc<HighlightSink>>,
}
```

Apres :
```rust
struct NgramContainsQuery {
    raw_field: Field,
    ngram_field: Field,
    stored_field: Option<Field>,
    trigram_sources: Vec<String>,    // tokens (fuzzy) ou litteraux (regex)
    verification: VerificationMode,
    highlight_sink: Option<Arc<HighlightSink>>,
}
```

Idem pour `NgramContainsWeight` et `NgramContainsScorer`.

**Validation** : `cargo test --lib` — les 1015 tests doivent passer sans changement de comportement.

### Etape 2 : VerificationMode::Regex — verification regex pure

**Fichier** : `src/query/phrase_query/ngram_contains_query.rs`

Ajouter la branche `VerificationMode::Regex` dans le scorer.

La methode `verify()` du scorer dispatch selon le mode :

```rust
fn verify(&mut self) -> bool {
    match &self.verification {
        VerificationMode::Fuzzy { .. } => {
            // Code existant (count_single_token / count_multi_token)
            let tf = /* ... existant ... */;
            self.last_tf = tf;
            tf > 0
        }
        VerificationMode::Regex { compiled, .. } => {
            let stored_text = self.load_stored_text();
            let matches: Vec<regex::Match> = compiled.find_iter(&stored_text).collect();
            self.last_tf = matches.len() as u32;
            // Highlights
            if let Some(ref sink) = self.highlight_sink {
                let offsets: Vec<[usize; 2]> = matches.iter()
                    .map(|m| [m.start(), m.end()])
                    .collect();
                sink.insert(self.segment_ord, self.doc(), offsets);
            }
            self.last_tf > 0
        }
    }
}
```

Le `score()` reste identique (BM25, partage entre les deux modes).

### Etape 3 : VerificationMode::Regex — verification hybride (regex + fuzzy)

**Fichier** : `src/query/phrase_query/ngram_contains_query.rs`

Quand `VerificationMode::Regex` a `fuzzy_distance > 0`, la verification fait les deux :

```rust
VerificationMode::Regex { compiled, literals, fuzzy_distance } => {
    let stored_text = self.load_stored_text();

    // 1. Verification regex exacte
    let regex_matches: Vec<regex::Match> = compiled.find_iter(&stored_text).collect();
    let tf_regex = regex_matches.len() as u32;

    // 2. Verification fuzzy sur les litteraux (si distance > 0)
    let tf_fuzzy = if *fuzzy_distance > 0 {
        let doc_tokens = self.tokenize_stored(&stored_text);
        literals.iter()
            .map(|lit| self.count_fuzzy_matches(lit, &doc_tokens, *fuzzy_distance))
            .max()
            .unwrap_or(0)
    } else {
        0
    };

    // tf = max des deux
    self.last_tf = std::cmp::max(tf_regex, tf_fuzzy);

    // Highlights : utiliser les offsets regex si disponibles, sinon fuzzy
    if let Some(ref sink) = self.highlight_sink {
        let offsets = if tf_regex > 0 {
            regex_matches.iter().map(|m| [m.start(), m.end()]).collect()
        } else {
            /* offsets fuzzy */
            vec![]  // a implementer
        };
        if !offsets.is_empty() {
            sink.insert(self.segment_ord, self.doc(), offsets);
        }
    }

    self.last_tf > 0
}
```

### Etape 4 : Routing dans query.rs

**Fichier** : `lucivy_fts/rust/src/query.rs`

Adapter `build_contains_query()` pour gerer le champ `"regex"` du JSON.

```rust
fn build_contains_query(
    config: &QueryConfig,
    schema: &Schema,
    index: &Index,
    raw_pairs: &[(String, String)],
    ngram_pairs: &[(String, String)],
    highlight_sink: Option<Arc<HighlightSink>>,
) -> Result<Box<dyn Query>, String> {
    let is_regex = config.regex.unwrap_or(false);

    if is_regex {
        build_contains_regex(config, schema, index, raw_pairs, ngram_pairs, highlight_sink)
    } else {
        // Code actuel (fuzzy contains) — inchange
        build_contains_fuzzy(config, schema, index, raw_pairs, ngram_pairs, highlight_sink)
    }
}
```

La nouvelle fonction `build_contains_regex()` :

1. Parse le pattern avec `regex_syntax::parse(pattern)`
2. Compile le regex avec `regex::Regex::new(pattern)`
3. Extrait les litteraux avec `Extractor::new().extract(&hir)`
4. Filtre : garde les litteraux >= 3 chars
5. Si litteraux suffisants ET ngram field disponible :
   - `trigram_sources` = litteraux
   - `verification` = `VerificationMode::Regex { compiled, literals, fuzzy_distance }`
   - Cree `NgramContainsQuery`
6. Sinon (fallback) :
   - FST walk via `RegexQuery` pour le candidat collection
   - Mais wrap dans une structure qui fait quand meme la verification BM25 sur texte stocke

Ajouter `regex: Option<bool>` dans `QueryConfig` (deserialization serde).

### Etape 5 : Fallback FST walk avec BM25

Quand les litteraux sont trop courts (< 3 chars) ou absents, on ne peut pas utiliser les trigrams. Mais on veut quand meme du BM25.

**Option pragmatique** : creer le `NgramContainsQuery` sans trigram_sources. Le `Weight::scorer()` detecte que trigram_sources est vide et fait un FST walk (via `AutomatonWeight`) pour obtenir les candidats, puis passe ces candidats au meme `NgramContainsScorer` avec verification regex + BM25.

Ca necessite d'adapter la collecte de candidats dans `NgramContainsWeight::scorer()` :

```rust
let candidates = if self.trigram_sources.is_empty() {
    // Fallback : FST walk sur le term dictionary du champ raw
    self.collect_candidates_fst_walk(reader)?
} else {
    // Fast path : trigram intersection
    self.collect_candidates_ngram(reader)?
};
```

### Etape 6 : Tests

#### 6a. Tests Rust unitaires (`cargo test --lib`)

Ajouter dans `ngram_contains_query.rs` ou un module de test dedie :

1. **Refactoring OK** : les tests existants passent (fuzzy contains inchange)
2. **Regex basique** : pattern `"program[a-z]+"` matche "programming", pas "the cat sat"
3. **Regex sans litteraux** : pattern `"[a-z]+"` -> fallback FST, resultats corrects
4. **Regex + BM25** : deux docs avec des frequences differentes -> scores differents
5. **Regex + fuzzy hybride** : pattern `"programing[a-z]+"` distance=1 matche "programming" (le regex echoue, le fuzzy rattrape)
6. **Highlights regex** : offsets corrects des matchs
7. **Litteraux courts** (< 3 chars) : fallback FST, BM25 quand meme

#### 6b. Tests GTest E2E (`lucivy_fts_test.cpp`)

Ajouter un test `LucivyRegexContainsTest` :

```cpp
// Regex accelere par trigrams
auto r1 = conn->query(
    "CALL QUERY_LUCIVY_INDEX('doc', "
    "'{\"type\":\"contains\",\"field\":\"body\","
    "\"value\":\"program[a-z]+\",\"regex\":true}', 10) "
    "RETURN node_id, score, highlights");
// -> matche les docs contenant "programming", "programmer", etc.
// -> score BM25 (pas constant)

// Regex + fuzzy hybride
auto r2 = conn->query(
    "CALL QUERY_LUCIVY_INDEX('doc', "
    "'{\"type\":\"contains\",\"field\":\"body\","
    "\"value\":\"programing[a-z]+\",\"regex\":true,\"distance\":1}', 10) "
    "RETURN node_id, score");
// -> matche "programming" malgre la typo dans le pattern

// Regex fallback (litteral trop court)
auto r3 = conn->query(
    "CALL QUERY_LUCIVY_INDEX('doc', "
    "'{\"type\":\"contains\",\"field\":\"body\","
    "\"value\":\"v[0-9]+\",\"regex\":true}', 10) "
    "RETURN node_id, score");
// -> fallback FST, BM25 quand meme
```

#### 6c. Rebuild et validation complete

```bash
# 1. Tests Rust
cd extension/lucivy/ld-lucivy && cargo test --lib

# 2. Rebuild Rust + extension (piege cmake)
cargo build --release -p ld-lucivy -p lucivy-fts
cd ../../../build/release
cmake --build . --target rag3db_lucivy_fts_extension -j$(nproc)
cmake --build . --target lucivy_fts_test -j$(nproc)

# 3. Tests GTest E2E
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu ./extension/lucivy_fts/test/lucivy_fts_test
```

---

## Fichiers modifies (recapitulatif)

| Fichier | Changement |
|---------|------------|
| `src/query/phrase_query/ngram_contains_query.rs` | `VerificationMode` enum, adapter verify/count/score, fallback FST |
| `lucivy_fts/rust/src/query.rs` | `regex` dans `QueryConfig`, `build_contains_regex()`, routing |
| `Cargo.toml` (ld-lucivy) | Ajouter `regex-syntax = "0.8"` en dependance directe |
| `extension/lucivy_fts/test/lucivy_fts_test.cpp` | Nouveau test `LucivyRegexContainsTest` |

**Pas de nouveau fichier Rust.** Tout reste dans `ngram_contains_query.rs`.

---

## Cas limites

| Pattern | Litteraux | Trigrams | Candidats | Verification |
|---------|----------|----------|-----------|-------------|
| `program[a-z]+ing` | `["program"]` | 5 trigrams | Ngram fast path | Regex exact |
| `programing[a-z]+` (distance=1) | `["programing"]` | 7 trigrams (threshold reduit) | Ngram fast path | Hybride : regex + fuzzy |
| `v[0-9]+\.[0-9]+` | `["v"]` | trop court | Fallback FST | Regex exact + BM25 |
| `foo\|bar` | `["foo", "bar"]` | union | Ngram fast path | Regex exact |
| `[a-z]+` | `[]` | aucun | Fallback FST | Regex exact + BM25 |
| `log_.*_error` | `["log_"]` | 2 trigrams | Ngram fast path | Regex exact |
| `\bprogramming\b` | `["programming"]` | 9 trigrams | Ngram fast path | Regex exact |

---

## Estimation

- **Etape 1** (refactoring VerificationMode::Fuzzy) : ~1h — pas de changement de comportement, refactoring pur
- **Etape 2** (VerificationMode::Regex pure) : ~1h — verification regex, BM25 partage
- **Etape 3** (verification hybride regex+fuzzy) : ~1h — logique de double verification + highlights
- **Etape 4** (routing query.rs) : ~30min — parsing regex-syntax, extraction litteraux, QueryConfig
- **Etape 5** (fallback FST + BM25) : ~1h — adapter le candidat collection
- **Etape 6** (tests) : ~1h — Rust unitaires + GTest E2E

**Total** : ~5-6h de travail

- **Risque** : faible — reutilise le pattern existant, pas de nouveau fichier, dependances deja presentes
- **Gain** : regex 10-100x plus rapide sur gros index (trigrams vs FST walk), BM25 sur toutes les queries, verification hybride unique
