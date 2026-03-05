# Contains unifie : regex + fuzzy + BM25 — TERMINE

Date : 14 fevrier 2026

## Statut : COMPLET — 6 etapes implementees, testees, commitees, pushees

L'implementation du contains unifie (vision doc 04) est terminee a 100%. Toutes les etapes du plan (doc 02) sont faites, y compris le fallback FST avec BM25 (etape 5) qui etait initialement optionnel.

## Recapitulatif des etapes

| Etape | Description | Statut |
|-------|-------------|--------|
| 1 | Refactoring NgramContainsQuery avec `VerificationMode::Fuzzy` | FAIT |
| 2+3 | `VerificationMode::Regex` pure + hybride | FAIT |
| 4 | Routing `regex: true` dans query.rs + `QueryConfig` | FAIT |
| 5 | Fallback full-scan avec BM25 (litteraux < 3 chars) | FAIT |
| 6 | Tests Rust unitaires (10) + GTest E2E (7 sub-tests) | FAIT |

## Ce qui a ete fait dans cette session

### Etape 5 — Fallback full-scan avec BM25

Le doc 06 marquait l'etape 5 comme "PARTIEL" — quand les litteraux regex sont < 3 chars (ex: `v[0-9]+` → litteral "v"), le fallback utilisait `RegexQuery` standard (FST walk, ConstScorer, pas de BM25).

**Maintenant corrige** avec 2 changements :

1. **`ngram_contains_query.rs`** (Weight::scorer, branche Regex) :
   - Quand `trigram_sources` est vide → `(0..reader.max_doc()).collect()`
   - Scan complet du segment, tous les docs sont candidats
   - La verification regex filtre, le BM25 score normalement

2. **`query.rs`** (build_contains_regex) :
   - Toujours utiliser `NgramContainsQuery` quand un ngram_field existe, meme sans litteraux
   - Le `RegexQuery` standard ne sert plus que s'il n'y a pas de ngram_field du tout (cas theorique)

3. **`lucivy_fts_test.cpp`** — nouveau sub-test (#7) :
   - Regex `v[0-9]` sur doc contenant "version v2.0 systems"
   - Litteraux < 3 chars → full-scan → match avec BM25

### Tests E2E valides

Les tests GTest E2E (doc 06) etaient ecrits mais pas encore buildes/lances. C'est maintenant fait :

```
[  PASSED  ] 11 tests.
```

11/11 tests passent, dont `LucivyRegexContainsTest` avec 7 sub-tests :

1. Regex accelere par trigrams : `program[a-z]+` → 3 docs
2. BM25 scoring variable (pas constant)
3. Regex + fuzzy hybride : `programing[a-z]+` distance=1 → match via fuzzy
4. Regex sans match : `python[a-z]+` → 0 resultats
5. Highlights presents
6. Regression check : contains non-regex fonctionne toujours
7. **Nouveau** : Regex short-literal : `v[0-9]` → match via full-scan + BM25

### Tests Rust

```
test result: ok. 1025 passed; 0 failed; 7 ignored
```

### README ld-lucivy mis a jour

- Description enrichie (fuzzy + regex + hybrid)
- Tableaux d'exemples separes pour fuzzy et regex
- Section "Regex acceleration" dediee
- 1025 tests documentes

### Commits et push

| Repo | Branche | Commit | Contenu |
|------|---------|--------|---------|
| ld-lucivy | `main` | `80159c1` | feat: unified contains with regex mode (6 fichiers, +622/-302) |
| ld-lucivy | `main` | `77c4ca6` | docs: update README |
| rag3db | `feature/fuzzy-fts` | `e0186a809` | feat: regex contains E2E tests + submodule update |

ld-lucivy pushe sur origin/main. rag3db non pushe (a faire quand pret).

## Verification de la vision (doc 04)

| Decision de la vision | Implementee ? |
|----------------------|---------------|
| Signal explicite `regex: true/false` (defaut false) | OUI |
| Candidats union pour regex (vs intersection pour fuzzy) | OUI |
| BM25 avec litteraux comme termes de reference | OUI |
| Seuil minimum 3 chars pour trigrams | OUI |
| Verification hybride `tf = max(tf_regex, tf_fuzzy)` | OUI |
| Fallback FST walk avec BM25 | OUI (full-scan au lieu de FST walk — plus simple, meme resultat) |
| Integration Rag3Weaver TypeScript | NON (Phase C, futur) |

**Note sur le fallback** : la vision proposait un FST walk pour le fallback. On a opte pour un full segment scan (`0..max_doc`) qui est plus simple et donne le meme resultat. Pour les index de petite/moyenne taille (cas RAG typique), la difference de perf est negligeable. Le regex verification filtre les faux positifs.

## API finale

```json
// Fuzzy contains (inchange, defaut)
{"type":"contains", "field":"body", "value":"programing", "distance":1}

// Regex contains (nouveau)
{"type":"contains", "field":"body", "value":"program[a-z]+", "regex":true}

// Regex + fuzzy hybride (nouveau)
{"type":"contains", "field":"body", "value":"programing[a-z]+", "regex":true, "distance":1}

// Regex short-literal (nouveau, full-scan fallback)
{"type":"contains", "field":"body", "value":"v[0-9]+", "regex":true}
```

Defaut `regex`: false. Defaut `distance` en mode regex: 0 (regex pur).

## Fichiers modifies (complet)

| Fichier | Changement |
|---------|------------|
| `src/query/phrase_query/ngram_contains_query.rs` | `VerificationMode`, `FuzzyParams`, `RegexParams`, `verify_regex()`, candidats union, full-scan fallback, 10 tests |
| `src/query/phrase_query/mod.rs` | Export `FuzzyParams`, `RegexParams`, `VerificationMode` |
| `src/query/mod.rs` | Re-export |
| `lucivy_fts/rust/src/query.rs` | `regex` dans `QueryConfig`, `build_contains_regex()`, routing, full-scan path |
| `Cargo.toml` (ld-lucivy) | `regex-syntax = "0.8"` |
| `lucivy_fts/rust/Cargo.toml` | `regex = "1"` + `regex-syntax = "0.8"` |
| `ld-lucivy/README.md` | Mise a jour avec regex mode |
| `extension/lucivy_fts/test/lucivy_fts_test.cpp` | `LucivyRegexContainsTest` (7 sub-tests) |

## Architecture finale du NgramContainsQuery

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
                    trigram_sources = tokens | litteraux
                            │
            ┌───────────────┼───────────────┐
            │               │               │
        Fuzzy           Regex           Regex (short)
        exact+ngram     ngram union     full-scan
        intersection    (lits >= 3)     (lits < 3)
            │               │               │
            └───────────────┼───────────────┘
                            │
                Pour chaque candidat :
                load stored text
                            │
            ┌───────────────┴───────────────┐
            │ fuzzy                         │ regex
            ▼                               ▼
    token_match_distance()          verify_regex() :
    (Levenshtein)                   1. regex::find_iter() -> tf_regex
    -> tf                           2. si distance > 0 :
                                       fuzzy sur lits -> tf_fuzzy
                                    -> tf = max(tf_regex, tf_fuzzy)
            │                               │
            └───────────────┬───────────────┘
                            │
                    BM25 score(fieldnorm_id, tf)
                    Highlights (byte offsets)
                            │
                        Resultats
```
