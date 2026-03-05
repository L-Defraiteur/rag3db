# 05 — Rapport de progression : Filtrage universel pré-filter

## TOUT EST FAIT (étapes 1-5 complètes)

### Étape 1 : ld-lucivy (handle.rs + query.rs) ✅

#### 1a. handle.rs — Ngram pour champs string

**Fichier :** `ld-lucivy/lucivy_fts/rust/src/handle.rs`

- Branche `"string"` dans `build_schema()` : ajout d'un champ `{name}._ngram` (trigrams, tokenizer `NGRAM_TOKENIZER`, `IndexRecordOption::Basic`) pour chaque champ string
- Enregistrement dans `ngram_field_pairs` → `auto_duplicate_field()` (bridge.rs) écrit automatiquement les trigrams à l'indexation
- `open()` : reconstruction de `ngram_field_pairs` pour les champs `"string"` en plus des `"text"` (chain des deux itérateurs)
- **Résultat :** tout champ `"string"` a désormais un `._ngram` counterpart, comme les champs `"text"`

#### 1b. query.rs — FilterClause étendu

**Fichier :** `ld-lucivy/lucivy_fts/rust/src/query.rs`

**Struct FilterClause modifié :**
```rust
pub struct FilterClause {
    pub field: Option<String>,     // None pour composites
    pub op: String,                // eq, ne, lt, lte, gt, gte, in, between, not_in, starts_with, contains, must, should, must_not
    pub value: Option<serde_json::Value>,
    pub distance: Option<u8>,      // fuzzy distance pour contains (default 1)
    pub clauses: Option<Vec<FilterClause>>,  // sous-clauses pour composites
}
```

**Nouveaux ops :** between, not_in, starts_with (TermQuery+RegexQuery combo), contains (NgramContainsQuery avec distance configurable), must/should/must_not (récursif).

**Tests :** 47 tests lucivy-fts (13 nouveaux), 1062 tests ld-lucivy — tout vert.

### Étape 2 : schema.rs — Indexer tous les champs ✅

**Fichier :** `rag3weaver/src/schema.rs`

- `generate_fts_index_ddl(table, fields, filter_fields)` — 3ème param ajouté
- Si `filter_fields` non vide → émet `filter_fields := ['col1', 'col2']` dans le DDL
- `generate_full_schema()` collecte les champs non-texte de chaque entité :
  - `String/Choice/Tags/Json` → filter field (Kuzu STRING → Lucivy "string")
  - `Int64/Integer` → filter field (Kuzu INT64 → Lucivy "i64")
  - `Double/Number` → filter field (Kuzu DOUBLE → Lucivy "f64")
  - `Boolean/Timestamp` → skip V1
- **Tests :** 24 tests schema (2 nouveaux) — tout vert

### Étape 3 : filter.rs — FilterCompiler ✅

**Fichier :** `rag3weaver/src/filter.rs`

**Nouveau struct `SplitResult` :**
```rust
pub struct SplitResult {
    pub lucivy: Option<FilterCondition>,  // Lucivy-native pre-filter
    pub kuzu: Option<FilterCondition>,     // Kuzu Cypher → allowed_ids
}
```

**`FilterCompiler` avec 2 méthodes principales :**
- `split(condition) → SplitResult` — sépare les ops Lucivy-compat des Kuzu-only
- `to_lucivy_json(condition) → Vec<serde_json::Value>` — compile en FilterClause JSON

**Règles de split :**
- Lucivy-compat : Eq, Neq, Lt, Lte, Gt, Gte, In, Between, NotIn, StartsWith, Contains
- Kuzu-only : IsNull, IsNotNull, IsEmpty, IsNotEmpty, HasAny, HasAll, HasNone, ValuesCount
- Must : split récursif (chaque enfant va dans sa catégorie)
- Should/MustNot : all-or-nothing (si un enfant est Kuzu-only, tout → Kuzu)
- Cross-entity (clé contient ".") : toujours Kuzu

