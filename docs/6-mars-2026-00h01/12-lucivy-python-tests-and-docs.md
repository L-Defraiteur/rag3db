# Doc 12 — Lucivy Python : Fix SegmentId, Tests, Documentation

## Résumé

Fix du bug de compilation Python (SegmentId), création d'une suite de 71 tests pytest, et réécriture complète de la documentation (README ld-lucivy + README lucivy Python) pour guider les utilisateurs vers les bonnes pratiques.

---

## Bug fix : SegmentId mismatch

**Fichier :** `lucivy/src/lib.rs:544`

**Cause :** `HighlightSink::get()` a été migré de `(u32, DocId)` vers `(SegmentId, DocId)` comme clé (SegmentId = UUID, pas un ordinal). Le code Python passait `doc_addr.segment_ord` (u32) directement.

**Fix :**
```rust
// Avant (cassé)
let by_field = sink.get(doc_addr.segment_ord, doc_addr.doc_id)?;

// Après
let seg_id = searcher.segment_reader(doc_addr.segment_ord).segment_id();
let by_field = sink.get(seg_id, doc_addr.doc_id)?;
```

Pattern identique à celui utilisé dans les tests natifs ld-lucivy (`ngram_contains_query.rs:1205`).

---

## Tests Python : 71 tests pytest

**Fichier :** `lucivy/tests/test_lucivy.py`

| Classe | Tests | Couverture |
|--------|-------|-----------|
| `TestCRUD` | 7 | create, count, schema, repr, add single/many, context manager |
| `TestContains` | 7 | single word, dict query, no match, case insensitive, cross-token, multi-token phrase, substring |
| `TestContainsSplit` | 5 | string split, dict, multi-word, single-word fallback, contains vs contains_split sémantique |
| `TestFuzzy` | 7 | fuzzy per-token (d=1, d=2, exact), contains fuzzy single/multi-word, d=0 exact only, contains_split fuzzy |
| `TestRegex` | 8 | contains+regex cross-token, alternation, wildcard, no match, regex per-token, per-token alternation, per-token no cross-token (preuve), no match |
| `TestHighlights` | 9 | returned, byte offsets valid, off by default, multi-field, specific field, no internal fields, with contains+regex, with fuzzy, with boolean |
| `TestDelete` | 3 | removes doc, multiple, then search |
| `TestUpdate` | 2 | modifies content, preserves others |
| `TestPersistence` | 5 | reopen, after delete, after update, uncommitted not persisted, rollback |
| `TestAllowedIds` | 3 | filter, empty, multiple |
| `TestComposite` | 4 | boolean must/should/must_not, mixed query types |
| `TestFilterFields` | 3 | eq, gt, between (i64 avec indexed+fast) |
| `TestLimit` | 2 | restricts, default 10 |
| `TestSearchResult` | 2 | repr sans/avec highlights |
| `TestEdgeCases` | 4 | empty index, empty string → ValueError, special chars, add before commit |

**Total : 71 tests, 1.3s.**

---

## Documentation : distinction cross-token vs per-token

Problème identifié : les utilisateurs risquent d'utiliser `type: regex` ou `type: fuzzy` (per-token, hérité de Tantivy) alors que `type: contains` (cross-token, custom lucivy) est supérieur dans presque tous les cas.

### Deux catégories de requêtes

| | Cross-token (`contains`) | Per-token (`regex`, `fuzzy`, `term`) |
|---|---|---|
| **Opère sur** | Texte stocké (champ complet) | Tokens individuels dans l'index inversé |
| **Multi-mots** | Oui : `"programming language"` matche | Non : chaque token séparé |
| **Sous-chaînes** | Oui : `"program"` dans `"programming"` | Non (sauf regex `program.*`) |
| **Séparateurs** | Oui : `"std::collections"`, `"c++"` | Non |
| **Fuzzy** | `distance: 1` sur le texte stocké | Levenshtein sur un seul token |
| **Regex** | `regex: true` sur le texte stocké | Pattern sur tokens individuels |
| **Scoring** | BM25 (trigram-accéléré) | BM25 ou ConstScorer |

### Recommandation documentée

> Utiliser `contains` pour tout. Utiliser `regex`/`fuzzy`/`term` seulement quand on veut spécifiquement le comportement per-token de l'index inversé.

---

## Fichiers modifiés

| Fichier | Changement |
|---------|-----------|
| `lucivy/src/lib.rs` | Fix `segment_ord` → `segment_reader().segment_id()` (ligne 544) |
| `README.md` (racine ld-lucivy) | Restructuré : guide query types Python (cross-token vs per-token), filtres, highlights, internals. License corrigée LRSL v1.2. |

## Fichiers créés

| Fichier | Contenu |
|---------|---------|
| `lucivy/tests/test_lucivy.py` | 71 tests pytest couvrant CRUD, contains, contains_split, fuzzy, regex, highlights, delete, update, persistence, filters, boolean, edge cases |
| `lucivy/README.md` | README Python avec quick start, guide "use contains for everything", anti-pattern documenté |

---

## Limitations documentées

1. **`type: regex` ne supporte pas les anchors `^`/`$`** — Tantivy regex sur tokens ne les gère pas
2. **`type: regex` ne matche pas cross-token** — `"programming language"` = 2 tokens séparés → utiliser `contains` à la place
3. **Filtres non-texte requièrent `indexed: true, fast: true`** — sans ça, erreur "Field is not indexed"
4. **Query string vide → ValueError** — `contains_split` sur "" produit 0 clauses boolean

---

## Vérification

```
cargo check -p lucivy          → OK
maturin develop --release      → OK
pytest tests/ -v               → 71 passed in 1.3s
```
