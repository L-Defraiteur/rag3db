# Rag3Weaver — Etape 1 : Port L1 + L2 (schema, query, hash, uuid, chunker, fusion)

Date : 15 fevrier 2026
Statut : FAIT

---

## Contexte

Suite au squelette (Etape 0 — events, config, traits, 35 tests), on porte les niveaux L1 et L2 du TypeScript existant (`kuzu-wasm-exp/src/lib/l1/` et `l2/`) en Rust.

**Approche** : adapter niveau par niveau depuis le TS, en collant au squelette existant. Ce qui peut reutiliser les types du squelette le fait (CypherValue, QueryParam, CatalogConfig, FieldType...). Ce qui doit etre ajoute l'est comme module pur (zero DB, zero async).

**Seule nouvelle dependance** : `blake3 = "1"` (no_std, WASM SIMD, 3x SHA-256).

---

## Emplacement

```
packages/rag3db/extension/rag3weaver/src/
├── lib.rs            (10 modules, re-exports)
├── events.rs         (Etape 0)
├── config.rs         (Etape 0)
├── embedder.rs       (Etape 0)
├── connection.rs     (Etape 0)
├── schema.rs         ← NOUVEAU (L1)
├── query.rs          ← NOUVEAU (L1)
├── hash.rs           ← NOUVEAU (L2)
├── uuid.rs           ← NOUVEAU (L2)
├── chunker.rs        ← NOUVEAU (L2)
└── fusion.rs         ← NOUVEAU (design doc Etape 1)
```

---

## Module schema.rs — Config → Cypher DDL

Source TS : `NodeTableBuilder.toCypher()`, `RelTableBuilder.toCypher()`, `SchemaBuilder.createVectorIndex/createFTSIndex`, `validateIdentifier`

**Difference avec le TS** : pas de pattern fluent builder. On part directement de `CatalogConfig` (serde) et on genere le Cypher via des fonctions pures. Plus idiomatique Rust, plus simple.

### Fonctions publiques

| Fonction | Genere |
|----------|--------|
| `validate_identifier(name, kind)` | Verifie `[a-zA-Z_][a-zA-Z0-9_]*`, pas de regex crate |
| `field_type_to_kuzu(ft)` | FieldType → STRING/INT64/DOUBLE/BOOLEAN/TIMESTAMP |
| `resolve_entity_kbs(entity_def)` | Scan title_for/content_for → map KB → champs |
| `generate_node_table_ddl(name, entity, dim)` | CREATE NODE TABLE avec _uuid, _content_hash, user fields, embeddings, PK |
| `generate_chunk_table_ddl(entity, dim)` | CREATE NODE TABLE Entity_Chunk (15 colonnes systeme + embedding) |
| `generate_chunk_rel_ddl(entity)` | CREATE REL TABLE Entity_HAS_CHUNK |
| `generate_rel_table_ddl(name, rel, config)` | CREATE REL TABLE avec validation des endpoints |
| `generate_vector_index_ddl(table, col, idx)` | CALL CREATE_VECTOR_INDEX(..., metric := 'cosine') |
| `generate_fts_index_ddl(table, fields)` | CALL CREATE_LUCIVY_INDEX('Table', ['col1', 'col2']) |
| `generate_meta_table_ddl()` | CREATE NODE TABLE _catalog_meta (_key, _value, PK) |
| `generate_insert_cypher(table, cols)` | CREATE (:Table {col: $col, ...}) |
| `entity_has_chunks(entity)` | true si au moins un champ chunked |
| `generate_full_schema(config)` | Orchestre tout → FullSchema { ddl, indexes } |

### FullSchema

```rust
pub struct FullSchema {
    pub ddl: Vec<String>,     // CREATE TABLE (executer d'abord)
    pub indexes: Vec<String>, // CREATE INDEX (executer apres les tables)
}
```

Ordre DDL : meta → node tables → chunk tables → chunk rels → user rels.
Les indexes sont separes car ils necessitent que les tables existent.

### Resolution des KBs

```rust
pub struct KBFieldMapping {
    pub title_field: Option<String>,
    pub content_fields: Vec<String>,
}
```

Algorithme : pour chaque champ d'une entite, si `title_for = "kb"` ou `content_for` contient `"kb"`, l'entite est liee a cette KB. Genere une colonne `{kb}_embedding FLOAT[dim]` + indexes vector/FTS.

### SchemaError

- `InvalidIdentifier { kind, name }` — nom invalide
- `UnknownEntity { rel, entity }` — relation pointe vers une entite absente

### 22 tests

validate_identifier (valid/invalid), field_type_to_kuzu, resolve_entity_kbs (basic/multi_kb/no_kb), node_table (basic/embedding/multi_kb/invalid), chunk_table, chunk_rel, rel_table (basic/properties/unknown_entity), vector_index, fts_index, meta_table, insert_cypher, full_schema (order/no_kb/validates_endpoints).

---

## Module query.rs — QueryBuilder parametrise

Source TS : `QueryBuilder.ts` (WHERE/ORDER/LIMIT parametrise, prepared statements)

