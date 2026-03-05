# Rag3Weaver — Etape 1a : Port du L1 TypeScript (schema + query)

Date : 15 fevrier 2026
Statut : DESIGN

---

## Contexte

Le L1 TypeScript (`kuzu-wasm-exp/src/lib/l1/`) contient 4 classes :
- `NodeTableBuilder` — fluent API pour definir des node tables + generer le DDL Cypher
- `RelTableBuilder` — fluent API pour definir des rel tables
- `QueryBuilder` — construction de queries MATCH parametrees (WHERE/ORDER/LIMIT)
- `SchemaBuilder` — orchestrateur (apply, indexes, insert, transactions)

En Rust, on ne reprend PAS le pattern fluent builder. On part de `CatalogConfig` (deja dans le squelette, serde) et on genere le Cypher directement. C'est plus idiomatique Rust et plus simple.

**Ce qui change par rapport au TS** :
- Pas de `NodeTableBuilder`/`RelTableBuilder` classes — remplace par des fonctions pures `config → Cypher`
- Pas de `SchemaBuilder` classe — les fonctions de generation sont dans `schema.rs`, l'execution sera dans `catalog.rs` (Etape 2)
- `QueryBuilder` garde le pattern builder car c'est utile pour construire des queries incrementalement

**Ce qui reste identique** :
- Le Cypher genere (CREATE NODE TABLE, CREATE REL TABLE, indexes)
- Le query builder parametrise (WHERE conditions, ORDER BY, LIMIT/OFFSET)
- La validation des identifiants
- Les colonnes systeme (_uuid, _content_hash, embeddings)

---

## Mapping TS → Rust

| TS L1 | Rust | Module |
|-------|------|--------|
| `NodeTableBuilder.toCypher()` | `generate_node_table_ddl()` | `schema.rs` |
| `NodeTableBuilder.toInsertCypher()` | `generate_insert_cypher()` | `schema.rs` |
| `RelTableBuilder.toCypher()` | `generate_rel_table_ddl()` | `schema.rs` |
| `SchemaBuilder.createVectorIndex()` | `generate_vector_index_ddl()` | `schema.rs` |
| `SchemaBuilder.createFTSIndex()` | `generate_fts_index_ddl()` | `schema.rs` |
| `SchemaBuilder.apply()` | sera dans `catalog.rs` (Etape 2) | — |
| `SchemaBuilder.insert()` | sera dans `catalog.rs` (Etape 2) | — |
| `SchemaBuilder.transaction()` | sera dans `catalog.rs` (Etape 2) | — |
| `validateIdentifier()` | `validate_identifier()` | `schema.rs` |
| `KuzuDataType` type | `field_type_to_kuzu()` | `schema.rs` |
| `QueryBuilder` | `QueryBuilder` | `query.rs` |
| `WhereCondition` | `WhereCondition` enum | `query.rs` |
| `PreparedQuery` | `PreparedQuery` struct | `query.rs` |
| `CypherValue`, `QueryParam` | deja dans `connection.rs` | squelette |
| `CatalogConfig`, `EntityDef`, etc. | deja dans `config.rs` | squelette |

---

## Deja couvert par le squelette (Etape 0)

Ces types existent deja et seront reutilises directement :

- `CypherValue` (Null, Bool, Int, Float, String, List, Map) — `connection.rs`
- `QueryParam { name, value }` — `connection.rs`
- `QueryResult { columns, rows }` — `connection.rs`
- `CatalogConfig` avec entities, relations, knowledge_bases, embedding_dim — `config.rs`
- `EntityDef { fields, hashsafe }` — `config.rs`
- `FieldDef { field_type, title_for, content_for, chunked, boost }` — `config.rs`
- `RelationDef { from, to, properties }` — `config.rs`
- `FieldType` (String, Text, Int64, Integer, Double, Number, Boolean, Timestamp, Json, Tags, Choice) — `config.rs`
- `KBConfig { search, keyword_weight, chunking, ... }` — `config.rs`

---

## Module schema.rs — Generation DDL pure

Fonctions pures : `CatalogConfig` → `Vec<String>` (Cypher DDL). Zero DB, zero async, 100% testable.

### Mapping FieldType → Kuzu

```rust
pub fn field_type_to_kuzu(ft: &FieldType) -> &'static str {
    match ft {
        FieldType::String | FieldType::Text | FieldType::Json | FieldType::Tags | FieldType::Choice => "STRING",
        FieldType::Int64 | FieldType::Integer => "INT64",
        FieldType::Double | FieldType::Number => "DOUBLE",
        FieldType::Boolean => "BOOLEAN",
        FieldType::Timestamp => "TIMESTAMP",
    }
}
```

