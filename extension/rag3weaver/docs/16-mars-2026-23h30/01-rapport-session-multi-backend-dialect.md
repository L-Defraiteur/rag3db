# Doc 01 — Rapport session : SchemaDialect multi-backend + investigation LockBusy

Date : 16 mars 2026

## Résumé

Session en deux parties :
1. Finalisation sparse (MemBlobStore, LockBusy fix, shutdown events)
2. Début du multi-backend Supabase/pgvector via SchemaDialect

## Commits de cette session

| Commit | Description |
|--------|-------------|
| `99fd8d76` | fix: MemBlobStore fallback + sparse_handles service registration |
| `db867275` | fix: move blob_store init before ensure_sparse_handle |
| `6057ac9a` | fix: LockBusy on DB reopen + shutdown events + shared Database sync conn |
| `9bcebed2` | docs: session rapport, architecture, FTS plan, multi-backend audit |
| `77404bfb` | docs: progression multi-backend dialect migration |
| `cfde189d` | feat: SchemaDialect trait (Rag3dbDialect + PostgresDialect, 18 tests) |
| `1f27f9ed` | feat: wire SchemaDialect into Catalog + generate_full_schema |
| `f5beccda` | feat: migrate all generate_* functions to use SchemaDialect |
| `bdf08439` | feat: migrate catalog meta ops and blob DDL to dialect |
| `d1a225a0` | feat: migrate migrate_entity() to dialect + document bypasses |

## Ce qui est fait

### Sparse — finalisé
- MemBlobStore fallback pour tests in-memory
- Ordering fix (blob_store init avant ensure_sparse_handle)
- LockBusy root cause : `CREATE_LUCIVY_INDEX` pas idempotent + `initLucivyEntries` conflit
- Fix : guard idempotent + `~LucivyIndex()` destructeur + shutdown events
- `Rag3dbConnection` : `Box<Database>` → `Arc<Database>` + `create_sync_connection()`
- **36/38 E2E pass** (2 fails BM25 pré-existants — champs `._ngram`/`._raw` pas alimentés par extension C++)

### SchemaDialect — Phase 1 complète
- Trait `SchemaDialect` avec 15+ méthodes (types, tables, rels, indexes, meta, DML)
- `Rag3dbDialect` : Cypher DDL/DML identique à l'existant
- `PostgresDialect` : SQL DDL/DML + pgvector + schema namespace `rag3weaver.*`
- Docker compose PostgreSQL 17 + pgvector 0.8.2

### Migrations vers le dialect — terminées (hors nodes)

**schema.rs** — toutes les fonctions `generate_*` :
- `generate_node_table_ddl` → `dialect.create_table()`
- `generate_index_table_ddl` → `dialect.create_table()`
- `generate_index_chunk_table_ddl` → `dialect.create_table()`
- `generate_simple_chunk_table_ddl` → `dialect.create_table()`
- 5 fonctions rel → `dialect.create_rel_table()`
- `generate_vector_index_ddl` → `dialect.create_vector_index()`
- `generate_meta_table_ddl` → `dialect.create_meta_table()`

**catalog.rs** — opérations meta + lifecycle :
- `persist_meta_key` → `dialect.upsert_meta()`
- `load_entity_configs` / `load_kb_configs` / `load_relations` → `dialect.load_meta_by_prefix()`
- `_index_blobs` DDL → `dialect.create_blob_table()`
- `count()` → `dialect.count_rows()`
- `migrate_entity()` ALTER TABLE → `dialect.alter_add_column()`
- `migrate_entity()` vector index → `dialect.create_vector_index()`

### Décisions de design prises

| Décision | Choix | Raison |
|----------|-------|--------|
| Relations en PostgreSQL | Tables intermédiaires (join tables) pour tout | Uniformité, relations custom, même abstraction multi-backend |
| Schema namespace | `rag3weaver.*` pour tables internes | Isolation propre, pas de collision namespace user |
| Syntaxe params | `$name` partout, traduction `$name` → `$1` dans la future PostgresConnection | Dialect reste simple et testable |
| FTS rebuild en migration | Conditionné à `dialect.name() == "rag3db"` | Sera remplacé par handles Rust (doc 02) |

### Bypasses documentés

| Bypass | Quand régler |
|--------|--------------|
| `DROP/CREATE_LUCIVY_INDEX` dans migrate_entity | Migration FTS → Rust (doc 02) |
| `generate_fts_index_ddl` dans generate_full_schema | Idem |
| `exists()` par UUID (Cypher inline) | Phase 2 (BackendOps) |
| `get_entity()` (Cypher inline) | Phase 2 (BackendOps) |
| Param syntax `$name` → `$1` | PostgresConnection |

## 561 tests lib passent

## Ce qui reste pour le multi-backend complet

### Phase 2 — BackendOps (nodes)
~25 statements Cypher dans `record_nodes.rs`. Trait à designer :
- `batch_upsert(table, records)` — UNWIND MERGE vs INSERT ON CONFLICT
- `batch_delete(table, uuids)` — DETACH DELETE vs DELETE CASCADE
- `batch_link(rel, pairs)` — MATCH+MERGE vs INSERT rel table
- `batch_update(table, updates)` — MERGE SET vs UPDATE SET

### Phase 3 — SearchBackend
~10 queries dans `search.rs` :
- Vector search (QUERY_VECTOR_INDEX vs pgvector `<=>`)
- BM25 search (QUERY_LUCIVY_INDEX — lucivy handles, backend-agnostic)
- Sparse search (handles Rust, déjà backend-agnostic)
- Chunk resolution (MATCH+JOIN vs SELECT+JOIN)
- Enrichment (MATCH vs SELECT)

### Phase 4 — Infrastructure PostgreSQL
- `PostgresConnection` (tokio-postgres, param translation)
- `PostgresBlobStore` (BYTEA dans `rag3weaver._index_blobs`)
- `PostgresCheckpointStore`
- Tests d'intégration sur Docker pgvector

## Docs de référence pour la suite

| Doc | Contenu | Localisation |
|-----|---------|-------------|
| **04-audit-multi-backend** | Inventaire complet du Cypher à migrer + plan 4 phases | `docs/15-mars-2026-00h00/04-audit-multi-backend-supabase-pgvector.md` |
| **05-progression-dialect** | Checklist migration + bypasses | `docs/15-mars-2026-00h00/05-progression-multi-backend-dialect.md` |
| **02-plan-migration-fts** | Plan pour migrer FTS de C++ vers Rust LucivyHandle | `docs/15-mars-2026-00h00/02-plan-migration-fts-vers-rust.md` |
| **03-architecture** | Vue d'ensemble complète rag3weaver | `docs/15-mars-2026-00h00/03-architecture-rag3weaver.md` |
| **08-migrations-destructives** | TODO colonnes orphelines, embedding dim mismatch | `docs/12-mars-2026-15h21/08-todo-migrations-destructives.md` |