### API fluente

```rust
let q = QueryBuilder::new("Document")
    .select(&["title", "body"])
    .where_eq("status", "published")
    .where_gt("page_count", 10_i64)
    .order_by("title", SortDir::Asc)
    .offset(5)
    .limit(25)
    .build();
// q.cypher = "MATCH (n:Document)\nWHERE n.status = $p0 AND n.page_count > $p1\n..."
// q.params = [{p0: "published"}, {p1: 10}, {p2: 5}, {p3: 25}]
```

### Conditions WHERE

| Methode | Cypher genere |
|---------|---------------|
| `where_eq(f, v)` | `n.f = $pN` |
| `where_neq(f, v)` | `n.f <> $pN` |
| `where_lt/lte/gt/gte(f, v)` | `n.f < $pN` etc. |
| `where_in(f, vals)` | `n.f IN $pN` (CypherValue::List) |
| `where_not_in(f, vals)` | `NOT n.f IN $pN` |
| `where_null(f)` | `n.f IS NULL` (pas de parametre) |
| `where_raw(clause)` | clause brute inseree telle quelle |

Chaque parametre a un nom unique `$pN` (compteur incremental). SKIP et LIMIT sont aussi parametrises.

### PreparedQuery

```rust
pub struct PreparedQuery {
    pub cypher: String,
    pub params: Vec<QueryParam>,  // reutilise de connection.rs
}
```

### build_insert()

```rust
pub fn build_insert(table: &str, columns: &[&str]) -> String;
// "CREATE (:Document {_uuid: $_uuid, title: $title, body: $body})"
```

### 17 tests

basic_match, select_fields, custom_alias, where_eq/neq/comparisons/null/in/not_in/multiple/raw, order_by (single/multi), limit_offset, params_unique, build_insert, full_query.

---

## Module hash.rs — Content hashing (blake3)

Source TS : `utils.sha256()` (Web Crypto)

```rust
pub fn content_hash(text: &str) -> String {
    blake3::hash(text.as_bytes()).to_hex().to_string()
}
```

**Pourquoi blake3 au lieu de SHA-256** :
- 3x plus rapide en natif
- WASM SIMD accelere (feature `wasm32_simd`)
- `no_std` compatible
- 6000+ stars, maintenu activement

Retourne un hex string de 64 caracteres. Utilise pour `_content_hash` (detection de changements) et comme base pour les UUIDs.

### 4 tests

deterministic, different_inputs, hex_length_64, empty_string.

---

## Module uuid.rs — HASHSAFE UUIDs deterministes

Source TS : `UUIDGenerator.ts` + `utils.hashToUUID()`

### Fonctions

```rust
pub fn hash_to_uuid(hash_hex: &str) -> String;
// "abcdef12-3456-7890-abcd-ef1234567890" (format 8-4-4-4-12)

pub fn hashsafe_uuid(entity_name: &str, field_values: &[&str]) -> String;
// "entity_name:val1|val2|..." → blake3 → UUID

pub fn chunk_uuid(parent_uuid: &str, index: usize) -> String;
// "parent_uuid|chunk|0" → blake3 → UUID
```

**Amelioration vs le TS** : le nom de l'entite est inclus dans le hash (`entity_name:values`). Ca previent les collisions entre entites differentes ayant les memes valeurs de champs. Le TS ne faisait que `values.join('|')`.

### 10 tests

hash_to_uuid_format, hashsafe (deterministic/different_values/entity_name_matters/field_order_matters/empty_fields), chunk_uuid (deterministic/different_index/different_parent), uuid_is_valid_format.

---

## Module chunker.rs — Chunking avec overlap et tracking d'offsets

Source TS : `Chunker.ts` (decoupe avec overlap, break points)

### Chunk

```rust
pub struct Chunk {
    pub text: String,
    pub index: usize,       // position sequentielle (0, 1, 2, ...)
    pub start_byte: usize,  // offset octet dans le texte original
    pub end_byte: usize,
    pub start_line: usize,  // numero de ligne (0-based)
    pub end_line: usize,
}
```

### ChunkerConfig

```rust
pub struct ChunkerConfig {
    pub max_size: usize,       // default 1500 (caracteres)
    pub overlap: usize,        // default 200
    pub strategy: ChunkStrategy, // Semantic, Fixed, Sentence
}
```

### Strategies

| Strategie | Delimiteurs cherches (par priorite) |
|-----------|-------------------------------------|
| Semantic | `\n\n`, `\n`, `. `, `! `, `? `, `; `, `, `, ` ` |
| Sentence | `. `, `! `, `? `, `\n\n`, `\n` |
| Fixed | Aucun (coupe a max_size) |

### Algorithme

1. Texte court (≤ max_size) → un seul chunk
2. Sinon, boucle :
   - Calculer `end = min(pos + max_size, text.len())`
   - Si pas a la fin : chercher un break point depuis le milieu du range vers la fin (`rfind` sur les delimiteurs par priorite)
   - Extraire et trimmer le chunk
   - Avancer avec overlap : `next_pos = end - overlap` (minimum pos + 1)

