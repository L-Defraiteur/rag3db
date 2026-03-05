# 02 — Plan filtres avances : atteindre la parite Qdrant

## Contexte

Qdrant est la reference en matiere de filtrage de payloads pour la recherche vectorielle. Son API de filtres est composable, riche, et couvre tous les cas d'usage courants. Notre `filter.rs` actuel couvre les bases mais manque de composition logique et d'operateurs avances. L'objectif est d'atteindre une parite fonctionnelle pour que rag3weaver soit une alternative credible a Qdrant pour toute application RAG — pas seulement le code.

## Etat actuel de filter.rs

### Ce qui fonctionne (325 lignes, 26 tests)

```rust
pub enum FilterOp {
    Eq(CypherValue),           // n.field = $p
    Neq(CypherValue),          // n.field <> $p
    Lt(CypherValue),           // n.field < $p
    Lte(CypherValue),          // n.field <= $p
    Gt(CypherValue),           // n.field > $p
    Gte(CypherValue),          // n.field >= $p
    In(Vec<CypherValue>),      // n.field IN $p
    HasAny(Vec<CypherValue>),  // list_any_match(n.field, v -> list_contains($p, v))
    HasAll(Vec<CypherValue>),  // list_all($p, v -> list_contains(n.field, v))
    HasNone(Vec<CypherValue>), // NOT list_any_match(n.field, v -> list_contains($p, v))
}

pub enum FilterValue {
    Direct(CypherValue),       // Shorthand pour Eq / IS NULL
    List(Vec<CypherValue>),    // Shorthand pour IN
    Ops(Vec<FilterOp>),        // Multiple ops, combinées en AND
}
```

**Cross-entity** : `"Entity.field"` → genere un MATCH clause avec la bonne direction de relation.

**Combinaison** : toutes les clauses WHERE sont jointes par AND. Pas de OR, pas de NOT compose.

### Comparaison avec Qdrant

| Capacite | Qdrant | rag3weaver | Gap |
|---|---|---|---|
| **Exact match** (string/int/bool) | `match: { value }` | `Eq(v)` | - |
| **Not equal** | `match: { except }` | `Neq(v)` | - |
| **Range** (lt/gt/gte/lte) | `range: { gte, lte }` | `Gt/Lt/Gte/Lte` | - |
| **IN** (multi-select) | `match: { any: [...] }` | `In([...])` | - |
| **Tags has-any/all/none** | Pas natif (must/should sur match) | `HasAny/HasAll/HasNone` | Nous avons plus |
| **AND** | `must: [...]` | Implicite (toutes les clauses) | - |
| **OR** | `should: [...]` | **MANQUE** | Gap 1 |
| **NOT** | `must_not: [...]` | **MANQUE** (juste Neq) | Gap 2 |
| **Nested AND/OR/NOT** | Recursif a l'infini | **MANQUE** | Gap 3 |
| **IS NULL** | `is_null: { is_null: true }` | `Direct(Null)` → IS NULL | Partiel |
| **IS NOT NULL** | `is_null: { is_null: false }` | **MANQUE** | Gap 4 |
| **IS EMPTY** | `is_empty: { is_empty: true }` | **MANQUE** | Gap 5 |
| **Values count** | `values_count: { gte, lte }` | **MANQUE** | Gap 6 |
| **Full-text match** | `full_text_match: { text }` | **MANQUE** (BM25 = search only) | Gap 7 |
| **Datetime range** | Via range sur timestamps | Timestamp FieldType existe, pas de filtre | Gap 8 |
| **Geo radius/bbox** | `geo_radius`, `geo_bounding_box` | **MANQUE** | Gap 9 |
| **Has ID** | `has_id: { has_id: [...] }` | `allowed_ids` (Lucivy-side) | Partiel |
| **Nested object** | `nested: { key, filter }` | Cross-entity via `Entity.field` | Different mais fonctionnel |

## Plan d'implementation

### Phase 1 : Composition logique (must/should/must_not)

C'est le gap le plus critique. Sans OR et NOT composes, impossible de faire des requetes comme "les documents qui sont soit en Python soit en Rust, mais pas archived".

**Nouveau type `FilterCondition`** (remplace `FilterValue` au top-level) :

