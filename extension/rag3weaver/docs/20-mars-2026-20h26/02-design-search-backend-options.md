# Doc 02 — Design : abstraction search pour multi-backend

Date : 20 mars 2026

## Le problème

`search.rs` contient 40 patterns Cypher inline. Contrairement à `record_nodes.rs` (CRUD) où les opérations sont similaires entre backends (INSERT/MERGE, UPDATE/SET, DELETE), le search est **fondamentalement différent** :

| Opération | rag3db | PostgreSQL |
|-----------|--------|------------|
| Vector HNSW | `CALL QUERY_VECTOR_INDEX(table, index, $emb, limit)` | `SELECT *, embedding <=> $1 AS dist FROM table ORDER BY dist LIMIT $2` |
| Vector filtré | `PROJECT_GRAPH_CYPHER(graph, filter_query)` → HNSW sur graph projeté | `WHERE filter_condition ORDER BY embedding <=> $1` (WHERE + ORDER BY combinés) |
| Offset → UUID | `MATCH (n) WHERE OFFSET(id(n)) IN [...]` | `SELECT * FROM table WHERE _row_id = ANY(...)` |
| Chunk résolution | `MATCH (c) WHERE c._uuid IN [...]` + champs inline | `SELECT ... FROM chunk WHERE _uuid = ANY(...)` |
| Entity enrichment | `MATCH (n) WHERE n._uuid IN [...] RETURN n.f1, n.f2` | `SELECT f1, f2 FROM entity WHERE _uuid = ANY(...)` |
| Chunk + parent join | `MATCH (n) WHERE OFFSET(id(n)) IN [...] OPTIONAL MATCH (n)<-[:REL]-(c)` | `SELECT ... FROM entity JOIN rel ON ... JOIN chunk ON ...` |
| Graph projection | `PROJECT_GRAPH_CYPHER` / `DROP_PROJECTED_GRAPH` | N'existe pas — CTEs ou subqueries |
| BM25 search | Handles Rust lucivy (identique) | Handles Rust lucivy (identique) |
| Sparse search | Handles Rust SparseHandle (identique) | Handles Rust SparseHandle (identique) |

**Bonne nouvelle** : BM25 et sparse sont déjà backend-agnostic (handles Rust directs). Le problème c'est le vector search, la résolution d'offsets, et l'enrichment.

## Analyse du code actuel

### Fonctions publiques (API haute niveau)
```
search_vector()           → dispatch vers hnsw ou hnsw_filtered
resolve_chunk_results()   → chunk UUID → metadata
enrich_results_with_data() → entity UUID → data fields
resolve_and_enrich()      → offset → UUID + data (composé)
resolve_and_enrich_chunked() → offset → UUID + data + chunks (composé)
resolve_vector_chunks()   → chunk UUID → parent + chunks
search_bm25()             → BM25 via lucivy (déjà abstrait)
search_bm25_chunked()     → BM25 + chunk resolution
search_sparse()           → sparse via handle (déjà abstrait)
fuse_results()            → fusion RRF/Weighted (pur calcul, pas de DB)
explore_bfs()             → graph traversal (rag3db-specific)
```

### Ce qui est déjà backend-agnostic
- `search_bm25*` — appelle `QUERY_LUCIVY_INDEX` mais c'est géré par les handles Rust
- `search_sparse` — `SparseHandle::search()` + `resolve_and_enrich()`
- `fuse_results` — pur calcul, zéro DB
- `embed_query` — appelle l'Embedder trait (déjà abstrait)

### Ce qui doit changer
- `search_vector_hnsw` — CALL extension rag3db
- `search_vector_hnsw_filtered` — graph projection (rag3db-only)
- `resolve_and_enrich` — OFFSET(id(n)) inline
- `resolve_and_enrich_chunked` — OFFSET(id(n)) + OPTIONAL MATCH
- `resolve_chunk_results` — UUID inline
- `enrich_results_with_data` — UUID inline
- `resolve_vector_chunks` — UUID inline
- `explore_bfs` / `explore_relation_batch` — graph traversal