**Tests :** 60 tests filter (15 nouveaux) — tout vert

### Étape 4 : search.rs + catalog.rs — Pipeline branché ✅

**search.rs :**
- `build_bm25_query()` accepte `lucivy_filters: Option<&[serde_json::Value]>` → injecte `"filters": [...]` dans le JSON
- `search_bm25()` : `extra_where/extra_params` remplacés par `lucivy_filters` + `allowed_ids`
  - Si `allowed_ids` → `CALL QUERY_LUCIVY_INDEX(..., allowed_ids := [1, 2, 3])`
  - **Zéro post-filter** : plus de `MATCH (n) WHERE id(n) = node_id AND {w}`
- `search_vector()` : inchangé (tout passe par Cypher WHERE, déjà pré-filter)

**catalog.rs :**
- `search()` : conversion `filters` HashMap → `FilterCondition`
- Pour vector : toutes les filters → `FilterParser::parse_condition()` → Cypher WHERE (inchangé)
- Pour BM25 :
  1. `FilterCompiler::split(&condition)` → `SplitResult`
  2. Partie Lucivy → `to_lucivy_json()` → injectée dans le JSON query
  3. Partie Kuzu → `parse_condition()` → Cypher MATCH → `RETURN id(n)` → `allowed_ids`
  4. `search_bm25(... lucivy_filters, allowed_ids)` — 100% pré-filter

### Étape 5 : lib.rs — Exports ✅

- Ajout `FilterCompiler`, `SplitResult` aux re-exports publics
- **Tests :** 306 tests rag3weaver — tout vert

## Résumé des tests

```
ld-lucivy:    1062 passed, 0 failed
rag3weaver:     306 passed, 0 failed
Total:         1368 passed, 0 failed
```

## Findings

### starts_with et la limitation empty-match lucivy-fst
`RegexQuery("^prefix.*")` échoue : `"Empty match operators are not allowed"`. Solution : `BooleanQuery(Should[TermQuery(exact), RegexQuery(prefix.+)])`.

### contains via NgramContainsQuery, pas RegexQuery
Notre fork a une cascade optimisée à 3 niveaux (NgramContainsQuery → AutomatonPhraseQuery → RegexQuery). On dispatch vers `build_contains_query()` avec distance configurable.

### Ngram SYSTÉMATIQUE
Tout champ texte ou string a un counterpart `._ngram`. Les champs numériques n'en ont pas (pas pertinent). Le code `build_contains_query()` a encore un fallback `if let Some(ngram_field)`, mais avec la garantie systématique des ngrams, le chemin `NgramContainsQuery` est **toujours** pris.

## Fichiers modifiés

| Fichier | Lignes | Quoi |
|---|---|---|
| `ld-lucivy/.../handle.rs` | +12 | `._ngram` pour champs `"string"`, rebuild pairs dans `open()` |
| `ld-lucivy/.../query.rs` | +130 | between, not_in, starts_with, contains, composition must/should/must_not |
| `rag3weaver/src/schema.rs` | +20 | filter_fields dans `generate_fts_index_ddl` + `generate_full_schema` |
| `rag3weaver/src/filter.rs` | +180 | FilterCompiler: split() + to_lucivy_json() + cypher_to_json() |
| `rag3weaver/src/search.rs` | +30/-20 | lucivy_filters + allowed_ids, suppression post-filter |
| `rag3weaver/src/catalog.rs` | +50/-15 | Orchestration split → pré-résolution → appels |
| `rag3weaver/src/lib.rs` | +1 | Exports FilterCompiler, SplitResult |

## Commandes de vérification

```bash
# ld-lucivy (1062 tests)
cd packages/rag3db/extension/lucivy/ld-lucivy && cargo test --lib

# rag3weaver (306 tests)
cd packages/rag3db/extension/rag3weaver && cargo test --lib
```
