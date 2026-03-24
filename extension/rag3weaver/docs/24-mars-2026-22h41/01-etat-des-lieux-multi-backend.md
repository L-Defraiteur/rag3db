# Doc 01 — État des lieux : multi-backend complet

Date : 24 mars 2026

## Résumé

Migration multi-backend rag3db ↔ PostgreSQL/pgvector terminée. Tout le Cypher inline a été remplacé par des appels au `SchemaDialect` trait (DDL/DML) et au `SearchBackend` trait (search). Les implémentations PostgreSQL sont en place.

## Composants par backend

| Composant | rag3db | PostgreSQL | Feature flag |
|-----------|--------|------------|--------------|
| `DbConnection` | `Rag3dbConnection` | `PostgresConnection` | `rag3db-native` / `postgres` |
| `SchemaDialect` | `Rag3dbDialect` (32 méthodes) | `PostgresDialect` (32 méthodes) | — (toujours dispo) |
| `SearchBackend` | `Rag3dbSearchBackend` | `PostgresSearchBackend` | — |
| `BlobStore` | `CypherBlobStore` | `PostgresBlobStore` | — / `postgres` |
| Docker | — | `docker/docker-compose.supabase.yml` (pgvector 0.8.2, port 5433) | — |

## Fichiers Cypher inline restants

| Fichier | Restant | Contexte |
|---------|---------|----------|
| `record_nodes.rs` | **0** | 100% migré vers dialect |
| `catalog.rs` | **1** | Filter resolution search (MATCH dynamique, Phase C) |
| `schema.rs` | **0** | 100% migré vers dialect |
| `search.rs` | ~15 | Versions legacy gardées, versions `_via_backend` disponibles |

## Dialect — 32 méthodes

### Générique (CRUD)
`create_table`, `create_rel_table`, `alter_add_column`, `create_vector_index`, `create_meta_table`, `create_blob_table`, `batch_upsert`, `batch_delete`, `batch_cascade_delete`, `batch_link`, `batch_update_fields`, `batch_update_returning`, `batch_cascade_delete_returning_count`, `batch_delete_by_field`, `batch_delete_relation`, `batch_select`, `batch_set_null`, `select_by_uuids`, `select_entity_all_by_uuids`, `select_all`, `exists_by_uuid`, `join_select`, `join_delete_returning_count`, `upsert_meta`, `load_meta_by_prefix`, `count_rows`

### Row identity
`node_offset_expr` (OFFSET(id(n)) / _row_id), `node_id_expr` (ID(n) / _row_id)

### Embed
`embed_check_hashes`, `embed_set`, `embed_set_hash_returning_offset`, `embed_get_offset`

### KB
`kb_gather_fields`, `kb_gather_content`, `kb_upsert_index`

### Search
`resolve_chunks_with_parent`

### Infra
`name()`, `internal_schema()`, `internal_table()`, `setup_statements()`

## SearchBackend — 6 méthodes

`vector_search`, `vector_search_filtered`, `resolve_offsets`, `fetch_entities`, `fetch_chunks`, `fetch_with_chunks`

## Catalog intégration

- `dialect: Arc<dyn SchemaDialect>` — default Rag3dbDialect, set via `set_dialect()`
- `search_backend: Option<Arc<dyn SearchBackend>>` — auto-crée Rag3dbSearchBackend dans `initialize()`
- Services registered : `dialect` + `search_backend` disponibles dans les nodes

## Tests

- 591 lib tests passent (default features)
- 45 dialect tests
- Pas encore de tests d'intégration PostgreSQL (Docker up, pas encore testé)
- E2E rag3db : 36/38 (2 fails BM25 pré-existants — champs ._ngram pas alimentés par extension C++)
