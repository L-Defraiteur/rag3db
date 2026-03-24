# Doc 02 — Plan : phases en cours et prochaines étapes

Date : 24 mars 2026

## Ce qui est FAIT

### Phase 1 — SchemaDialect ✅
- 32 méthodes, 45 tests
- schema.rs 100% migré
- catalog.rs 99% migré (1 filter resolution reste)

### Phase 2 — BackendOps (nodes) ✅
- record_nodes.rs : 63 → 0 Cypher inline
- Tous les 14 nodes passent par le dialect
- Aucune concession sur le batching (UNWIND → unnest, même nombre de roundtrips)

### Phase B — SearchBackend ✅
- B-1 : Trait + types structurés (VectorHit, OffsetResult, EntityRow, ChunkMeta, ChunkWithParent)
- B-2 : Rag3dbSearchBackend impl + refactoring search.rs (versions _via_backend)
- B-3 : PostgresSearchBackend impl (pgvector <=>)
- Catalog::search() utilise search_backend pour vector + sparse + enrichment

### Infrastructure PostgreSQL ✅
- PostgresConnection (tokio-postgres + deadpool, param translation $name → $1)
- PostgresBlobStore (BYTEA dans rag3weaver._index_blobs)
- PostgresDialect (schema namespace rag3weaver.*, _row_id BIGSERIAL, pgvector)
- Docker compose (pgvector 0.8.2 sur port 5433)

## Ce qui reste

### Phase B-4 — Tests d'intégration PostgreSQL
- Écrire des tests qui créent des tables via PostgresDialect
- Tester le CRUD complet (insert, update, delete, link)
- Tester vector search via pgvector
- Tester BlobStore (save/load/delete)
- Même suite que les E2E rag3db, backend swappable

### Phase C — Search DAG (via luciole)
Prérequis : migrer le dataflow rag3weaver vers luciole (Phase 0 ci-dessous)

**Phase 0 — Migrer vers luciole**
1. Rendre les nodes sync (DbConnection est sync sous le capot)
2. Remplacer DataflowGraph par luciole::Dag
3. Remplacer DataflowRuntime par luciole::execute_dag
4. Adapter les 14 nodes au Node trait luciole (ServiceRegistry, undo, etc.)
5. CypherCheckpointStore implémente luciole::CheckpointStore

**Phase C-1 — Search nodes**
- VectorSearchNode { backend: Arc<dyn SearchBackend> }
- BM25SearchNode { handles: lucivy }
- SparseSearchNode { handles: sparse }
- FuseNode { strategy: RRF }
- ChunkResolveNode { backend }
- EnrichNode { backend }

**Phase C-2 — Search DAG builder**
- Catalog::search() construit un DAG selon les signals actifs
- execute_dag() avec parallélisme vector+BM25+sparse (3x potentiel)

**Phase C-3 — Features avancées**
- Timeout par signal (dégradation gracieuse)
- Search streaming (résultats partiels)
- Re-ranking nodes (CrossEncoder)
- Multi-KB parallèle
- Observabilité par étape (vector 12ms, BM25 45ms, fusion 2ms)

### Autres chantiers (parallèles)

- **Migration FTS → Rust** : LucivyHandle direct depuis rag3weaver (doc `15-mars/02-plan-migration-fts-vers-rust.md`)
- **Sparse segments** : segments WORM + incremental sync (doc `24-mars-20h35/07-design-sparse-segments-incremental-sync.md`)
- **Colonnes orphelines** : cleanup sparse_indices/sparse_weights sur DBs existantes
- **Embedding dim detection** : erreur si config change de dim sans rebuild

## Décisions de design prises

| Décision | Choix | Doc |
|----------|-------|-----|
| Relations en PostgreSQL | Tables intermédiaires (join tables) partout | `15-mars/04-audit` |
| Schema namespace | `rag3weaver.*` pour tables internes | `15-mars/04-audit` |
| Param syntax | `$name` partout, traduction dans PostgresConnection | `15-mars/04-audit` |
| Row offset | OFFSET(id(n)) rag3db / _row_id BIGSERIAL pg | `20-mars/01-rapport` |
| FTS rebuild | Conditionné à `dialect.name() == "rag3db"` | `15-mars/05-progression` |
| Search abstraction | B (SearchBackend) d'abord, C (Search DAG) au-dessus | `20-mars/02-design` |
| Convergence dataflow | Utiliser luciole (pas maintenir 2 DAG engines) | `24-mars-20h35/03-convergence` |