## Option A : Étendre le SchemaDialect

Ajouter des méthodes search au dialect existant :

```rust
trait SchemaDialect {
    // ... 30 méthodes existantes ...

    // Search
    fn vector_search(&self, table: &str, index: &str, limit: usize) -> String;
    fn vector_search_filtered(&self, table: &str, index: &str, filter: &str, limit: usize) -> String;
    fn select_by_offsets(&self, table: &str, return_fields: &[&str]) -> String;
    fn select_chunks_by_uuids(&self, chunk_table: &str, fields: &[&str]) -> String;
    fn select_with_chunk_join(&self, entity: &str, chunk_table: &str, rel: &str, return_fields: &[&str]) -> String;
}
```

**Avantages** :
- Un seul trait, cohérent avec ce qu'on a fait
- Les fonctions search appellent `dialect.method()` comme les nodes

**Inconvénients** :
- Le trait grossit (35+ méthodes)
- Le vector filtré est fondamentalement différent (graph projection vs WHERE) — la signature ne peut pas être la même
- Pas de graph projection en PostgreSQL — la méthode serait un no-op ou un fallback brute-force
- Les UUID inline (`IN [uuid1, uuid2, ...]`) devraient passer à des params (`ANY($uuids)`) — change le format d'appel

**Verdict** : possible pour les opérations simples (offset → uuid, enrichment), pas pour le vector filtré.

## Option B : Trait SearchBackend séparé

Un trait dédié au search, implémenté par chaque backend :

```rust
#[async_trait]
trait SearchBackend: Send + Sync {
    /// Vector similarity search (top-K).
    async fn vector_search(
        &self,
        table: &str,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(String, f64)>, String>;  // (uuid, score)

    /// Vector search with filter conditions.
    async fn vector_search_filtered(
        &self,
        table: &str,
        embedding: &[f32],
        limit: usize,
        filter_match: Option<&str>,
        filter_where: Option<&str>,
    ) -> Result<Vec<(String, f64)>, String>;

    /// Resolve node offsets → UUIDs + optional entity data.
    async fn resolve_offsets(
        &self,
        table: &str,
        offsets: &[u64],
        return_fields: &[&str],
    ) -> Result<Vec<OffsetResult>, String>;

    /// Batch fetch entity data by UUIDs.
    async fn fetch_entities(
        &self,
        table: &str,
        uuids: &[&str],
        fields: &[&str],
    ) -> Result<Vec<EntityRow>, String>;

    /// Batch fetch chunk metadata by UUIDs.
    async fn fetch_chunks(
        &self,
        chunk_table: &str,
        uuids: &[&str],
    ) -> Result<Vec<ChunkMeta>, String>;

    /// Fetch chunks with their parent entity data (join).
    async fn fetch_chunks_with_parents(
        &self,
        entity: &str,
        chunk_table: &str,
        rel_table: &str,
        offsets: &[u64],
        entity_fields: &[&str],
    ) -> Result<Vec<ChunkWithParent>, String>;
}
```

Implémentations :
- `Rag3dbSearchBackend` — utilise `QUERY_VECTOR_INDEX`, `PROJECT_GRAPH_CYPHER`, `OFFSET(id(n))`
- `PostgresSearchBackend` — utilise `ORDER BY embedding <=>`, `WHERE`, `_row_id`

**Avantages** :
- Chaque backend implémente sa logique optimale sans compromis
- Le graph projection rag3db reste intact, PostgreSQL fait des WHERE+ORDER BY
- Les types de retour sont structurés (pas des `QueryResult` à parser)
- Les fonctions search deviennent backend-agnostic — appellent `backend.method()`

**Inconvénients** :
- Deux implémentations complètes à écrire et maintenir
- Le Catalog doit porter un `search_backend: Arc<dyn SearchBackend>` en plus du `dialect`
- Plus de code total

