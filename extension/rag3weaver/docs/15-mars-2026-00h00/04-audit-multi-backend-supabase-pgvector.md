# Doc 04 — Audit multi-backend : vers Supabase/pgvector

Date : 15 mars 2026

## Objectif

Rendre rag3weaver backend-agnostic pour supporter Supabase/pgvector en plus de rag3db. Commencer par identifier tout ce qui est rag3db-spécifique, puis abstraire couche par couche.

## Inventaire du code rag3db-spécifique

### Vue d'ensemble

```
                    ┌────────────────────────┐
                    │  Code rag3db-spécifique │
                    └───────────┬────────────┘
                                │
         ┌──────────────────────┼──────────────────────┐
         │                      │                      │
    ┌────┴─────┐          ┌─────┴──────┐         ┌────┴──────┐
    │  Schema  │          │  Catalog   │         │  Nodes    │
    │  (DDL)   │          │  (DML/Read)│         │  (DML)    │
    │  ~25 stmts│         │  ~20 stmts │         │  ~25 stmts│
    └──────────┘          └────────────┘         └───────────┘
```

**~70 statements Cypher/DDL** répartis dans 7 fichiers.

### 1. Schema — DDL (schema.rs)

Ce qui doit changer pour PostgreSQL :

| Cypher (rag3db) | SQL (PostgreSQL) |
|------------------|-----------------|
| `CREATE NODE TABLE {T}(...)` | `CREATE TABLE {t}(...)` |
| `CREATE REL TABLE {R}(FROM {A} TO {B}, ...)` | `CREATE TABLE {r}(from_uuid UUID REFERENCES {a}, to_uuid UUID REFERENCES {b}, ...)` |
| `PRIMARY KEY(_uuid)` | `_uuid UUID PRIMARY KEY DEFAULT gen_random_uuid()` |
| `STRING` | `TEXT` |
| `INT64` | `BIGINT` |
| `DOUBLE` | `DOUBLE PRECISION` |
| `BOOL` | `BOOLEAN` |
| `BLOB` | `BYTEA` |
| `FLOAT[{N}]` (fixed-size array) | `vector({N})` (pgvector) |
| `CALL CREATE_VECTOR_INDEX(...)` | `CREATE INDEX ... USING hnsw (... vector_cosine_ops)` |
| `CALL CREATE_LUCIVY_INDEX(...)` | *(géré par lucivy Rust, pas pg_trgm)* |

**Abstraction proposée** : trait `SchemaDialect`

```rust
trait SchemaDialect {
    fn create_table(&self, name: &str, columns: &[ColumnDef]) -> String;
    fn create_rel_table(&self, name: &str, from: &str, to: &str, props: &[ColumnDef]) -> String;
    fn alter_add_column(&self, table: &str, col: &ColumnDef) -> String;
    fn create_vector_index(&self, table: &str, column: &str, name: &str) -> String;
    fn type_name(&self, ft: &FieldType) -> String;
    fn default_value(&self, ft: &FieldType) -> String;
}
```

### 2. Catalog — Métadonnées & lifecycle (catalog.rs)

| Opération | Cypher | SQL equiv | Abstraction |
|-----------|--------|-----------|-------------|
| Upsert meta | `MERGE (m:_catalog_meta {_key: $k}) SET m._value = $v` | `INSERT ... ON CONFLICT DO UPDATE` | `backend.upsert_meta(key, value)` |
| Load meta | `MATCH (m:_catalog_meta) WHERE m._key STARTS WITH $prefix RETURN ...` | `SELECT ... WHERE _key LIKE $prefix%` | `backend.load_meta(prefix)` |
| Count | `MATCH (n:{T}) RETURN count(n)` | `SELECT count(*) FROM {t}` | `backend.count(table)` |
| Get entity | `MATCH (n:{T}) RETURN {fields}` | `SELECT {fields} FROM {t}` | `backend.get(table, filters, fields)` |
| Close FTS | `CALL CLOSE_LUCIVY_INDEX(...)` | *(lucivy handle.close())* | Géré directement par les handles Rust |
| Blob ops | Via CypherBlobStore (MERGE/MATCH sur _index_blobs) | `INSERT/SELECT/DELETE FROM _index_blobs` | Trait `BlobStore` (déjà abstrait!) |

### 3. Nodes dataflow — DML (record_nodes.rs)

