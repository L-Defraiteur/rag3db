# Progression — Ngram sans stemmer + BM25 contains

Date : 14 fevrier 2026

## Objectif

1. Que le fast path ngram (trigram lookup) fonctionne meme sans stemmer
2. Que le scoring du contains soit BM25 au lieu d'un boost constant (1.0)

## Statut : TERMINE

Les deux changements sont implementes, testes (10/10 E2E + 1015 Rust), commites et pushes.

## Changement 1 : Ngram sans stemmer (handle.rs)

Fichier : `extension/tantivy/ld-tantivy/tantivy_fts/rust/src/handle.rs`

- `build_schema()` : supprime le `if has_stemmer` — les 3 champs (principal, `._raw`, `._ngram`) sont TOUJOURS crees pour chaque colonne texte
- Sans stemmer, le champ principal utilise le tokenizer "default" (lowercase) au lieu de "stemmed"
- `configure_tokenizers()` : le tokenizer ngram est enregistre inconditionnellement (avant il etait dans le `if let Some(stemmer)`)
- `open()` : reconstruit toujours les `raw_field_pairs` et `ngram_field_pairs` (plus de condition sur stemmer)
- Import `TEXT` supprime (plus utilise)

## Changement 2 : BM25 scoring (ngram_contains_query.rs)

Fichier : `extension/tantivy/ld-tantivy/src/query/phrase_query/ngram_contains_query.rs`

- `NgramContainsQuery::weight()` : calcule `Bm25Weight` via `EnableScoring` (meme pattern que `TermQuery`)
- `NgramContainsWeight` : nouveau champ `bm25_weight: Bm25Weight`
- `NgramContainsWeight::scorer()` : lit le `FieldNormReader` du segment, passe `bm25_weight.boost_by(boost)` au scorer
- `NgramContainsScorer` : remplace `boost: Score` par `bm25_weight` + `fieldnorm_reader` + `last_tf`
- `verify()` → `&mut self` : compte TOUTES les occurrences (tf) au lieu de s'arreter au premier match
- `score()` : `bm25_weight.score(fieldnorm_id, last_tf)` — vrai BM25 (k1=1.2, b=0.75)

## Bug BM25 "scores tous a 1.0" — RESOLU

### Symptome

Le 10eme test GTest montrait score=1.0 pour tous les docs au lieu de scores differencies.

### Cause racine

**Probleme de build, pas de code.** Le `CMakeLists.txt` de tantivy_fts utilise `add_custom_command(OUTPUT libtantivy_fts.a ...)` sans `DEPENDS` sur les sources Rust. Consequence :

1. `cmake --build` ne relance pas `cargo` quand les `.rs` changent
2. L'extension `.rag3db_extension` n'est pas re-linkee avec la nouvelle `.a`
3. Le test charge l'ancienne extension via `LOAD EXTENSION` → ancien code sans BM25

### Fix

Rebuild manuel en 2 etapes :

```bash
# 1. Recompiler le Rust
cd extension/tantivy/ld-tantivy
cargo build --release -p ld-tantivy -p tantivy-fts

# 2. Re-linker l'extension
cd build/release
cmake --build . --target rag3db_tantivy_fts_extension -j$(nproc)
```

### Verification par traces eprintln

Apres rebuild correct, les traces ont confirme que le BM25 fonctionnait :
- Doc 0 (tf=3, fieldnorm=8, body court) : score = **0.767**
- Doc 2 (tf=1, fieldnorm=13, body long) : score = **0.412**

Le piege a ete documente dans `extension/tantivy_fts/BUILD.md` et `BUILD.md` a la racine.

## Tests passes (tout vert)

- **Tests unitaires Rust** : 1015 passed
- **10 tests GTest E2E** : tous passent (y compris le nouveau `TantivyNgramContainsNoStemmerTest`)
- **Build Node.js natif** : OK
- **Build WASM** : OK
- **Playwright browser (2 tests IDBFS)** : OK

## Commits pushes

| Repo | Branch | Commit | Contenu |
|------|--------|--------|---------|
| ld-tantivy | `main` | `4c4e7ad` | handle.rs + ngram_contains_query.rs + README rebuild docs |
| ld-tantivy | `main` | `76ed60f` | README restructure (NgramContainsQuery + BM25 en avant) |
| rag3db | `feature/fuzzy-fts` | `7ed62275e` | Extension C++ complete + 10 tests + BUILD.md + submodule update |

## Documentation ajoutee

| Fichier | Contenu |
|---------|---------|
| `rag3db/BUILD.md` | Guide complet : tous les builds, sequence apres modif Rust, problemes courants |
| `extension/tantivy_fts/BUILD.md` | Architecture 3 couches, piege cmake/cargo, commande tout-en-un |
| `ld-tantivy/README.md` | Restructure : NgramContainsQuery en avant, triple-field layout, BM25, exemples de matchs |