Note : `Text`, `Json`, `Tags`, `Choice` sont stockes comme STRING dans Kuzu. La distinction semantique vit dans la config (pour le chunking, l'indexation FTS, etc.).

### Validation des identifiants

```rust
pub fn validate_identifier(name: &str, kind: &str) -> Result<(), SchemaError> {
    // Regex: [a-zA-Z_][a-zA-Z0-9_]*
    // Identique au TS: const VALID_IDENTIFIER = /^[a-zA-Z_][a-zA-Z0-9_]*$/;
}
```

Pas de regex crate (overhead WASM). Implementable en pur Rust :

```rust
fn is_valid_ident(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
```

### generate_node_table_ddl

Pour chaque entite dans `config.entities`, genere un CREATE NODE TABLE avec :

1. **`_uuid STRING` + PRIMARY KEY** — colonne systeme, toujours presente
2. **`_content_hash STRING`** — pour detecter les changements (HASHSAFE)
3. **Champs utilisateur** — mappes via `field_type_to_kuzu()`
4. **Colonnes embedding** — une par knowledge base qui reference cette entite : `{kb}_embedding FLOAT[{dim}]`

```
CREATE NODE TABLE IF NOT EXISTS Document(
    _uuid STRING,
    _content_hash STRING,
    title STRING,
    body STRING,
    page_count INT64,
    main_embedding FLOAT[384],
    PRIMARY KEY(_uuid)
)
```

**Resolution des KB** : une KB reference une entite si un de ses champs a `content_for` ou `title_for` pointant vers cette KB. Scanner les champs pour determiner quelles KBs sont liees.

### generate_chunk_table_ddl

Pour chaque entite ayant des champs `chunked: true`, genere une table de chunks :

```
CREATE NODE TABLE IF NOT EXISTS Document_Chunk(
    _uuid STRING,
    _parent_uuid STRING,
    _parent_field STRING,
    _kb_name STRING,
    _text STRING,
    _text_hash STRING,
    _index INT64,
    _start_char INT64,
    _end_char INT64,
    _start_line INT64,
    _end_line INT64,
    _core_start_char INT64,
    _core_end_char INT64,
    _core_start_line INT64,
    _core_end_line INT64,
    embedding FLOAT[384],
    PRIMARY KEY(_uuid)
)
```

Plus une relation :

```
CREATE REL TABLE IF NOT EXISTS Document_HAS_CHUNK(FROM Document TO Document_Chunk)
```

### generate_rel_table_ddl

Pour chaque relation dans `config.relations` :

```
CREATE REL TABLE IF NOT EXISTS REFERENCES(FROM Document TO Document, role STRING)
```

Les properties optionnelles sont mappees via `field_type_to_kuzu()`.

### generate_vector_index_ddl

Pour chaque entite × KB ayant une colonne embedding :

```
CALL CREATE_VECTOR_INDEX('Document', 'Document_main_vec', 'main_embedding', metric := 'cosine')
```

Et pour les chunks :

```
CALL CREATE_VECTOR_INDEX('Document_Chunk', 'Document_Chunk_vec', 'embedding', metric := 'cosine')
```

### generate_fts_index_ddl

Pour chaque entite × KB, on cree un index Lucivy sur les champs `title_for` et `content_for` :

```
CALL CREATE_LUCIVY_INDEX('Document', ['title', 'body'])
```

Note : on utilise `CREATE_LUCIVY_INDEX` (pas `CREATE_FTS_INDEX`). C'est notre extension custom rag3db.

### generate_meta_table_ddl

Table systeme pour persister la config du catalog :

```
CREATE NODE TABLE IF NOT EXISTS _catalog_meta(
    _key STRING,
    _value STRING,
    PRIMARY KEY(_key)
)
```

### generate_insert_cypher

Genere un INSERT parametre pour une entite :

```rust
pub fn generate_insert_cypher(entity_name: &str, column_names: &[&str]) -> String {
    // CREATE (:Document {_uuid: $_uuid, _content_hash: $_content_hash, title: $title, ...})
}
```

Les noms de colonnes incluent les colonnes systeme + utilisateur + embeddings.

### generate_full_schema

Orchestre tout dans l'ordre :

```rust
pub fn generate_full_schema(config: &CatalogConfig) -> Result<Vec<String>, SchemaError> {
    let mut statements = Vec::new();
    // 1. Table meta
    // 2. Node tables (entites)
    // 3. Chunk tables (pour les entites avec champs chunked)
    // 4. Rel tables (relations utilisateur + HAS_CHUNK)
    // 5. (indexes seront crees apres apply, pas dans le DDL initial)
    Ok(statements)
}
```

Les indexes (vector + FTS) sont dans des fonctions separees car ils necessitent que les tables existent deja. Ils seront appeles par `catalog.rs` apres le DDL.

### SchemaError

```rust
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("invalid {kind} name: \"{name}\" — must match [a-zA-Z_][a-zA-Z0-9_]*")]
    InvalidIdentifier { kind: String, name: String },

    #[error("entity \"{0}\" not found in config")]
    EntityNotFound(String),

    #[error("relation \"{rel}\" references unknown entity \"{entity}\"")]
    UnknownEntity { rel: String, entity: String },

    #[error("no chunked fields in entity \"{0}\" but chunking is expected")]
    NoChunkedFields(String),
}
```

---

## Module query.rs — Query Builder parametrise

Builder pour des queries MATCH parametrees. Logique pure, pas d'execution (l'execution passe par `DbConnection` du squelette).