| Node | Cypher pattern | SQL equiv | Volume |
|------|---------------|-----------|--------|
| **InsertRecordNode** | `UNWIND $items AS item MERGE (n:{T} {_uuid: item._uuid}) SET ...` | Batch `INSERT ... ON CONFLICT` | Hot path |
| **ChunkRecordNode** | `UNWIND ... MERGE (c:{T}_Chunk {_uuid: ...}) SET ...` | Batch `INSERT` chunks | Hot path |
| **EmbedNode** | `UNWIND ... MATCH (n:{T} {_uuid: ...}) SET n.embedding = ...` | Batch `UPDATE ... SET embedding` | Hot path |
| **LinkRecordNode** | `UNWIND ... MATCH (a), (b) MERGE (a)-[:{R}]->(b)` | `INSERT INTO {rel_table}` | Medium |
| **DeleteRecordNode** | `MATCH (n:{T} {_uuid: ...}) DETACH DELETE n` | `DELETE FROM {t} WHERE _uuid = ANY(...)` + cascades | Low |
| **UpdateRecordNode** | `MERGE ... SET ...` | `UPDATE ... SET ... WHERE _uuid = ANY(...)` | Medium |
| **FlushNode** | `CALL FLUSH_LUCIVY_INDEX(...)` | handle.commit() direct | Control |
| **KBGatherNode** | `MATCH (n)-[:REL]->(idx) RETURN ...` | JOINs multi-tables | Medium |
| **KBUpdateNode** | `MATCH + SET` sur KB_Index | `UPDATE` sur KB_Index | Medium |

### 4. Search — Queries (search.rs)

| Opération | Cypher | SQL equiv |
|-----------|--------|-----------|
| Vector search | `CALL QUERY_VECTOR_INDEX('{T}', '{idx}', $emb, {limit})` | `SELECT *, embedding <=> $emb AS dist FROM {t} ORDER BY dist LIMIT {limit}` |
| BM25 search | `CALL QUERY_LUCIVY_INDEX(...)` | *(lucivy handle.search() — pas pg_trgm)* |
| Sparse search | `handle.search()` + `MATCH WHERE OFFSET(id(n)) IN [...]` | `handle.search()` + `SELECT ... WHERE id = ANY(...)` |
| Chunk resolution | `MATCH (c:{Chunk})-[:REL]->(n:{Entity})` | `SELECT ... FROM chunk JOIN entity ON ...` |
| Graph projection | `CALL PROJECT_GRAPH_CYPHER(...)` | CTEs ou subqueries |

### 5. Checkpoint store (checkpoint_store.rs)

~15 statements MERGE/MATCH sur `_DataflowExecution` et `_DataflowNodeState`. Transposition directe en INSERT/SELECT/UPDATE/DELETE SQL.

## Stratégie d'abstraction

### Couche 1 : `BackendDialect` (schema + types)

```
schema.rs  →  trait SchemaDialect
                ├── Rag3dbDialect (Cypher DDL)
                └── PostgresDialect (SQL DDL + pgvector)
```

Impacte uniquement la génération DDL. **Pas de changement dans les nodes.** C'est la couche la plus simple.

### Couche 2 : `BackendOps` (opérations CRUD)

```
trait BackendOps {
    // DML
    async fn batch_upsert(&self, table: &str, records: &[Record]) -> Result<Vec<Uuid>>;
    async fn batch_delete(&self, table: &str, uuids: &[&str]) -> Result<()>;
    async fn batch_update(&self, table: &str, updates: &[FieldUpdate]) -> Result<()>;

    // Relations
    async fn batch_link(&self, rel: &str, pairs: &[(Uuid, Uuid)]) -> Result<()>;

    // Read
    async fn get(&self, table: &str, uuids: &[&str], fields: &[&str]) -> Result<Vec<Row>>;
    async fn count(&self, table: &str) -> Result<usize>;
    async fn query_by_offsets(&self, table: &str, offsets: &[u64], fields: &[&str]) -> Result<Vec<Row>>;

    // Meta
    async fn upsert_meta(&self, key: &str, value: &str) -> Result<()>;
    async fn load_meta(&self, prefix: &str) -> Result<Vec<(String, String)>>;
}
```

Chaque node utiliserait `BackendOps` au lieu de construire du Cypher. Ça impacte tous les nodes mais c'est mécanique.

### Couche 3 : `SearchBackend` (recherche)

```
trait SearchBackend {
    async fn vector_search(&self, table: &str, embedding: &[f32], limit: usize) -> Result<Vec<(Uuid, f64)>>;
    async fn vector_search_filtered(&self, table: &str, embedding: &[f32], filter: &Filter, limit: usize) -> Result<Vec<(Uuid, f64)>>;
}
```

**Note** : FTS (lucivy) et Sparse restent via les handles Rust — ils ne dépendent pas du backend SQL. Seul le vector search change (HNSW rag3db vs pgvector).

### Couche 4 : `CheckpointBackend`

Trait `CheckpointStore` existe déjà — il suffit d'implémenter un `PostgresCheckpointStore`.

## Ce qui NE change PAS

| Composant | Raison |
|-----------|--------|
| **lucivy FTS** | Handles Rust directs, indépendant du backend SQL |
| **Sparse index** | SparseHandle + BlobStore, indépendant du backend SQL |
| **BlobStore** | Trait déjà abstrait (CypherBlobStore vs MemBlobStore → + PostgresBlobStore) |
| **Embedder** | Trait déjà abstrait |
| **Dataflow engine** | Le runtime, le checkpoint, l'undo — tout est backend-agnostic |
| **Config** | CatalogConfig, EntityDef, KBConfig — purement déclaratif |
| **Events** | EventBus — purement in-process |

## Plan d'exécution

