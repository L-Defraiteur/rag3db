# 04 — Plan d'implémentation : Filtrage universel pré-filter

Basé sur le design 03 et l'exploration du code existant.

## État des lieux (ce qu'on a lu)

### Côté Tantivy (ld-tantivy/tantivy_fts)

**query.rs — FilterClause existant :**
```rust
pub struct FilterClause {
    pub field: String,
    pub op: String,    // "eq", "ne", "lt", "lte", "gt", "gte", "in"
    pub value: serde_json::Value,
}
```
- Compilé via `build_filter_clause()` en TermQuery, RangeQuery, BooleanQuery natifs
- Injecté dans `QueryConfig.filters` (JSON array)
- Wrappé dans un BooleanQuery(Must) avec la text query
- Supporte u64, i64, f64, string (via `json_to_term()`)

**query.rs — Contains query (notre implémentation optimisée) :**
```
build_contains_query() — cascade 3 niveaux :
  1. NgramContainsQuery (si champ ._ngram existe) → trigrams → vérification fuzzy/regex → BM25
  2. AutomatonPhraseQuery (fallback) → FST walk cascade exact → fuzzy → substring
  3. RegexQuery (dernier recours si zéro token après tokenisation)
```
- `NgramContainsQuery` utilise le champ `._ngram` pour la génération de candidats, le champ `._raw` pour le scoring, et le champ stored pour la vérification
- C'est notre implémentation optimisée — bien plus performante qu'un `RegexQuery(".*substring.*")`

**handle.rs — Triple layout pour champs texte :**
```
"text" → {name} (stemmed) + {name}._raw (lowercase) + {name}._ngram (trigrams)
"string" → {name} seul (exact match STRING, pas de _raw, pas de _ngram)
```
- `raw_field_pairs` et `ngram_field_pairs` dans TantivyHandle : mapping user field → counterpart
- `auto_duplicate_field()` (bridge.rs) écrit automatiquement dans `_raw` et `_ngram` à l'indexation
- Les champs `"string"` n'ont PAS de `_ngram` actuellement → pas de `NgramContainsQuery` possible

**bridge.rs — Fonctions cxx :**
```rust
fn search_filtered_with_highlights(handle, query_json, limit, allowed_ids) -> Vec<SearchResultWithHighlights>
fn search_with_highlights(handle, query_json, limit) -> Vec<SearchResultWithHighlights>
```
- `allowed_ids` passe par un `FilterCollector` avec HashSet<u64> — O(1) par doc
- Le JSON query peut contenir `"filters": [...]` en plus de la text query

### Côté C++ (extension tantivy_fts)

**create_tantivy_index.cpp :**
- `CREATE_TANTIVY_INDEX('table', ['field1', 'field2'], filter_fields := ['status', 'price'])`
- Champs texte → `{"type":"text","stored":true}` dans le schema JSON
- Filter fields → `{"type":"i64/u64/f64/string","stored":true,"indexed":true,"fast":true}`
- `mapLogicalTypeToTantivy()` : INT64→i64, UINT64→u64, DOUBLE→f64, STRING→string
- Scan toutes les rows existantes, `add_document_mixed()` pour indexer

**query_tantivy_index.cpp :**
- `CALL QUERY_TANTIVY_INDEX('table', '{json}', limit, allowed_ids := [...])`
- `allowed_ids` est un optional param — si présent → `search_filtered_with_highlights()`
- `flushIfDirty()` appelé avant chaque query (lazy commit)
- Retourne : `node_id UINT64, score DOUBLE, highlights STRING`

### Côté rag3weaver

**schema.rs — `generate_fts_index_ddl()` :**
```rust
fn generate_fts_index_ddl(table: &str, fields: &[&str]) -> String {
    // CALL CREATE_TANTIVY_INDEX('table', ['field1', 'field2'])
}
```
- Appelé uniquement pour les champs `title_for`/`content_for` d'une KB
- **Pas de filter_fields** passé actuellement

