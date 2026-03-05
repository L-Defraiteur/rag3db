# Progression — Contains unifie : regex + fuzzy + BM25

Date : 14 fevrier 2026

## Statut : Implementation terminee, tests E2E a valider

## Ce qui a ete fait

### Etape 1 : Refactoring NgramContainsQuery (FAIT)

**Fichier** : `src/query/phrase_query/ngram_contains_query.rs`

- Ajout de `FuzzyParams` struct (tokens, separators, prefix, suffix, fuzzy_distance, distance_budget, strict_separators)
- Ajout de `VerificationMode` enum avec variant `Fuzzy(FuzzyParams)`
- `NgramContainsQuery` restructure : `trigram_sources: Vec<String>` + `verification: VerificationMode`
- Constructeur `new()` prend maintenant `(raw_field, ngram_field, stored_field, trigram_sources, verification)`
- Les methodes `count_single_token`, `count_multi_token`, `check_at_position` sont devenues des fonctions libres (`count_single_token_fuzzy`, `count_multi_token_fuzzy`, `check_at_position_fuzzy`) pour eviter les conflits de borrow
- `verify()` dispatch sur le mode via `match &self.verification`
- BM25 utilise `trigram_sources` comme termes de reference (fonctionne pour fuzzy ET regex)
- **Validation** : 1015 tests passes, aucun changement de comportement

**Fichiers modifies** :
- `src/query/phrase_query/ngram_contains_query.rs` — restructuration complete
- `src/query/phrase_query/mod.rs` — export `FuzzyParams`, `VerificationMode`
- `src/query/mod.rs` — re-export `FuzzyParams`, `VerificationMode`
- `lucivy_fts/rust/src/query.rs` — import + construction `VerificationMode::Fuzzy(FuzzyParams {...})`

### Etape 2+3 : VerificationMode::Regex pure + hybride (FAIT)

**Fichier** : `src/query/phrase_query/ngram_contains_query.rs`

- Ajout de `RegexParams` struct : `compiled: Regex`, `literals: Vec<String>`, `fuzzy_distance: u8`
- Ajout variant `VerificationMode::Regex(RegexParams)`
- Fonction `verify_regex()` implementee :
  - **Regex pur** (fuzzy_distance == 0) : `compiled.find_iter(stored_text)` → count tf
  - **Hybride** (fuzzy_distance > 0) : regex exact OR fuzzy sur litteraux extraits, `tf = max(tf_regex, tf_fuzzy)`
  - Highlights : offsets des matchs regex (quand regex matche)
- Candidat collection dans `Weight::scorer()` adapte :
  - **Fuzzy** : exact lookup + ngram + **intersection** (tous les tokens doivent matcher)
  - **Regex** : ngram seulement + **union** (chaque litteral est une alternative) + dedup
- Ajout `use regex::Regex` et `use std::cmp::max` dans les imports
- Export `RegexParams` depuis `phrase_query/mod.rs` et `query/mod.rs`
- **Validation** : 1015 tests passes

### Etape 4 : Routing dans query.rs (FAIT)

**Fichier** : `lucivy_fts/rust/src/query.rs`

- Ajout `regex: Option<bool>` dans `QueryConfig` (serde deserialization)
- `build_contains_query()` refactorise : dispatch vers `build_contains_fuzzy()` (defaut) ou `build_contains_regex()` (quand `regex: true`)
- `build_contains_regex()` implementee :
  1. Compile le regex avec `Regex::new(&format!("(?i){pattern}"))` (case-insensitive)
  2. Parse le HIR avec `regex_syntax::parse(pattern)`
  3. Extrait les litteraux avec `Extractor::new().extract(&hir)`
  4. Filtre : garde les litteraux >= 3 chars (utiles pour trigrams)
  5. Si litteraux suffisants + ngram field dispo → `NgramContainsQuery` avec `VerificationMode::Regex`
  6. Sinon → fallback `RegexQuery` standard (FST walk, pas de BM25)
- Dependances ajoutees :
  - `Cargo.toml` (ld-lucivy) : `regex-syntax = "0.8"`
  - `lucivy_fts/rust/Cargo.toml` : `regex = "1"` + `regex-syntax = "0.8"`
- Imports ajoutes dans query.rs : `use regex::Regex`, `use regex_syntax::hir::literal::Extractor`, `RegexParams`
- **Validation** : 1025 tests passes (1015 existants + 10 nouveaux)

### Etape 5 : Fallback FST (PARTIEL)

Le fallback quand les litteraux sont < 3 chars utilise actuellement `RegexQuery` standard (FST walk, ConstScorer — pas de BM25). C'est fonctionnel mais sans BM25 scoring. A ameliorer plus tard si besoin. Le cas principal (litteraux >= 3 chars) est couvert.

### Etape 6 : Tests (PARTIELLEMENT FAIT)

#### Tests Rust unitaires : FAIT (10/10)

Module `ngram_contains_query::tests` ajoute dans `ngram_contains_query.rs` :

