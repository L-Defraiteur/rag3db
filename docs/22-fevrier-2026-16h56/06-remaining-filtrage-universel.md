# 06 — Points restants : Filtrage universel pré-filter

## Priorité 1 : match_clauses pour vector search ✅ FAIT

**Problème :** les filtres cross-entity (`Author.name = "John"`) génèrent des `MATCH` clauses via `FilterParser::parse_condition()`. Ces MATCH étaient correctement câblés dans le path BM25 (pré-résolution Kuzu → allowed_ids), mais `search_vector()` recevait uniquement `filter_where` (le WHERE) sans les MATCH clauses nécessaires.

**Ce qui a été fait :**
- `search_vector()` : ajout param `extra_match: Option<&str>`
- Le MATCH clause construit est `MATCH (n:Entity) {extra_match}` quand extra_match est présent
- `catalog.rs` : extraction de `filter_match` depuis `parsed.match_clauses.join(" ")`, passé aux 2 appels search_vector (Hybrid + Semantic)
- Test existant `search_vector_empty` mis à jour avec le nouveau param
- **306 tests rag3weaver — tout vert**

## Priorité 2 : SearchOptions avec FilterCondition ✅ FAIT

**Problème :** `SearchOptions.filters` est un `HashMap<String, FilterValue>` — pas de support pour la composition Must/Should/MustNot. Les utilisateurs ne peuvent pas exprimer `status = "active" OR type = "article"` via SearchOptions.

**Ce qui a été fait :**
- `search.rs` : ajout `pub filter_condition: Option<FilterCondition>` au struct `SearchOptions`
- `search.rs` : import de `FilterCondition` depuis `crate::filter`
- `search.rs` : ajout `filter_condition: None` dans le `Default` impl
- `catalog.rs` : résolution `condition` modifiée — `filter_condition` est prioritaire sur `filters` HashMap :
  ```rust
  let condition = if options.filter_condition.is_some() {
      options.filter_condition.clone()
  } else if !options.filters.is_empty() {
      Some(options.filters.clone().into())
  } else {
      None
  };
  ```
- 1 test ajouté `search_filter_condition_takes_priority`
- **307 tests rag3weaver — tout vert**

## Priorité 3 : Boolean dans filter_fields ✅ FAIT (côté Rust)

**Problème :** `generate_full_schema()` skip les champs Boolean et Timestamp. Boolean est simple (0/1 → i64).

**Ce qui a été fait (Rust) :**
- `schema.rs` : `FieldType::Boolean` retiré du skip → désormais inclus dans filter_fields
- `schema.rs` : `make_full_config()` étendu avec champ `published: Boolean` pour tester
- Assertion ajoutée au test `full_schema_fts_includes_filter_fields` : vérifie `'published'` dans le DDL
- **307 tests rag3weaver — tout vert**

**C++ : ✅ FAIT** (résolu entre-temps)
- `create_lucivy_index.cpp` : `mapLogicalTypeToLucivy()` gère BOOL et TIMESTAMP → "i64"
- `create_lucivy_index.cpp` : `originalTypeID` stocké, conversion bool→0/1 dans bulk indexing
- `lucivy_index.cpp` : `keyDataTypes[f] == PhysicalTypeID::BOOL` → `getValue<bool>() ? 1 : 0` dans insert() et finalize()
- `IndexInfo.keyDataTypes` existait déjà dans rag3db (hérité de Kuzu) — pas besoin d'Option B/C du doc 07

**Timestamp : ✅ FAIT**
- TIMESTAMP a PhysicalTypeID::INT64 (microseconds epoch) → `getValue<int64_t>()` fonctionne directement
- Filtrage gt/lt avec epoch microseconds testé E2E

## Priorité 4 : Tests E2E C++ ✅ FAIT

**15 tests E2E couvrent tout :**
- `LucivyBoolTimestampFilterTest` : boolean eq 1/0, timestamp gt/lt (5 sous-tests)
- `LucivyStringFilterFieldTest` : string eq/ne/starts_with/contains (6 sous-tests)
- `LucivyBetweenAndInFilterTest` : between, in, not_in sur i64/f64
- `LucivyFiltersWithAllowedIdsTest` : allowed_ids + filters combinés
- `LucivyFilterFieldsTest` : i64/f64/string filter fields basiques

## Résumé

| Priorité | Status | Tests |
|---|---|---|
| 1. match_clauses vector | ✅ FAIT | 306 |
| 2. SearchOptions FilterCondition | ✅ FAIT | 307 |
| 3. Boolean filter_fields (Rust) | ✅ FAIT | 307 |
| 3. Boolean filter_fields (C++) | ✅ FAIT | 15 E2E |
| 3. Timestamp filter_fields | ✅ FAIT | 15 E2E |
| 4. Tests E2E C++ | ✅ FAIT | 15/15 PASSED |

## Bugs corrigés (doc 10)

- `ne` / `not_in` / `must_not` : BooleanQuery MustNot-only → ajout AllQuery (doc 10)
- Build cmake/cargo : `--whole-archive` + suppression bridge dupliqué (doc 10)

## Commandes de vérification

```bash
# Rust (ld-lucivy + lucivy-fts : 1062 tests)
cd packages/rag3db/extension/lucivy/ld-lucivy && cargo test --lib

# Rust (rag3weaver : 307 tests)
cd packages/rag3db/extension/rag3weaver && cargo test --lib

# C++ E2E (15 tests)
cd packages/rag3db/build/release
cmake --build . --target lucivy_fts_test -j$(nproc)
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu ./extension/lucivy_fts/test/lucivy_fts_test
```