## Option C : Hybrid — Dialect étendu + nodes spécialisés

Utiliser le dialect pour les opérations génériques (fetch entities, fetch chunks) et des **search nodes** spécialisés pour le vector search.

Le dataflow de search utiliserait des nœuds interchangeables :

```
SearchGraph (dataflow DAG):

  EmbedQueryNode ──▸ VectorSearchNode ──▸ FuseNode
                     BM25SearchNode    ──┘
                     SparseSearchNode  ──┘
                                          │
                                          ▼
                                     ChunkResolutionNode
                                          │
                                          ▼
                                     EnrichmentNode
```

Les nodes `VectorSearchNode`, `ChunkResolutionNode`, et `EnrichmentNode` auraient des factory functions par backend :

```rust
// Node factories
fn rag3db_vector_search_node() -> Box<dyn Node>;
fn postgres_vector_search_node() -> Box<dyn Node>;

// Le Catalog choisit la factory selon le backend
let vector_node = match dialect.name() {
    "rag3db" => rag3db_vector_search_node(),
    "postgresql" => postgres_vector_search_node(),
    _ => panic!("unsupported"),
};
```

**Avantages** :
- Réutilise le dataflow existant (même pattern que les ingestion nodes)
- Les nodes BM25 et sparse sont identiques pour les deux backends
- Le vector search est le seul node qui change vraiment
- L'enrichment et la chunk resolution peuvent utiliser le dialect (comme record_nodes)

**Inconvénients** :
- Le search n'est pas un DAG aujourd'hui — c'est des appels de fonctions chaînés dans `Catalog::search()`
- Refactorer le search en DAG c'est un chantier en soi
- Over-engineering si on a que 2 backends

## Recommandation : Option B (SearchBackend trait)

**Raison** : c'est le bon niveau d'abstraction. Le search est une opération **complète** (embedding → vector → BM25 → sparse → fusion → resolution → enrichment) qui est fondamentalement différente selon le backend. Le trait SearchBackend encapsule toute cette logique.

L'Option A (étendre le dialect) est trop bas niveau — le vector filtré ne peut pas être abstrait en une seule query string.

L'Option C (search DAG) est over-engineering — on n'a pas besoin d'un DAG pour le search, les appels séquentiels suffisent. Le DAG d'ingestion est justifié par le parallélisme des merges, le checkpoint, l'undo. Le search n'a pas ces besoins.

### Plan d'implémentation

```
Phase 1 : Trait + types de retour structurés
  - Définir SearchBackend + OffsetResult, EntityRow, ChunkMeta, ChunkWithParent
  - Implémenter Rag3dbSearchBackend (copier le code existant de search.rs)

Phase 2 : Refactorer search.rs pour utiliser SearchBackend
  - Les fonctions publiques prennent &dyn SearchBackend au lieu de &dyn DbConnection
  - Le Catalog crée le bon SearchBackend dans initialize()

Phase 3 : Implémenter PostgresSearchBackend
  - Vector search via pgvector <=> operator
  - Offset resolution via _row_id
  - Enrichment via SELECT ... WHERE _uuid = ANY(...)

Phase 4 : Tests d'intégration
  - Mêmes tests, deux backends
```

### Ce qui ne change PAS

| Composant | Raison |
|-----------|--------|
| `search_bm25*` | Handles Rust lucivy — identique sur tous les backends |
| `search_sparse` | Handle Rust SparseHandle — identique |
| `fuse_results` | Pur calcul — pas de DB |
| `embed_query` | Trait Embedder — déjà abstrait |
| `build_bm25_query` | Config JSON pour lucivy — pas de SQL |

### Impact sur l'API publique

```rust
// Avant :
catalog.search("kb", "query", options).await?

// Après : identique ! Le SearchBackend est interne au Catalog.
catalog.search("kb", "query", options).await?
```

L'utilisateur ne voit pas le SearchBackend. C'est un détail d'implémentation du Catalog, comme le dialect.
