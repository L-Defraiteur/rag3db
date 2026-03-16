# Doc 05 — Progression : migration multi-backend via SchemaDialect

Date : 16 mars 2026

## Ce qui est fait

### SchemaDialect trait (`src/dialect.rs`)
- Trait `SchemaDialect` avec 15 méthodes (types, tables, rels, indexes, meta, DML)
- `Rag3dbDialect` : Cypher DDL/DML (comportement identique à l'existant)
- `PostgresDialect` : SQL DDL/DML avec pgvector + schema namespace `rag3weaver.*`
- `setup_statements()` : `CREATE EXTENSION vector` + `CREATE SCHEMA rag3weaver`
- `internal_table()` : préfixe automatique `rag3weaver.` pour les tables internes
- 18 tests unitaires couvrant les deux dialectes

### Branchement dans le Catalog (`src/catalog.rs`)
- Champ `dialect: Box<dyn SchemaDialect>` avec default `Rag3dbDialect`
- `set_dialect()` pour switcher avant `initialize()`
- `initialize()` exécute `setup_statements()` en étape 0
- `generate_full_schema_with_dialect()` utilisé au lieu de `generate_full_schema()`

### Premiers usages du dialect dans schema.rs
- `generate_meta_table_ddl()` → `dialect.create_meta_table()`
- `generate_vector_index_ddl()` KB → `dialect.create_vector_index()`
- Backward compatible : `generate_full_schema()` délègue à `Rag3dbDialect`

### Docker
- `docker/docker-compose.supabase.yml` : PostgreSQL 17 + pgvector 0.8.2 sur port 5433

## Ce qui reste à migrer dans schema.rs

| Fonction | Migré ? | Vers quel(s) méthode(s) du dialect |
|----------|---------|-------------------------------------|
| `generate_meta_table_ddl()` | ✅ | `dialect.create_meta_table()` |
| `generate_vector_index_ddl()` (KB) | ✅ | `dialect.create_vector_index()` |
| `generate_node_table_ddl()` | ❌ | `dialect.create_table()` |
| `generate_index_table_ddl()` | ❌ | `dialect.create_table()` |
| `generate_index_chunk_table_ddl()` | ❌ | `dialect.create_table()` |
| `generate_simple_chunk_table_ddl()` | ❌ | `dialect.create_table()` |
| `generate_simple_chunk_rel_ddl()` | ❌ | `dialect.create_rel_table()` |
| `generate_index_chunk_rel_ddl()` | ❌ | `dialect.create_rel_table()` |
| `generate_index_rel_ddl()` | ❌ | `dialect.create_rel_table()` |
| `generate_source_rel_ddl()` | ❌ | `dialect.create_rel_table()` |
| `generate_rel_table_ddl()` | ❌ | `dialect.create_rel_table()` |
| `generate_fts_index_ddl()` | ❌ | Reste FTS-spécifique (lucivy), pas SQL |
| `generate_vector_index_ddl()` (simple entity) | ❌ | `dialect.create_vector_index()` |
| `generate_insert_cypher()` | ❌ | `dialect.batch_upsert()` (utilisé dans nodes) |

## Ce qui reste à migrer dans catalog.rs (hors nodes)

| Opération | Migré ? | Vers quel(s) méthode(s) du dialect |
|-----------|---------|-------------------------------------|
| `persist_meta_key()` | ❌ | `dialect.upsert_meta()` |
| `load_entity_configs()` | ❌ | `dialect.load_meta_by_prefix()` |
| `load_kb_configs()` | ❌ | `dialect.load_meta_by_prefix()` |
| `load_relations()` | ❌ | `dialect.load_meta_by_prefix()` |
| `count_entity()` | ❌ | `dialect.count_rows()` |
| `get_entity()` | ❌ | Besoin d'un `dialect.select()` ou similar |
| `_index_blobs` DDL | ❌ | `dialect.create_blob_table()` |
| `migrate_entity()` ALTER TABLE | ❌ | `dialect.alter_add_column()` |

## Ce qui reste à migrer dans les nodes (Phase 2 — BackendOps)

Les nodes (`record_nodes.rs`) construisent du Cypher inline. C'est le gros du travail — ~25 statements à abstraire via un trait `BackendOps`. Pas commencé.

## Prochaines étapes

1. Finir la migration des `generate_*` dans schema.rs → dialect
2. Migrer `persist_meta_key` / `load_*_configs` dans catalog.rs → dialect
3. Migrer `_index_blobs` DDL → dialect
4. Ensuite : trait `BackendOps` pour les nodes (Phase 2 du doc 04)
