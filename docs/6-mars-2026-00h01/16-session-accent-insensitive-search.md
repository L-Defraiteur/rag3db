# Session 16 — Recherche accent-insensitive (contains, regex, fuzzy)

Date : 6 mars 2026

## Probleme

La recherche `contains` utilise des trigrams (champ `._ngram`) pour accelerer la recherche de sous-chaines. Les trigrams de "tronçonneuse" (`onç`, `nço`, `çon`) ne matchaient pas ceux de "tronconneuse" (`onc`, `nco`, `con`). Resultat : toute recherche croisant accents et absence d'accents echouait.

## Diagnostic

Trois bugs independants identifies, tous dans le chemin de verification des candidats ngram :

### Bug 1 : `tokenize_raw()` — split sur non-ASCII

**Fichier** : `ld-lucivy/src/query/phrase_query/scoring_utils.rs:87`

`tokenize_raw()` utilisait `bytes[i].is_ascii_alphanumeric()` pour delimiter les tokens. Les caracteres multi-octets UTF-8 comme `ç` (0xC3 0xA7) etaient traites comme des separateurs, splitant "tronçonneuse" en ["tron", "onneuse"].

**Fix** : Iteration via `char_indices()` + `char::is_alphanumeric()` pour gerer correctement Unicode.

### Bug 2 : `token_match_distance()` — comparaison byte-level sans folding

**Fichier** : `ld-lucivy/src/query/phrase_query/scoring_utils.rs:156`

La comparaison fuzzy (`edit_distance`, `contains_fuzzy_substring`) operait sur les bytes bruts. "tronçonneuse" (14 bytes) vs "tronconneuse" (12 bytes) = distance ≥ 2 en bytes, depassant `fuzzy_distance=1`, alors qu'en chars c'est une simple substitution ç→c.

**Fix** : Application de `to_ascii()` (ASCII folding) sur les deux tokens avant comparaison. Apres folding, les deux deviennent "tronconneuse" = distance 0.

### Bug 3 : `verify_regex()` — regex sur texte non-folde

**Fichier** : `ld-lucivy/src/query/phrase_query/ngram_contains_query.rs:557`

Le mode contains-regex executait le regex directement sur le texte stocke. Le regex `facad.` ne matchait pas `façade` car `c` ≠ `ç` en regex.

**Fix** :
- `verify_regex()` : fold le texte stocke via `fold_with_byte_map()` avant le regex match, puis mappe les offsets retour vers le texte original pour les highlights
- `build_contains_regex()` (query.rs) : fold le pattern regex avant compilation
- `fold_with_byte_map()` : nouvelle fonction utilitaire qui fold le texte et produit un vecteur `map[folded_byte] → original_byte` pour la correspondance des offsets highlights

## Changements anterieurs (session precedente, consolides ici)

### Indexation ngram : AsciiFoldingFilter

**Fichier** : `lucivy_fts/rust/src/handle.rs:308-321`

Ajout de `AsciiFoldingFilter` dans la chaine du tokenizer ngram. "tronçonneuse" est desormais indexe avec les trigrams de "tronconneuse" (`tro`, `ron`, `onc`, `nco`, `con`...).

### Query trigrams : folding dans `generate_trigrams()`

**Fichier** : `ld-lucivy/src/query/phrase_query/scoring_utils.rs:190`

`generate_trigrams()` applique `to_ascii()` avant de generer les trigrams cote query. La query "tronçonneuse" genere les memes trigrams que "tronconneuse".

### Exports publics

- `ascii_folding_filter.rs:1550` : `to_ascii()` rendue `pub`
- `tokenizer/mod.rs:146` : `to_ascii` ajoute aux exports publics

## Fichiers modifies

| Fichier | Changement |
|---|---|
| `ld-lucivy/src/query/phrase_query/scoring_utils.rs` | `tokenize_raw()` Unicode, `token_match_distance()` folding, `generate_trigrams()` folding, nouveau `fold_with_byte_map()` |
| `ld-lucivy/src/query/phrase_query/ngram_contains_query.rs` | `verify_regex()` fold texte+offsets, import `fold_with_byte_map`, `make_regex_params` test helper fold pattern |
| `ld-lucivy/src/tokenizer/ascii_folding_filter.rs` | `to_ascii()` rendue publique |
| `ld-lucivy/src/tokenizer/mod.rs` | Export `to_ascii` |
| `lucivy_fts/rust/src/handle.rs` | `AsciiFoldingFilter` dans tokenizer ngram |
| `lucivy_fts/rust/src/query.rs` | `build_contains_regex()` fold pattern avant compilation |

## Tests

### Tests Rust
- 1066 tests ld-lucivy : OK
- 48 tests lucivy-fts : OK

### Tests Python (validation E2E via bindings PyO3)

3 documents longs avec accents francais varies (tronçonneuse, café, gâteau, crème fraîche, façade, méditerranéen, Élysées, rétropropagation...).

**Contains fuzzy (mode par defaut)** — 11/11 :
- Sans accent → donnees avec accent : `tronconneuse` → "tronçonneuse"
- Avec accent → donnees avec accent : `tronçonneuse` → "tronçonneuse"
- Substring : `tronco` → "tronçonneuse", `cafe` → "café"
- Multi-mot : `creme fraiche` → "crème" + "fraîche"
- Accent + typo : `tronçonneuz` d=1 → "tronçonneuse"
- Sans accent + typo : `tronconneus` d=1 → "tronçonneuse"
- Mots techniques : `retropropagation` → "rétropropagation"
- Noms propres : `Francois` → "François" + "français", `Elysees` → "Élysées"

**Contains regex** — 8/8 :
- `façad.` et `facad.` → "façade" (les deux matchent)
- `tronçon.+` et `troncon.+` → match
- `météo.*` et `meteo.*` → match
- `caf[eé]` → "café"
- `(développ|implement).+` → match

**Fuzzy standard** — 5/5 :
- `developpeur` → "développeur"
- `façade` / `facade` → "façade"
- `tronconneuse` → "tronçonneuse"
- `electrique` d=1 → "électrique"

**Highlights** : tous les byte offsets corrects, meme avec caracteres multi-octets (ç=2 bytes → c=1 byte apres folding). Le `fold_with_byte_map` assure la correspondance.

## Caveat

Les index existants doivent etre recrees (DROP + CREATE) pour beneficier du folding cote indexation. Les nouvelles donnees sont correctes d'office.

## Wheel Python

Build wheel Python 3.12 produit :
```
target/wheels/lucivy-0.1.0-cp312-cp312-manylinux_2_34_x86_64.whl
```