### Structs

```rust
/// Query preparee avec ses parametres.
pub struct PreparedQuery {
    pub cypher: String,
    pub params: Vec<QueryParam>,  // reutilise de connection.rs
}

/// Operateur de comparaison pour WHERE.
#[derive(Debug, Clone)]
pub enum WhereOp {
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    In,
    NotIn,
    IsNull,
}

/// Direction de tri.
#[derive(Debug, Clone)]
pub enum SortDir {
    Asc,
    Desc,
}
```

### QueryBuilder

```rust
pub struct QueryBuilder {
    table: String,
    alias: String,          // default "n"
    select: Vec<String>,    // default ["*"]
    conditions: Vec<WhereClause>,
    raw_where: Vec<String>,
    order_by: Vec<(String, SortDir)>,
    limit: Option<i64>,
    offset: Option<i64>,
    param_counter: usize,
}

struct WhereClause {
    field: String,
    op: WhereOp,
    value: Option<CypherValue>,  // None pour IsNull
}
```

### API fluente

```rust
impl QueryBuilder {
    pub fn new(table: &str) -> Self;
    pub fn alias(mut self, alias: &str) -> Self;
    pub fn select(mut self, fields: &[&str]) -> Self;
    pub fn select_all(mut self) -> Self;

    // WHERE conditions
    pub fn where_eq(mut self, field: &str, value: impl Into<CypherValue>) -> Self;
    pub fn where_neq(mut self, field: &str, value: impl Into<CypherValue>) -> Self;
    pub fn where_lt(mut self, field: &str, value: impl Into<CypherValue>) -> Self;
    pub fn where_lte(mut self, field: &str, value: impl Into<CypherValue>) -> Self;
    pub fn where_gt(mut self, field: &str, value: impl Into<CypherValue>) -> Self;
    pub fn where_gte(mut self, field: &str, value: impl Into<CypherValue>) -> Self;
    pub fn where_in(mut self, field: &str, values: Vec<CypherValue>) -> Self;
    pub fn where_not_in(mut self, field: &str, values: Vec<CypherValue>) -> Self;
    pub fn where_null(mut self, field: &str) -> Self;
    pub fn where_raw(mut self, clause: &str) -> Self;

    // ORDER BY, LIMIT, OFFSET
    pub fn order_by(mut self, field: &str, dir: SortDir) -> Self;
    pub fn limit(mut self, n: i64) -> Self;
    pub fn offset(mut self, n: i64) -> Self;

    // Build
    pub fn build(&self) -> PreparedQuery;
}
```

### Cypher genere par build()

```
MATCH (n:Document)
WHERE n.title = $p0 AND n.page_count > $p1
RETURN n
ORDER BY n.title ASC
SKIP $p2
LIMIT $p3
```

Parametres : `[{p0: "hello"}, {p1: 5}, {p2: 10}, {p3: 20}]`

- Chaque valeur utilise un parametre unique `$pN` (compteur incremental)
- `where_null` genere `n.field IS NULL` sans parametre
- `where_in` genere `n.field IN $pN` avec une valeur `CypherValue::List`
- `where_raw` insere une clause brute (pas de parametre, pour cas speciaux)
- `select_all` genere `RETURN n`, `select(["title", "body"])` genere `RETURN n.title, n.body`

### InsertBuilder

Generer un INSERT parametre pour un ensemble de colonnes :

```rust
pub fn build_insert(table: &str, columns: &[&str]) -> PreparedQuery {
    // CREATE (:Document {col1: $col1, col2: $col2, ...})
}
```

Simple fonction, pas besoin de builder car l'insert est toujours one-shot.

---

## Fichiers a creer/modifier

| Fichier | Action |
|---------|--------|
| `src/schema.rs` | Creer — toute la generation DDL |
| `src/query.rs` | Creer — QueryBuilder + PreparedQuery |
| `src/lib.rs` | Modifier — ajouter `pub mod schema; pub mod query;` + re-exports |