```rust
/// A composable filter condition, inspired by Qdrant's must/should/must_not.
#[derive(Debug, Clone)]
pub enum FilterCondition {
    /// Direct field filter (existing behavior).
    Field { key: String, value: FilterValue },

    /// All conditions must match (AND). Default for flat filter maps.
    Must(Vec<FilterCondition>),

    /// At least one condition must match (OR).
    Should(Vec<FilterCondition>),

    /// No condition must match (NOT).
    MustNot(Vec<FilterCondition>),
}
```

**Cypher generation** :

```
Must([A, B])     → (A AND B)
Should([A, B])   → (A OR B)
MustNot([A])     → NOT (A)
Should([Must([A, B]), C])  → ((A AND B) OR C)
```

**Implementation** dans `FilterParser::parse_condition()` — recursif :

```rust
fn parse_condition(
    &mut self,
    condition: &FilterCondition,
    result_alias: &str,
    result_entity: &str,
    match_clauses: &mut Vec<String>,
    params: &mut Vec<QueryParam>,
    aliases: &mut HashMap<String, String>,
    alias_counter: &mut usize,
) -> Result<String, FilterError> {
    match condition {
        FilterCondition::Field { key, value } => {
            // Existing logic: parse key, build clause
        }
        FilterCondition::Must(conditions) => {
            let clauses: Vec<String> = conditions.iter()
                .map(|c| self.parse_condition(c, ...))
                .collect::<Result<_, _>>()?;
            Ok(format!("({})", clauses.join(" AND ")))
        }
        FilterCondition::Should(conditions) => {
            let clauses: Vec<String> = conditions.iter()
                .map(|c| self.parse_condition(c, ...))
                .collect::<Result<_, _>>()?;
            Ok(format!("({})", clauses.join(" OR ")))
        }
        FilterCondition::MustNot(conditions) => {
            let clauses: Vec<String> = conditions.iter()
                .map(|c| self.parse_condition(c, ...))
                .collect::<Result<_, _>>()?;
            Ok(format!("NOT ({})", clauses.join(" AND ")))
        }
    }
}
```

**Retrocompatibilite** : l'API actuelle `HashMap<String, FilterValue>` continue de fonctionner — c'est equivalent a `Must([Field(k1, v1), Field(k2, v2), ...])`. On ajoute un `From<HashMap<String, FilterValue>> for FilterCondition`.

**Effort** : ~150 lignes.

### Phase 2 : Nouveaux FilterOp

```rust
pub enum FilterOp {
    // Existants
    Eq(CypherValue),
    Neq(CypherValue),
    Lt(CypherValue),
    Lte(CypherValue),
    Gt(CypherValue),
    Gte(CypherValue),
    In(Vec<CypherValue>),
    HasAny(Vec<CypherValue>),
    HasAll(Vec<CypherValue>),
    HasNone(Vec<CypherValue>),

    // NOUVEAUX
    IsNull,                            // n.field IS NULL
    IsNotNull,                         // n.field IS NOT NULL
    IsEmpty,                           // size(n.field) = 0  (pour lists/strings)
    IsNotEmpty,                        // size(n.field) > 0
    ValuesCount { min: Option<usize>, max: Option<usize> },  // size(n.field) >= min AND <= max
    TextMatch(String),                 // QUERY_LUCIVY_INDEX sur un champ specifique
    Between(CypherValue, CypherValue), // n.field >= $p0 AND n.field <= $p1 (sugar)
    NotIn(Vec<CypherValue>),           // NOT (n.field IN $p)
    StartsWith(String),                // starts_with(n.field, $p)
    Contains(String),                  // contains(n.field, $p)
}
```

**Cypher generation pour chaque nouveau op** :

| Op | Cypher |
|---|---|
| `IsNull` | `{prop} IS NULL` |
| `IsNotNull` | `{prop} IS NOT NULL` |
| `IsEmpty` | `size({prop}) = 0` |
| `IsNotEmpty` | `size({prop}) > 0` |
| `ValuesCount { min: 2, max: 5 }` | `size({prop}) >= 2 AND size({prop}) <= 5` |
| `TextMatch("rust")` | sous-requete QUERY_LUCIVY_INDEX (complexe, voir ci-dessous) |
| `Between(a, b)` | `{prop} >= $p0 AND {prop} <= $p1` |
| `NotIn([...])` | `NOT ({prop} IN $p)` |
| `StartsWith("pre")` | `starts_with({prop}, $p)` |
| `Contains("sub")` | `contains({prop}, $p)` |

