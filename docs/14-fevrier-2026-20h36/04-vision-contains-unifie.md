# Vision — Contains unifie : fuzzy + regex + BM25 + trigrams

## Principe

Un seul type de query `"contains"` qui gere tout : recherche fuzzy, regex, et les deux combines. Meme pipeline trigram, meme BM25, meme highlights. Le mode de verification est selectionne par un flag `"regex": true/false` (defaut: false).

## API

### Fuzzy contains (comportement actuel, inchange)

```json
{"type":"contains", "field":"body", "value":"programing", "distance":1}
```

- Tokenize le texte -> trigrams -> candidats -> verification fuzzy (Levenshtein) -> BM25
- Gere les separateurs (`c++`, `std::collections`, `os.path.join`)
- `distance` defaut: 1

### Regex contains (nouveau)

```json
{"type":"contains", "field":"body", "value":"program[a-z]+ing", "regex":true}
```

- Parse le pattern -> extrait les litteraux obligatoires -> trigrams -> candidats -> verification regex -> BM25
- Fallback FST walk si les litteraux sont trop courts (< 3 chars)
- `distance` defaut: 0 (regex exact)

### Regex + fuzzy combines (le cas interessant)

```json
{"type":"contains", "field":"body", "value":"programing[a-z]+", "regex":true, "distance":1}
```

- Trigrams des litteraux avec threshold reduit (fuzzy_distance=1 -> plus de candidats)
- Verification hybride : regex exact OU fuzzy sur les litteraux
- Un document matche si le regex passe, OU si les parties litterales matchent en fuzzy
- `tf` = max des deux verifications

## Pipeline unifie

```
                         Input query JSON
                                |
                    ┌───────────┴───────────┐
                    │ regex: false           │ regex: true
                    │ (defaut)              │
                    ▼                        ▼
            Tokenize texte            regex_syntax::parse()
            -> tokens                 -> Hir
            -> separateurs            Extractor::extract()
                    │                 -> litteraux obligatoires
                    │                        │
                    └───────────┬────────────┘
                                │
                    generate_trigrams(sources)
                                │
                    ngram_candidates_for_token()
                    (intersection posting lists ._ngram)
                    fuzzy_distance abaisse le threshold
                                │
                    ┌───────────┴───────────┐
                    │ litteraux >= 3 chars   │ litteraux < 3 chars
                    │ -> candidats ngram     │ -> fallback FST walk
                    │                        │    (RegexQuery actuelle)
                    │                        │    puis verification
                    └───────────┬────────────┘
                                │
                    Pour chaque candidat :
                    load stored text (store_reader)
                                │
                    ┌───────────┴───────────┐
                    │ regex: false           │ regex: true
                    ▼                        ▼
            token_match_distance()    Verification hybride :
            (fuzzy Levenshtein)       1. regex::find_iter() -> tf_regex
            -> count all matches      2. si distance > 0 :
            -> tf                        fuzzy sur litteraux -> tf_fuzzy
                    │                 -> tf = max(tf_regex, tf_fuzzy)
                    │                        │
                    └───────────┬────────────┘
                                │
                    BM25 score(fieldnorm_id, tf)
                    Highlights (byte offsets)
                                │
                            Resultats
```

## Decisions prises

### 1. Signal explicite `"regex": true/false`

- **Defaut : false** — pas de detection automatique des metacaracteres regex
- `"c++"` reste un litteral avec separateurs, jamais interprete comme regex
- L'utilisateur opt-in explicitement avec `"regex": true`

### 2. Candidats : toujours union

L'`Extractor` retourne des prefixes alternatifs. Chaque litteral est une alternative (le match peut commencer par l'un ou l'autre). Donc **union** des candidats de tous les litteraux.

Les faux positifs dans les candidats sont filtres par la verification. Pas de faux negatifs.

### 3. BM25 : litteraux comme termes de reference

En mode regex, on utilise les litteraux extraits comme termes pour calculer le `Bm25Weight` :

```rust
let terms: Vec<Term> = literals.iter()
    .map(|lit| Term::from_field_text(raw_field, lit))
    .collect();
Bm25Weight::for_terms(stats, &terms)
```

Coherent avec le mode fuzzy (qui utilise les tokens du query). L'IDF des litteraux reflete la rarete du pattern.

### 4. Seuil minimum : 3 chars