### Break point

Cherche le dernier delimiteur entre `midpoint..max_end`, en essayant chaque delimiteur par priorite decroissante (`\n\n` avant `\n` avant `. ` etc.). Retourne la position juste apres le delimiteur.

### Tracking de lignes

Index cumulatif pre-calcule en O(n) :

```rust
fn build_line_index(text: &str) -> Vec<usize>
// line_at[i] = nombre de \n dans text[..i]
```

Lookup O(1) par chunk. Gere correctement l'overlap (pas de double-comptage).

### UTF-8 safety

`snap_to_char_boundary()` recule au boundary le plus proche pour eviter de couper un caractere multi-octets.

### Ameliorations vs le TS

- Tracking des lignes directement dans le chunker (le TS le faisait apres coup dans DocumentStore)
- Index cumulatif O(n) au lieu de comptage incremental (correct avec overlap)
- UTF-8 safe (le TS travaille en UTF-16 nativement)
- 3 strategies (Semantic, Sentence, Fixed) au lieu d'une seule
- Offsets en octets (pret pour le stockage DB), pas en "tokens estimes"

### 19 tests

empty_text, whitespace_only, short_text_single_chunk, splits_at_paragraph/sentence_boundary, overlap_produces_shared_content, sequential_indices, fixed (splits_at_size/with_overlap), offsets_cover_full_text, line_tracking (basic/no_newlines), utf8_multibyte_chars, overlap_larger_than_chunk, single_long_word, default_config, count_newlines_helper, snap_to_char_boundary (ascii/utf8).

---

## Module fusion.rs — Score fusion pour hybrid search

Source : design doc (Etape 1) + findings crates

3 fonctions pures, zero dependance :

```rust
pub fn boost_fuse(vector_score: f32, bm25_normalized: f32, boost_factor: f32) -> f32;
// vector × (1 + bm25 × boost)

pub fn weighted_fuse(vector_score: f32, bm25_score: f32, keyword_weight: f32) -> f32;
// (1 - w) × vector + w × bm25

pub fn rrf_fuse(ranked_lists: &[&[(String, f32)]], k: f32) -> Vec<(String, f32)>;
// Reciprocal Rank Fusion : score = Σ 1/(k + rank + 1)
```

| Strategie | Quand l'utiliser |
|-----------|------------------|
| boost | Vector est le signal principal, keywords en bonus |
| weighted | Equilibre configurable (keyword_weight 0.2–0.4 typique) |
| RRF | Score-agnostic, robuste quand les echelles vector/BM25 different |

### 11 tests

boost (no_keyword/with_keyword/zero_vector), weighted (pure_semantic/pure_keyword/balanced), rrf (single_list/two_lists_shared/disjoint/empty/k_parameter_effect).

---

## Bilan

### Tests

| Module | Tests | Source |
|--------|:-----:|--------|
| events.rs | 5 | Etape 0 |
| config.rs | 11 | Etape 0 |
| embedder.rs | 5 | Etape 0 |
| connection.rs | 14 | Etape 0 |
| schema.rs | 22 | L1 |
| query.rs | 17 | L1 |
| hash.rs | 4 | L2 |
| uuid.rs | 10 | L2 |
| chunker.rs | 19 | L2 |
| fusion.rs | 11 | Design doc |
| **Total** | **118** | |

```bash
cd packages/rag3db/extension/rag3weaver && cargo test
# 118 passed, 0 failed, 0 warnings
```

### Dependances

```toml
[dependencies]
async-broadcast = "0.7"     # Events (runtime-agnostic, WASM)
blake3 = "1"                 # Hashing (3x SHA-256, no_std, WASM SIMD)
serde = { version = "1", features = ["derive"] }
serde_json = "1"
async-trait = "0.1"
thiserror = "2"

[dev-dependencies]
tokio = { version = "1", features = ["rt", "macros", "time"] }
```

### Ce qui reste pour text-splitter

Le chunker actuel est un port du TS (decoupe par delimiteurs). `text-splitter` pourra etre branche plus tard pour :
- Decoupe markdown-aware (respecte headers, code blocks, listes)
- Decoupe code-aware (tree-sitter, par fonctions/classes)
- Sizing par tokens (tiktoken, HuggingFace) au lieu de caracteres

L'interface `Chunker.chunk(text) -> Vec<Chunk>` ne changera pas — seule l'implementation interne evoluera.

---

## Prochaines etapes

### L3 — Catalog CRUD + pipeline async

Ce qui manque pour le coeur du systeme :
- `catalog.rs` — Catalog::create/open, create(entity, data), relate(), drain()
- `pipeline.rs` — 4 phases (prepare → embed → store → link)
- `refs.rs` — EntityRef, RelationRef (resolution lazy des UUIDs)
- `queue.rs` — Queue configurable (drain explicite + auto-flush)
- `persistence.rs` — Tables systeme (_catalog_meta, _catalog_queue)

Tout ca utilise `DbConnection` (trait async du squelette) + les modules L1/L2 qu'on vient de creer.