**TextMatch** est le plus complexe — il necessite une sous-requete Lucivy pour recuperer les node IDs, puis un filtre `_uuid IN [...]`. Alternative : generer un EXISTS avec QUERY_LUCIVY_INDEX. A etudier selon les capacites de Kuzu.

**Effort** : ~100 lignes.

### Phase 3 : API ergonomique (builder pattern)

Pour rendre les filtres composables sans construire des enums a la main :

```rust
pub struct FilterBuilder {
    conditions: Vec<FilterCondition>,
}

impl FilterBuilder {
    pub fn new() -> Self { Self { conditions: vec![] } }

    /// Add a field equality filter.
    pub fn eq(mut self, field: &str, value: impl Into<CypherValue>) -> Self {
        self.conditions.push(FilterCondition::Field {
            key: field.to_string(),
            value: FilterValue::Ops(vec![FilterOp::Eq(value.into())]),
        });
        self
    }

    /// Add a range filter.
    pub fn range(mut self, field: &str, min: Option<impl Into<CypherValue>>, max: Option<impl Into<CypherValue>>) -> Self {
        let mut ops = vec![];
        if let Some(min) = min { ops.push(FilterOp::Gte(min.into())); }
        if let Some(max) = max { ops.push(FilterOp::Lte(max.into())); }
        self.conditions.push(FilterCondition::Field {
            key: field.to_string(),
            value: FilterValue::Ops(ops),
        });
        self
    }

    /// Add an IN filter.
    pub fn is_in(mut self, field: &str, values: Vec<impl Into<CypherValue>>) -> Self { ... }

    /// Add tags filter.
    pub fn has_any(mut self, field: &str, values: Vec<impl Into<CypherValue>>) -> Self { ... }
    pub fn has_all(mut self, field: &str, values: Vec<impl Into<CypherValue>>) -> Self { ... }

    /// Add null checks.
    pub fn is_null(mut self, field: &str) -> Self { ... }
    pub fn is_not_null(mut self, field: &str) -> Self { ... }

    /// Create an OR group.
    pub fn or(conditions: Vec<FilterCondition>) -> FilterCondition {
        FilterCondition::Should(conditions)
    }

    /// Create a NOT group.
    pub fn not(conditions: Vec<FilterCondition>) -> FilterCondition {
        FilterCondition::MustNot(conditions)
    }

    /// Build the final filter condition.
    pub fn build(self) -> FilterCondition {
        FilterCondition::Must(self.conditions)
    }
}
```

**Usage** :
```rust
let filter = FilterBuilder::new()
    .eq("language", "rust")
    .range("linesOfCode", Some(10_i64), Some(500_i64))
    .has_any("modifiers", vec!["async", "pub"])
    .build();

// Equivalent Qdrant:
// { must: [
//     { key: "language", match: { value: "rust" } },
//     { key: "linesOfCode", range: { gte: 10, lte: 500 } },
//     { key: "modifiers", match: { any: ["async", "pub"] } },
// ] }
```

**Effort** : ~120 lignes.

### Phase 4 : Serde JSON pour filtres (compatibilite API)

Pour que les filtres puissent etre passes en JSON depuis JS/WASM :

```rust
// Deserialization from JSON
impl<'de> Deserialize<'de> for FilterCondition {
    // Supports both flat format and structured format:
    //
    // Flat (existing, retrocompatible):
    // { "language": "rust", "linesOfCode": { "$gte": 10 } }
    //
    // Structured (Qdrant-like):
    // { "must": [
    //     { "key": "language", "match": { "value": "rust" } },
    //     { "key": "linesOfCode", "range": { "gte": 10, "lte": 500 } }
    // ] }
    //
    // Mixed OK:
    // { "must": [...], "should": [...], "must_not": [...] }
}
```

**Format JSON pour les operateurs** (convention `$` prefix, comme MongoDB) :

```json
{
    "language": "rust",
    "linesOfCode": { "$gte": 10, "$lte": 500 },
    "modifiers": { "$hasAny": ["async", "pub"] },
    "deprecated": { "$isNull": true },
    "tags": { "$valuesCount": { "min": 1, "max": 10 } }
}
```

**Format structure (Qdrant-like)** :
```json
{
    "must": [
        { "key": "language", "match": { "value": "rust" } }
    ],
    "should": [
        { "key": "scopeType", "match": { "value": "class" } },
        { "key": "scopeType", "match": { "value": "interface" } }
    ],
    "must_not": [
        { "key": "deprecated", "match": { "value": true } }
    ]
}
```