```
Phase 1 : SchemaDialect (DDL)              ← Simple, pas de refactor nodes
  └─ PostgresDialect génère du SQL DDL
  └─ Tests : créer les tables dans un PostgreSQL Docker

Phase 2 : BackendOps (CRUD)                ← Le gros du travail
  └─ Extraire le Cypher des nodes en opérations abstraites
  └─ CypherBackendOps (rag3db) — même comportement qu'aujourd'hui
  └─ PostgresBackendOps (Supabase) — via sqlx ou tokio-postgres
  └─ Adapter les nodes pour utiliser BackendOps

Phase 3 : SearchBackend (vector)           ← Modéré
  └─ pgvector <=> similarity search
  └─ Les filtres graph deviennent des WHERE/JOIN SQL

Phase 4 : PostgresBlobStore + CheckpointStore  ← Simple
  └─ Implémentation BYTEA pour les blobs
  └─ INSERT/UPDATE/DELETE pour les checkpoints

Phase 5 : Docker Compose + tests E2E       ← Validation
  └─ Supabase local (PostgreSQL 15 + pgvector)
  └─ Même suite de tests E2E, backend swappable
```

## Docker Compose (cible)

```yaml
services:
  supabase-db:
    image: supabase/postgres:15.1.0.147
    environment:
      POSTGRES_PASSWORD: postgres
    ports:
      - "5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql/data
    command: >
      postgres -c shared_preload_libraries='vector'

volumes:
  pgdata:
```

## Décision : relations graph en PostgreSQL

### Approches évaluées

| Approche | Description | Verdict |
|----------|------------|---------|
| **FK directe** | `chunk._parent_uuid REFERENCES entity._uuid` | Simple pour 1-to-many, mais 2 chemins dans le code |
| **Table intermédiaire** | `entity_chunked_from(from_uuid, to_uuid)` | Uniforme, même pattern pour toutes les relations |
| **Apache AGE** | Extension Cypher dans PostgreSQL | Pas dispo sur Supabase managed, extension à installer |
| **SQL/PGQ** | Standard ISO SQL:2023 | Pas production-ready (prototype PostgreSQL 18) |

### Choix : tout en tables intermédiaires

**Toutes les relations = tables intermédiaires**, même les 1-to-many (chunks → parent).

Raisons :
1. **Uniformité** : un seul pattern dans `BackendOps::batch_link()`, pas de cas spéciaux FK vs table
2. **Relations custom** : `register_relation("AUTHORED_BY", "Doc", "Author", {date: Date})` — même mécanique que les relations internes
3. **Search nodes custom** : traversent n'importe quelle relation de la même façon
4. **Multi-backend** : un `REL TABLE` rag3db = une table intermédiaire PostgreSQL = même abstraction

Le coût (un JOIN en plus sur les 1-to-many) est négligeable face aux vector search + FTS + fusion.

### Schema PostgreSQL des relations

```sql
-- Relations internes (chunk → parent)
CREATE TABLE document_chunked_from (
    from_uuid UUID NOT NULL,
    to_uuid UUID NOT NULL,
    PRIMARY KEY (from_uuid, to_uuid)
);
CREATE INDEX ON document_chunked_from(to_uuid);  -- pour les lookups parent → chunks

-- Relations KB
CREATE TABLE document_in_main (
    from_uuid UUID NOT NULL,
    to_uuid UUID NOT NULL,
    PRIMARY KEY (from_uuid, to_uuid)
);

-- Relations custom (avec propriétés)
CREATE TABLE authored_by (
    from_uuid UUID NOT NULL,
    to_uuid UUID NOT NULL,
    date DATE,
    PRIMARY KEY (from_uuid, to_uuid)
);
```

Nos traversals sont simples (1-2 hops max), pas besoin de recursive CTEs ni d'extensions graph.

## Questions ouvertes restantes

1. **OFFSET(id(n))** : rag3db a des offsets stables par node. En PostgreSQL il faudrait des IDs séquentiels (`BIGSERIAL`) ou un mapping offset→uuid. Impact sur sparse index.
2. **Transactions** : rag3db n'a pas de transactions multi-statements classiques. PostgreSQL oui → opportunité pour des batch plus atomiques.
3. **UNWIND** : pattern très utilisé pour le batch. En PostgreSQL → `unnest()` ou des `INSERT ... SELECT FROM unnest(...)`.
4. **Supabase RLS** : Row Level Security peut ajouter de la latence sur les queries. À mesurer.

## Références

- [Apache AGE — Graph extension for PostgreSQL](https://age.apache.org/overview/)
- [SQL/PGQ in PostgreSQL (EDB)](https://www.enterprisedb.com/blog/representing-graphs-postgresql-sqlpgq)
- [Supabase pgvector](https://supabase.com/docs/guides/database/extensions/pgvector)
- [Supabase Recursive CTEs for hierarchical data](https://dev.to/roel_peters_8b77a70a08fdb/beyond-flat-tables-model-hierarchical-data-in-supabase-with-recursive-queries-4ndl)
- [PostgreSQL as Graph Database](https://www.puppygraph.com/blog/postgresql-graph-database)