1. `test_regex_pure_match` — pattern `program[a-z]+` matche "programming" → tf=1
2. `test_regex_pure_no_match` — "the cat sat" → tf=0
3. `test_regex_pure_multiple_matches` — 3 occurrences → tf=3
4. `test_regex_case_insensitive` — "Rust" matche pattern `rust`
5. `test_regex_hybrid_typo_in_pattern` — "programing[a-z]+" distance=1 matche "programming" via fuzzy
6. `test_regex_hybrid_exact_wins` — regex correct + fuzzy → tf > 0
7. `test_regex_hybrid_no_match` — "python[a-z]+" sur texte Rust → tf=0
8. `test_regex_highlights` — offsets corrects [5, 16] pour "programming"
9. `test_regex_empty_text` — texte vide → tf=0
10. `test_regex_dot_star` — `.*` matche tout

**Total : 1025 tests Rust passes.**

#### Tests GTest E2E : ECRIT, PAS ENCORE BUILDE/RUN

Test `LucivyRegexContainsTest` ajoute dans `extension/lucivy_fts/test/lucivy_fts_test.cpp` :

1. Regex accelere par trigrams : `program[a-z]+` → 3 docs (programming x2, programmer x1)
2. BM25 scoring variable (pas constant)
3. Regex + fuzzy hybride : `programing[a-z]+` distance=1 → matche via fuzzy
4. Regex sans match : `python[a-z]+` → 0 resultats
5. Highlights presents
6. Regression check : contains non-regex fonctionne toujours

## Ce qu'il reste a faire

### 1. Builder et lancer les tests GTest E2E

Le build Rust release est fait (`cargo build --release -p ld-lucivy -p lucivy-fts` OK).
L'extension est re-linkee (`cmake --build . --target rag3db_lucivy_fts_extension` OK).

**Il reste** :

```bash
cd /home/luciedefraiteur/LR_CodeRag/community-docs/packages/rag3db/build/release

# 1. Builder le test executable
cmake --build . --target lucivy_fts_test -j$(nproc)

# 2. Lancer les tests (attention au LD_LIBRARY_PATH miniconda)
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu ./extension/lucivy_fts/test/lucivy_fts_test
```

Si le test `LucivyRegexContainsTest` echoue, les causes probables :
- Le test 1 (3 resultats) pourrait etre 2 si "programmer" n'est pas dans les candidats trigram. Verifier que le litteral "program" (7 chars, 5 trigrams) genere bien des candidats couvrant les 3 docs.
- Le test 3 (hybride) depend du fait que les trigrams de "programing" couvrent assez de candidats avec fuzzy_distance=1.

### 2. Commit et push

Une fois les tests E2E verts :

```bash
# Dans ld-lucivy (branch main)
cd /home/luciedefraiteur/LR_CodeRag/community-docs/packages/rag3db/extension/lucivy/ld-lucivy
git add -A && git commit -m "feat: unified contains with regex mode (trigram-accelerated + hybrid fuzzy)"

# Dans rag3db (branch feature/fuzzy-fts)
cd /home/luciedefraiteur/LR_CodeRag/community-docs/packages/rag3db
git add extension/lucivy_fts/test/lucivy_fts_test.cpp
git commit -m "test: add LucivyRegexContainsTest E2E"
```

### 3. (Optionnel) Etape 5 complete — Fallback FST avec BM25

Quand les litteraux sont < 3 chars (ex: `v[0-9]+` → litteral "v"), le fallback actuel est `RegexQuery` standard (pas de BM25). Pour avoir BM25 meme en fallback, il faudrait :
- Creer un `NgramContainsQuery` avec `trigram_sources` vide
- Dans `Weight::scorer()`, detecter que trigram_sources est vide et faire un FST walk pour obtenir les candidats
- Puis passer ces candidats au scorer avec verification regex + BM25

C'est un cas edge, pas prioritaire.

## Fichiers modifies (recapitulatif)

| Fichier | Changement |
|---------|------------|
| `src/query/phrase_query/ngram_contains_query.rs` | `VerificationMode`, `FuzzyParams`, `RegexParams`, `verify_regex()`, candidats union, 10 tests |
| `src/query/phrase_query/mod.rs` | Export `FuzzyParams`, `RegexParams`, `VerificationMode` |
| `src/query/mod.rs` | Re-export |
| `lucivy_fts/rust/src/query.rs` | `regex` dans `QueryConfig`, `build_contains_regex()`, routing |
| `Cargo.toml` (ld-lucivy) | `regex-syntax = "0.8"` |
| `lucivy_fts/rust/Cargo.toml` | `regex = "1"` + `regex-syntax = "0.8"` |
| `extension/lucivy_fts/test/lucivy_fts_test.cpp` | Nouveau test `LucivyRegexContainsTest` |

## API finale

```json
// Fuzzy contains (inchange, defaut)
{"type":"contains", "field":"body", "value":"programing", "distance":1}

// Regex contains (nouveau)
{"type":"contains", "field":"body", "value":"program[a-z]+", "regex":true}

// Regex + fuzzy hybride (nouveau)
{"type":"contains", "field":"body", "value":"programing[a-z]+", "regex":true, "distance":1}
```

Defaut `regex`: false. Defaut `distance` en mode regex: 0 (regex pur).