---

## Resolution des KBs par entite

Point important : dans la config, les KBs ne listent pas explicitement quelles entites elles couvrent. C'est implicite via les champs `title_for` et `content_for` des `FieldDef`.

Algorithme :

```
Pour chaque entite E :
    Pour chaque champ F de E :
        Si F.title_for == "kb_name" → E est liee a kb_name (titre)
        Si F.content_for contient "kb_name" → E est liee a kb_name (contenu)
    Pour chaque KB liee :
        Ajouter une colonne embedding `{kb_name}_embedding FLOAT[dim]`
        Generer les indexes vector + FTS pour cette combinaison E × KB
```

Fonction helper utile :

```rust
/// Pour une entite, retourne les KBs liees et les champs titre/contenu par KB.
pub fn resolve_entity_kbs(
    entity_name: &str,
    entity_def: &EntityDef,
) -> HashMap<String, KBFieldMapping> {
    // ...
}

pub struct KBFieldMapping {
    pub title_field: Option<String>,
    pub content_fields: Vec<String>,
}
```

Cette fonction est reutilisee par :
- `generate_node_table_ddl` (colonnes embedding)
- `generate_vector_index_ddl` (indexes a creer)
- `generate_fts_index_ddl` (champs a indexer)

---

## Tests prevus

### schema.rs (~20 tests)

| Test | Verifie |
|------|---------|
| `validate_identifier_valid` | noms corrects passent |
| `validate_identifier_invalid` | noms avec espaces/tirets/chiffres en tete echouent |
| `field_type_to_kuzu_all` | tous les FieldType mappent correctement |
| `node_table_basic` | CREATE NODE TABLE avec _uuid, _content_hash, user fields, PRIMARY KEY |
| `node_table_with_embedding` | colonne embedding pour une KB liee |
| `node_table_multi_kb` | plusieurs colonnes embedding pour plusieurs KBs |
| `chunk_table_basic` | CREATE NODE TABLE Entity_Chunk avec toutes les colonnes systeme |
| `chunk_table_has_embedding` | colonne embedding dans la table chunk |
| `chunk_rel` | CREATE REL TABLE Entity_HAS_CHUNK |
| `rel_table_basic` | CREATE REL TABLE sans properties |
| `rel_table_with_properties` | CREATE REL TABLE avec properties mappees |
| `rel_table_validates_endpoints` | echoue si from/to referencent des entites inconnues |
| `vector_index_ddl` | CALL CREATE_VECTOR_INDEX correct |
| `fts_index_ddl` | CALL CREATE_LUCIVY_INDEX correct |
| `meta_table_ddl` | CREATE NODE TABLE _catalog_meta |
| `insert_cypher_basic` | CREATE (:Entity {_uuid: $_uuid, ...}) |
| `full_schema_order` | meta → nodes → chunks → rels, dans l'ordre |
| `full_schema_real_config` | config complete (entites, relations, KBs) → DDL coherent |
| `resolve_entity_kbs` | resolution correcte des KBs par entite |
| `no_kb_no_embedding` | entite sans KB liee = pas de colonne embedding |

### query.rs (~15 tests)

| Test | Verifie |
|------|---------|
| `basic_match` | MATCH (n:Table) RETURN n |
| `select_fields` | RETURN n.title, n.body |
| `where_eq` | WHERE n.field = $p0 |
| `where_neq` | WHERE n.field <> $p0 |
| `where_lt_lte_gt_gte` | operateurs de comparaison |
| `where_null` | WHERE n.field IS NULL |
| `where_in` | WHERE n.field IN $p0 |
| `where_not_in` | NOT n.field IN $p0 |
| `where_multiple` | conditions combinees avec AND |
| `where_raw` | clause brute inseree |
| `order_by` | ORDER BY n.field ASC |
| `order_by_multi` | ORDER BY n.f1 ASC, n.f2 DESC |
| `limit_offset` | SKIP $pN LIMIT $pM |
| `params_unique` | chaque parametre a un nom unique |
| `build_insert` | CREATE (:Table {col: $col, ...}) |

---

## Dependances nouvelles

Aucune. Tout est du string building pur avec les types deja dans le squelette (CypherValue, QueryParam, CatalogConfig, FieldType, etc.).

Le seul ajout est `thiserror` pour `SchemaError`, deja en dependance.

---

## Prochaines etapes apres L1

- **L2** : `chunker.rs` (SemanticChunker), `uuid.rs` (HASHSAFE), `hash.rs` (content hashing)
- **L3** : `catalog.rs` + `pipeline.rs` + `queue.rs` (le coeur async qui execute le schema et gere le CRUD batch)