**Effort** : ~200 lignes (serde custom deserializer).

### Phase 5 : Integration dans SearchOptions

```rust
pub struct SearchOptions {
    // ... existants ...
    pub filters: HashMap<String, FilterValue>,  // CONSERVE pour retrocompat

    // NOUVEAU : filtre compose (prend precedence si present)
    pub filter: Option<FilterCondition>,
}
```

Dans `catalog.search()` :
```rust
let effective_filter = if let Some(filter) = &options.filter {
    filter.clone()
} else if !options.filters.is_empty() {
    FilterCondition::from(options.filters.clone())  // conversion auto
} else {
    return // pas de filtre
};
```

**Effort** : ~30 lignes.

## Bilan effort total

| Phase | Description | Effort estime |
|---|---|---|
| 1 | Composition logique (must/should/must_not) | ~150 lignes |
| 2 | Nouveaux FilterOp (IsNull, ValuesCount, etc.) | ~100 lignes |
| 3 | FilterBuilder (API ergonomique) | ~120 lignes |
| 4 | Serde JSON (2 formats) | ~200 lignes |
| 5 | Integration SearchOptions | ~30 lignes |
| **Total** | | **~600 lignes** |

## Tests prevus

| Test | Phase | Verifie |
|---|---|---|
| `should_generates_or` | 1 | `Should([Eq("a"), Eq("b")])` → `(... OR ...)` |
| `must_not_generates_not` | 1 | `MustNot([Eq("x")])` → `NOT (...)` |
| `nested_composition` | 1 | `Must([Should([A, B]), Field(C)])` → `((A OR B) AND C)` |
| `is_null_op` | 2 | `IsNull` → `n.field IS NULL` |
| `is_not_null_op` | 2 | `IsNotNull` → `n.field IS NOT NULL` |
| `values_count_op` | 2 | `ValuesCount { min: 2, max: 5 }` → `size(...) >= 2 AND size(...) <= 5` |
| `between_op` | 2 | `Between(10, 50)` → `>= $p AND <= $p` |
| `starts_with_op` | 2 | `StartsWith("pre")` → `starts_with(...)` |
| `not_in_op` | 2 | `NotIn([...])` → `NOT (... IN $p)` |
| `builder_eq_range` | 3 | Builder produit les bonnes conditions |
| `builder_or_group` | 3 | `FilterBuilder::or(...)` → Should |
| `json_flat_format` | 4 | `{"field": "val"}` deserialise en Field + Eq |
| `json_dollar_ops` | 4 | `{"$gte": 10}` deserialise en Ops(Gte) |
| `json_structured_format` | 4 | `{"must": [...]}` deserialise en Must |
| `retrocompat_hashmap` | 5 | `HashMap<String, FilterValue>` continue de marcher |

## Comparaison finale apres implementation

| Capacite | Qdrant | rag3weaver apres | Notes |
|---|---|---|---|
| AND/OR/NOT compose | `must/should/must_not` | `Must/Should/MustNot` | Parite |
| Range | `range` | `Lt/Gt/Gte/Lte/Between` | Parite + sugar |
| Exact/Multi-select | `match` | `Eq/In` | Parite |
| Null/Empty checks | `is_null/is_empty` | `IsNull/IsNotNull/IsEmpty/IsNotEmpty` | Parite |
| Values count | `values_count` | `ValuesCount` | Parite |
| Full-text in filter | `full_text_match` | `TextMatch` | Parite |
| Tags | Pas natif | `HasAny/HasAll/HasNone` | Nous avons plus |
| String ops | Pas natif | `StartsWith/Contains` | Nous avons plus |
| Cross-entity | Pas natif (nested seulement) | `Entity.field` → MATCH | Nous avons plus (graph-aware) |
| Geo | `geo_radius/bbox/polygon` | Pas prevu | Pas pertinent pour code RAG |
| Builder API | SDK clients | `FilterBuilder` | Parite |
| JSON API | REST payload | Serde deserializer | Parite |

**Resultat** : parite Qdrant sur tous les axes pertinents, avec 3 avantages exclusifs (Tags natifs, String ops, Cross-entity graph-aware). Seul le geo est absent (pas pertinent pour nos use-cases actuels).