**search.rs — `search_bm25()` :**
```rust
fn search_bm25(conn, entity, query, fields, mode, fuzzy_distance, limit, extra_where, extra_params)
```
- Construit un JSON query via `build_bm25_query()` (type contains/boolean)
- Cypher : `CALL QUERY_TANTIVY_INDEX('{entity}', '{json}', {limit}) RETURN node_id, score`
- **Pas de `filters` dans le JSON**, pas de `allowed_ids` dans le CALL
- Le `extra_where` actuel fait du post-filter Cypher (ce qu'on veut éliminer)

**filter.rs — FilterCompiler (à créer) :**
- `FilterOp` a tous les ops (Eq, Neq, Lt, Gte, Between, In, NotIn, StartsWith, Contains, IsNull, HasAny, etc.)
- `FilterCondition` : Must/Should/MustNot récursif
- `FilterParser::parse()` compile en Cypher WHERE + params
- **Manque** : compilation vers Tantivy FilterClause JSON

## Plan par étapes

### Étape 1 : Ngram pour champs string + FilterClause étendu (~60 lignes)

#### 1a. handle.rs — Ajouter `_ngram` aux champs string (~15 lignes)

**Fichier :** `ld-tantivy/tantivy_fts/rust/src/handle.rs`

Modifier la branche `"string"` dans `build_schema()` (ligne 274) :
```rust
"string" => {
    use ld_tantivy::schema::STRING;
    let opts = if field_def.stored.unwrap_or(true) {
        STRING | STORED
    } else {
        STRING
    };
    let field = builder.add_text_field(&field_def.name, opts);
    field_map.push((field_def.name.clone(), field));

    // Ngram counterpart pour substring matching (NgramContainsQuery)
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

Modifier aussi `open()` (ligne 126-138) : reconstruire `ngram_field_pairs` pour les champs `"string"` en plus des `"text"`.

**Effet :** les champs string filter fields auront un `_ngram` counterpart. `auto_duplicate_field()` (bridge.rs) y écrira automatiquement les trigrams à l'indexation.

#### 1b. query.rs — Étendre `build_filter_clause()` (~45 lignes)

**Fichier :** `ld-tantivy/tantivy_fts/rust/src/query.rs`

Ajouter les nouveaux ops à `build_filter_clause()` :

| Op | Implémentation Tantivy |
|---|---|
| `between` | `BooleanQuery(Must[RangeQuery(>=lo), RangeQuery(<=hi)])` |
| `not_in` | `BooleanQuery(MustNot[BooleanQuery(Should[TermQuery, ...])])` |
| `starts_with` | `RegexQuery("^{prefix}.*")` sur champ string (FST optimisé pour les préfixes) |
| `contains` | `build_contains_query()` → dispatch `NgramContainsQuery` (si `_ngram` dispo) ou `AutomatonPhraseQuery` (fallback) |

**Important — `contains` :** on réutilise notre implémentation optimisée existante (`build_contains_query()`), pas un `RegexQuery(".*substring.*")`. Cela nécessite d'avoir le schema + index + raw/ngram pairs disponibles dans `build_filter_clause()`, donc sa signature change :

```rust
fn build_filter_clause(
    filter: &FilterClause,
    schema: &Schema,
    index: &Index,
    raw_pairs: &[(String, String)],
    ngram_pairs: &[(String, String)],
) -> Result<Box<dyn Query>, String>
```

Pour `contains`, on construit un `QueryConfig` temporaire et on appelle `build_contains_query()` :
```rust
"contains" => {
    let value = filter.value.as_str()
        .ok_or("'contains' requires a string value")?;
    let config = QueryConfig {
        query_type: "contains".into(),
        field: Some(filter.field.clone()),
        value: Some(value.to_string()),
        ..Default::default()
    };
    build_contains_query(&config, schema, index, raw_pairs, ngram_pairs, None)
}
```

Ajouter aussi la composition récursive à FilterClause :
```rust
pub struct FilterClause {
    pub field: Option<String>,     // None pour les composites
    pub op: String,                // "eq", ..., "between", "must", "should", "must_not"
    pub value: serde_json::Value,
    pub clauses: Option<Vec<FilterClause>>,  // pour must/should/must_not
}
```

**Tests :** 7 nouveaux tests (between, not_in, starts_with, contains via NgramContainsQuery, must/should composite, string ngram field existence).

### Étape 2 : Indexer tous les champs dans Tantivy (~30 lignes)

**Fichier :** `rag3weaver/src/schema.rs`

Modifier `generate_fts_index_ddl()` → nouvelle fonction `generate_fts_index_ddl_full()` :
- Champs `Text` → text fields (comme avant)
- Champs `String` → string filter field (aura `_ngram` grâce à l'étape 1a)
- Champs `Int64/Integer` → i64 filter field
- Champs `Double/Number` → f64 filter field
- Champs `Boolean` → i64 filter field (0/1)

Modifier `generate_full_schema()` pour passer tous les champs au lieu de seulement title/content.

Résultat DDL :
```sql
CALL CREATE_TANTIVY_INDEX('Document', ['title', 'body'],
    filter_fields := ['status', 'page_count', 'price'])
```

**Tests :** 2-3 tests schema.

### Étape 3 : FilterCompiler — split + to_tantivy_json (~100 lignes)

**Fichier :** `rag3weaver/src/filter.rs`

Nouveau struct `FilterCompiler` avec :

```rust
pub struct SplitResult {
    /// Ops compilables en Tantivy FilterClause
    pub tantivy: Option<FilterCondition>,
    /// Ops nécessitant Cypher (listes, null, cross-entity)
    pub kuzu: Option<FilterCondition>,
}

impl FilterCompiler {
    /// Sépare une FilterCondition en partie Tantivy-native et partie Kuzu-only.
    pub fn split(condition: &FilterCondition) -> SplitResult;

    /// Compile la partie Tantivy en JSON FilterClause array.
    pub fn to_tantivy_json(condition: &FilterCondition) -> Vec<serde_json::Value>;
}
```

Règles de split :
- **Tantivy-compatible** : Eq, Neq, Lt, Lte, Gt, Gte, Between, In, NotIn, StartsWith, Contains
- **Kuzu-only** : IsNull, IsNotNull, IsEmpty, IsNotEmpty, HasAny, HasAll, HasNone, ValuesCount
- **Must** : split récursif, chaque enfant va dans sa catégorie
- **Should** : si tous les enfants sont Tantivy-compat → Tantivy, sinon tout → Kuzu (pas de split d'un OR entre deux systèmes)
- **MustNot** : même logique que Should
- **Cross-entity** (clé contient ".") : toujours Kuzu

**Tests :** 8-10 tests (split simple, split mixte, Should full Tantivy, Should mixte → Kuzu, to_tantivy_json).

### Étape 4 : Brancher dans search pipeline (~80 lignes)

**Fichier :** `rag3weaver/src/search.rs`

Modifier `search_bm25()` :
```rust
pub async fn search_bm25(
    conn, entity, query, fields, mode, fuzzy_distance, limit,
    tantivy_filters: Option<&[serde_json::Value]>,  // FilterClause JSON
    allowed_ids: Option<&[u64]>,                      // pré-résolu via Kuzu
) -> Result<Vec<SearchResult>, CatalogError>
```

- Si `tantivy_filters` présent → injecter `"filters": [...]` dans le JSON query
- Si `allowed_ids` présent → ajouter `allowed_ids := [...]` dans le CALL Cypher
- Supprimer `extra_where` / `extra_params` (plus de post-filter)

Modifier `search_vector()` :
- Garder `extra_where` / `extra_params` pour les filtres Cypher (tous les ops, compilés par `to_cypher()`)
- Pas de changement fondamental — le vector search reste en Cypher WHERE

**Fichier :** `rag3weaver/src/catalog.rs`

Modifier `search()` :
```rust
// 1. Split les filtres
let split = FilterCompiler::split(&condition);

// 2. Compiler la partie Tantivy
let tantivy_filters = split.tantivy.map(|t| FilterCompiler::to_tantivy_json(&t));

// 3. Pré-résoudre la partie Kuzu → allowed_ids
let allowed_ids = if let Some(ref kuzu_cond) = split.kuzu {
    let parsed = parser.parse_condition(&kuzu_cond, &entity, "n")?;
    let cypher = format!("MATCH (n:{entity}) WHERE {} RETURN id(n)", parsed.combine_where());
    let result = conn.execute_with_params(&cypher, &parsed.params).await?;
    Some(result.rows.iter().map(|r| r[0].as_u64().unwrap_or(0) as u64).collect::<Vec<_>>())
} else {
    None
};

// 4. Appeler search_bm25 avec tantivy_filters + allowed_ids
// 5. Appeler search_vector avec all_filters en Cypher WHERE
```

**Tests :** 2-3 tests catalog (search avec filtre scalaire, search avec filtre mixte).

### Étape 5 : Nettoyage + exports (~20 lignes)

- Supprimer l'ancien code post-filter (`extra_where` dans `search_bm25`)
- Mettre à jour `lib.rs` : exporter `FilterCompiler`, `SplitResult`
- `cargo test --lib` — tout vert

## Résumé des fichiers

| Fichier | Lignes ~  | Quoi |
|---|---|---|
| `ld-tantivy/.../handle.rs` | +15 | `_ngram` counterpart pour champs `"string"`, rebuild pairs dans `open()` |
| `ld-tantivy/.../query.rs` | +45 | between, not_in, starts_with, contains (via `build_contains_query`), composition |
| `rag3weaver/src/schema.rs` | +30 | generate_fts_index_ddl_full, tous les champs |
| `rag3weaver/src/filter.rs` | +100 | FilterCompiler: split() + to_tantivy_json() |
| `rag3weaver/src/search.rs` | +50/-30 | tantivy_filters + allowed_ids, supprimer post-filter |
| `rag3weaver/src/catalog.rs` | +40/-20 | Orchestration split → pré-résolution → appels |
| `rag3weaver/src/lib.rs` | +2 | Exports |

**Total : ~280 lignes ajoutées, ~50 supprimées, ~22 nouveaux tests.**

## Vérification

```bash
# Étape 1 (handle.rs + query.rs)
cd packages/rag3db/extension/tantivy/ld-tantivy && cargo test --lib

# Étapes 2-5
cd packages/rag3db/extension/rag3weaver && cargo test --lib
```

## Ce qu'on ne fait PAS (V1)

- Pas de vector search via Tantivy
- Pas de seuil adaptatif sur allowed_ids
- Pas d'optimisation Should mixte (tout tombe en Kuzu)
- Pas de cache de pré-résolution Kuzu
- Pas de filter-only query Tantivy (pour pré-filtrer le vector search aussi)