Si le litteral le plus long fait < 3 chars, les trigrams ne sont pas discriminants. On passe par le fallback FST walk pour le candidat collection, mais on garde le BM25 scoring (verification sur texte stocke + count tf).

| Litteral | Trigrams | Decision |
|----------|----------|----------|
| `"v"` (1 char) | aucun | Fallback FST |
| `"lo"` (2 chars) | `["lo"]` (pas un vrai trigram) | Fallback FST |
| `"log"` (3 chars) | `["log"]` (1 trigram) | Trigram (borderline mais OK) |
| `"log_"` (4 chars) | `["log", "og_"]` | Trigram (bon) |
| `"program"` (7 chars) | 5 trigrams | Trigram (tres bon) |

### 5. Fuzzy sur les litteraux regex : verification hybride

Quand `"regex": true` ET `distance > 0`, la verification est **hybride** :

1. **Regex exact** : `regex::Regex::find_iter(stored_text)` -> `tf_regex`
2. **Fuzzy sur litteraux** : `token_match_distance()` sur chaque litteral extrait -> `tf_fuzzy`
3. **tf final** = `max(tf_regex, tf_fuzzy)`

Ca donne le meilleur des deux mondes :
- Le regex matche precis quand le pattern est correct
- Le fuzzy rattrape les cas ou la partie litterale du regex a une faute de frappe

Exemple :

```
Query: {"value": "programing[a-z]+", "regex": true, "distance": 1}
Litteraux extraits: ["programing"]

Document: "Rust is a systems programming language"

Verification 1 (regex): "programing[a-z]+" sur le texte
  -> "programming" ne contient pas "programing" -> tf_regex = 0

Verification 2 (fuzzy sur litteraux): "programing" vs tokens du doc
  -> "programming" vs "programing" -> Levenshtein distance = 1 -> MATCH
  -> tf_fuzzy = 1

tf = max(0, 1) = 1 -> document trouve avec BM25 score
```

### 6. Fallback FST walk avec BM25

Meme quand on tombe en fallback (litteraux trop courts), on garde le BM25 :

```
Fallback :
1. FST walk (RegexQuery actuelle) -> doc_ids candidats
2. Pour chaque candidat : verification sur texte stocke -> count tf
3. BM25 score(fieldnorm_id, tf)
```

Scoring uniforme quelle que soit la methode de candidat collection (trigrams ou FST). Le FST walk est plus lent, mais le scoring est le meme.

## Cote Rag3Weaver (TypeScript)

L'API haut niveau pourrait detecter automatiquement les RegExp JS :

```typescript
// Fuzzy contains (defaut)
weaver.search("programing")
// -> {"type":"contains", "value":"programing", "distance":1}

// Regex contains (detection du RegExp JS)
weaver.search(/program[a-z]+ing/)
// -> {"type":"contains", "value":"program[a-z]+ing", "regex":true}

// Regex + fuzzy (option explicite)
weaver.search(/programing[a-z]+/, { fuzzyDistance: 1 })
// -> {"type":"contains", "value":"programing[a-z]+", "regex":true, "distance":1}
```

## Implementation dans le code Rust

### Structure unifiee

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
        // Pour la verification hybride (regex + fuzzy quand distance > 0)
        literals: Vec<String>,       // litteraux extraits du pattern
        fuzzy_distance: u8,          // 0 = regex pur, > 0 = hybride
    },
}

struct NgramContainsQuery {
    raw_field: Field,
    ngram_field: Field,
    stored_field: Option<Field>,
    verification: VerificationMode,
    trigram_sources: Vec<String>,  // tokens (fuzzy) ou litteraux (regex)
    highlight_sink: Option<Arc<HighlightSink>>,
}
```

### Fichiers modifies

| Fichier | Changement |
|---------|------------|
| `src/query/phrase_query/ngram_contains_query.rs` | Ajouter `VerificationMode`, adapter `verify()` et `count_*()` |
| `lucivy_fts/rust/src/query.rs` | Adapter `build_contains_query()` pour gerer `"regex": true` |
| `src/query/phrase_query/scoring_utils.rs` | Eventuellement factoriser des utilitaires partages |
| `Cargo.toml` | Ajouter `regex-syntax` en dependance directe si pas deja le cas |

### Pas de nouveau fichier

On ne cree PAS de `ngram_regex_query.rs` separe. Tout reste dans `ngram_contains_query.rs` avec le `VerificationMode`. C'est plus propre et evite la duplication.
