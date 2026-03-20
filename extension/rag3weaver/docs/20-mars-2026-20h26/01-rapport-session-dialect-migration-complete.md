# Doc 01 — Rapport session : migration dialect complète (record_nodes.rs 100%)

Date : 20 mars 2026

## Résumé

Migration complète de `record_nodes.rs` — 63 patterns Cypher inline → 0. Tous passent par `dialect.method()`. Zéro concession sur le batching, les perfs, ou le comportement. 588 tests lib passent.

## Commits

| Commit | Description |
|--------|-------------|
| `795b64f` | feat: add embed + KB dialect methods (7 new methods, both backends) |
| `58ff008` | feat: wire dialect into EmbedNode + KBEmbedNode |
| `534bbe7` | feat: wire dialect into DeleteRecordNode (execute + undo) |
| `728f591` | feat: wire dialect into UpdateRecordNode (execute + undo + join_select) |
| `078966107` | feat: wire dialect into KB nodes (Gather + Update) — 63 → 3 |
| `d5b56ff` | feat: complete dialect migration — 0 Cypher remaining |
| + commits antérieurs pour InsertRecordNode, LinkRecordNode, dialect methods |

## SchemaDialect — état final

**30 méthodes**, 45 tests, couvre :

### Méthodes génériques (CRUD)
| Méthode | Rôle |
|---------|------|
| `create_table` | DDL table avec PK _uuid (+ _row_id BIGSERIAL en pg) |
| `create_rel_table` | DDL relation (join table) |
| `alter_add_column` | Migration additive |
| `create_vector_index` | HNSW / pgvector |
| `create_meta_table` | Table interne _catalog_meta |
| `create_blob_table` | Table interne _index_blobs |
| `batch_upsert` | INSERT batch avec RETURN node ID |
| `batch_delete` | DELETE par UUID |
| `batch_cascade_delete` | DETACH DELETE / CASCADE |
| `batch_link` | INSERT relation (from_uuid, to_uuid) |
| `batch_update_fields` | UPDATE champs par UUID |
| `batch_update_returning` | UPDATE + RETURN expressions |
| `batch_cascade_delete_returning_count` | DELETE children + count |
| `batch_delete_by_field` | DELETE WHERE field = ANY |
| `batch_delete_relation` | DELETE relation par (from, to) |
| `batch_select` | SELECT batch par champ arbitraire |
| `batch_set_null` | SET field = NULL par UUID |
| `select_by_uuids` | SELECT champs par UUID |
| `select_entity_all_by_uuids` | SELECT * par UUID |
| `join_select` | JOIN via relation table (forward/reverse) |
| `join_delete_returning_count` | DELETE via traversal + count |
| `upsert_meta` | Upsert _catalog_meta |
| `load_meta_by_prefix` | Load meta par prefix |
| `count_rows` | COUNT |
| `node_offset_expr` | OFFSET(id(n)) / _row_id |
| `node_id_expr` | ID(n) / _row_id |

### Méthodes Embed
| Méthode | Rôle |
|---------|------|
| `embed_check_hashes` | Check _embed_hash existants (skip unchanged) |
| `embed_set` | SET embedding + _embed_hash |
| `embed_set_hash_returning_offset` | SET hash + RETURN offset (sparse) |
| `embed_get_offset` | RETURN offset sans SET |

### Méthodes KB
| Méthode | Rôle |
|---------|------|
| `kb_gather_fields` | Read title entity fields + _source_uuid |
| `kb_gather_content` | Traversal relation → content fields + _source_uuid |
| `kb_upsert_index` | MERGE ON CREATE / ON MATCH |

### Infrastructure
| Méthode | Rôle |
|---------|------|
| `name()` | Backend name |
| `internal_schema()` | Schema namespace (rag3weaver.* en pg) |
| `internal_table()` | Qualify table name |
| `setup_statements()` | CREATE EXTENSION, CREATE SCHEMA |

## Nodes migrés

| Node | execute | undo | Status |
|------|---------|------|--------|
| InsertRecordNode | ✅ | ✅ | Done |
| LinkRecordNode | ✅ | ✅ | Done |
| KBEmbedNode | ✅ | ✅ | Done |
| EmbedNode | ✅ | ✅ | Done |
| KBGatherNode | ✅ | n/a | Done |
| KBUpdateNode | ✅ | ✅ | Done |
| KBChunkNode | n/a (CPU only) | n/a | No DB ops |
| ChunkRecordNode | n/a (CPU only) | n/a | No DB ops |
| DeleteRecordNode | ✅ | ✅ | Done |
| UpdateRecordNode | ✅ | ✅ | Done |
| RechunkDeleteNode | ✅ | n/a | Done |
| FlushNode | ✅ (FTS extension) | ✅ | rag3db-only, conditioned |
| SparseCommitNode | ✅ (Rust handle) | ✅ | Backend-agnostic |

## Ce qui reste

### Phase 3 — SearchBackend (search.rs)

40 patterns Cypher dans search.rs. Fondamentalement différents entre backends :

| Opération | rag3db | PostgreSQL |
|-----------|--------|------------|
| Vector search | `CALL QUERY_VECTOR_INDEX` | `ORDER BY embedding <=> $1` |
| Vector filtré | `PROJECT_GRAPH_CYPHER` + QUERY | WHERE clauses + JOIN |
| BM25 search | Handles Rust lucivy (identique) | Handles Rust lucivy (identique) |
| Sparse search | Handles Rust (identique) | Handles Rust (identique) |
| Chunk resolution | `MATCH WHERE _uuid IN [...]` | `SELECT WHERE _uuid = ANY(...)` |
| Offset resolution | `OFFSET(id(n)) IN [...]` | `_row_id = ANY(...)` |
| Graph projection | `PROJECT_GRAPH_CYPHER` | Pas d'équivalent (CTEs/subqueries) |

**Décision** : trait `SearchBackend` séparé avec implémentations complètes par backend, plutôt que d'étendre le dialect. Le search est trop spécifique pour des méthodes génériques.

### catalog.rs — 2 patterns restants
- `exists()` — `MATCH ... count` → `select_by_uuids` ou `count_rows` avec filtre
- `get_entity()` — `MATCH ... RETURN` → `select_entity_all_by_uuids`

### Aucune concession faite
- Tous les batch patterns conservés (UNWIND pour rag3db, unnest pour pg)
- Même nombre de roundtrips DB
- Params adaptés quand nécessaire (renommage clés Map pour matcher le dialect)
- Aucun changement de comportement observable

## Prochaines étapes

1. **SearchBackend trait** — Phase 3, chantier séparé
2. **catalog.rs** — 2 patterns restants (trivial)
3. **PostgresConnection** — tokio-postgres, param translation `$name` → `$1`
4. **Tests d'intégration PostgreSQL** — Docker pgvector
5. **Documentation API** — comment switcher de backend
