# Doc 04 — Audit code vs doc 03 + réflexion abstraction multi-backend

Date : 12 mars 2026

## 1. Corrections : ce que doc 03 disait "pas fait" mais qui est fait

### 1.1 CypherNode + ValidateNode — FAIT

**Fichier** : `src/dataflow/migration_nodes.rs` (612 lignes, 25 tests unitaires)

**CypherNode** :
- Exécute une query Cypher, capture optionnelle pour undo (capture_query)
- Undo : restaure les valeurs via MATCH SET sur _uuid
- Factory enregistrée dans NodeRegistry
- Config : `query` (required), `capture` (optional)
- Ports : trigger → result (Map) + done (Empty)

**ValidateNode** :
- 6 types d'assertion : `empty`, `not_empty`, `count == N`, `count > N`, `count < N`, `column op value`
- Parse depuis string config, check sur QueryResult
- Factory enregistrée

### 1.2 MigrationRunner — FAIT

**Fichier** : `src/dataflow/migrations.rs` (1204 lignes, 16 tests unitaires)

**API complète** :
- `initialize()` — crée _DataflowMigration + _DataflowMigrationLock
- `status()` — liste toutes les migrations avec état (Pending/Applied/Failed/RolledBack)
- `pending()` — filtre les non-appliquées
- `apply(target_version, dry_run, vars)` — applique via DataflowRuntime + checkpoint
- `rollback(version)` — charge undo contexts du checkpoint, exécute en reverse topological order
- `check_reversible(vars)` — vérifie can_undo() sur tous les nœuds
- Verrouillage avec TTL 10min, lock/unlock automatique
- Dry-run : parse + validate + affiche DryRunPlan (node_count, edge_count, all_reversible)

**Migration interne existante** : `migrations/internal/001_create_dataflow_tables.mmd`
- 4 CypherNode en chaîne : create _DataflowExecution, _DataflowNodeState, _DataflowMigration, _DataflowMigrationLock

### 1.3 Nœuds search génériques — FAIT

**Fichier** : `src/dataflow/generic_search_nodes.rs`

Les 5 nœuds sont **implémentés et enregistrés** dans NodeRegistry :
- `SearchSourceNode` — résout SearchTarget via **Catalog** (service "catalog")
- `VectorSearchNode` — embed query + search_vector + resolve chunks→parents (service "conn")
- `BM25SearchNode` — search_bm25_chunked + highlight→chunk resolution (service "conn")
- `SparseSearchNode` — DualEmbedder/SparseEmbedder + search_sparse_cypher (service "conn")
- `FuseResultsNode` — RRF fusion multi-signal (pas de service DB)
- `ResolveParentNode` — enrichissement parent data (service "conn")

### 1.4 Ce qui reste VRAIMENT à faire

| Item | État réel | Effort révisé |
|------|-----------|---------------|
| Deserialize types search | Pas fait (confirmé) | ~0.5j |
| Search port checkpoint | Pas fait (confirmé) | ~0.5j après Deserialize |
| Auto-drain post-rollback | Pas câblé dans MigrationRunner | ~0.5j |
| ScriptNode (Rhai) | Pas fait | ~2j |
| HttpNode | Pas fait | ~1j |
| Sparse index V2 mmap | Pas fait | ~3-5j |

**Total vrai travail restant** : beaucoup moins que ce que doc 03 suggérait. Les briques lourdes (MigrationRunner, nœuds search, migration nodes) sont déjà là.

---

## 2. Couplage Cypher — état actuel

### 2.1 Où est le Cypher ?

| Composant | Accède via | Cypher direct ? |
|-----------|-----------|-----------------|
| InsertRecordNode | `conn.execute_with_params()` | Oui — UNWIND MERGE |
| DeleteRecordNode | `conn.execute_with_params()` | Oui — MATCH DETACH DELETE |
| UpdateRecordNode | `conn.execute_with_params()` | Oui — MATCH SET |
| LinkRecordNode | `conn.execute_with_params()` | Oui — MATCH MERGE |
| KBGatherNode | `conn.execute_with_params()` | Oui — MATCH sur agrégats |
| KBUpdateNode | `conn.execute_with_params()` | Oui — MATCH SET |
| KBChunkNode | `conn.execute_with_params()` | Oui |
| EmbedNode | `conn.execute_with_params()` | Oui — UNWIND MATCH SET (vectors) |
| FlushNode | `conn.execute()` | Oui — CALL FLUSH_LUCIVY_INDEX |
| ChunkRecordNode | `conn.execute_with_params()` | Oui |
| RechunkDeleteNode | `conn.execute_with_params()` | Oui |
| **VectorSearchNode** | `conn.execute_with_params()` | Oui — Cypher vector similarity |
| **BM25SearchNode** | `conn.execute_with_params()` | Oui — CALL QUERY_LUCIVY_INDEX |
| **SparseSearchNode** | `conn.execute_with_params()` | Oui |
| SearchSourceNode | `catalog.resolve_search_target()` | **Non** — passe par Catalog |
| FuseResultsNode | aucun DB | Non |
| CypherNode | `conn.execute()` | Oui — par définition |
| ValidateNode | `conn.execute()` | Oui |
| MigrationRunner | `conn.execute_with_params()` | Oui — schema tables internes |

**Constat** : 16 nœuds sur 18 qui touchent la DB parlent Cypher directement. Seul SearchSourceNode passe par le Catalog. Le Catalog lui-même construit du Cypher en interne (`catalog.rs`, `search.rs`).

### 2.2 Le trait DbConnection — trop fin

```rust
pub trait DbConnection: Send + Sync {
    async fn execute(&self, cypher: &str) -> Result<QueryResult, DbError>;
    async fn execute_with_params(&self, cypher: &str, params: &[QueryParam]) -> Result<QueryResult, DbError>;
}
```

C'est un transport Cypher brut. Impossible d'implémenter ça pour pgvector ou qdrant — ils ne parlent pas Cypher.

---

## 3. Direction : le Catalog est la seule surface API

### 3.1 Principe

Les utilisateurs n'accèdent **qu'au Catalog** : `register_entity()`, `create()`, `update()`, `delete()`, `drain()`, `search()`, `link()`, `get()`, `count()`, etc. C'est déjà le cas aujourd'hui. L'API Catalog expose aussi les relations et le graph de manière suffisamment riche pour que les gens puissent structurer leurs données comme ils veulent.

**Pas de CypherNode exposé**, pas de raw query, pas de nœuds migration custom. Les utilisateurs composent avec les méthodes Catalog existantes.

### 3.2 Comment ça marche pour les sous-graphes

Quand l'utilisateur appelle `drain()`, le Catalog construit un sous-graphe d'ingestion (via `build_ingestion_graph()`) avec les nœuds builtin nécessaires : Insert, Delete, Update, Link, KB*, Embed, Flush. L'utilisateur ne voit jamais ces nœuds — c'est de l'implémentation interne.

Pareil pour `search()` : le Catalog orchestre Vector, BM25, Sparse, Fuse, Resolve en interne.

**L'utilisateur pense en termes de Catalog. Le Catalog pense en termes de sous-graphe. Les nœuds pensent en termes de backend.**

### 3.3 Multi-backend : notre problème, pas celui de l'utilisateur

Le Cypher est partout dans les nœuds (16/18 parlent `conn` directement). Mais c'est **notre** code interne. Si un jour on veut supporter pgvector :

1. On écrit de **nouvelles implémentations** des nœuds builtin qui parlent SQL au lieu de Cypher
2. Le Catalog choisit quels nœuds instancier selon le backend configuré
3. L'API Catalog ne change pas → le code utilisateur ne change pas

Pas besoin d'un trait `StorageBackend` ou `CatalogOps` abstrait maintenant. On le fera quand on aura un deuxième backend concret sous la main. Abstraire avant d'avoir 2 implémentations = abstraire dans le vide.

### 3.4 CypherNode et ValidateNode — usage interne uniquement

CypherNode et ValidateNode restent utiles pour :
- Les migrations internes (001_create_dataflow_tables.mmd)
- Le MigrationRunner (notre propre infra de schéma)
- Tests et debug

Mais ils ne sont **pas exposés** comme outil utilisateur. Pas de .mmd utilisateur, pas de Cypher dans l'API publique.

### 3.5 Et les migrations utilisateur ?

Pas de MigrationRunner exposé aux utilisateurs. Si quelqu'un veut ajouter un champ ou modifier son schéma :
- `register_entity()` avec la nouvelle config (idempotent si la table existe, avec migration automatique si les champs changent)
- Ou des méthodes Catalog dédiées qu'on ajoutera au besoin (`add_field()`, `reindex()`, etc.)

Le MigrationRunner reste un outil interne pour gérer les évolutions de schéma de rag3weaver lui-même.

---

## 4. Ce qui est actionnable

### Prochaines étapes concrètes

| Priorité | Tâche | Effort | Pourquoi |
|----------|-------|--------|----------|
| 1 | Tests E2E MigrationRunner (apply → rollback → verify) | ~1j | 0 test e2e — on ne sait pas si ça marche vraiment |
| 2 | Auto-drain post-rollback | ~0.5j | Dernière pièce du pipeline rollback |
| 3 | Deserialize types search | ~0.5j | Débloque checkpoint search + ScriptNode |
| 4 | `register_entity()` idempotent avec migration auto | ~2j | L'utilisateur re-register avec des champs en plus → ça migre tout seul |

### À venir (pas immédiat mais prévu)

- **ScriptNode / HttpNode** : existeront mais en variante Catalog — l'utilisateur aura un `CatalogScriptNode` ou équivalent qui reçoit un service Catalog, pas un `conn` brut. Le script/HTTP interagit avec le Catalog, jamais avec le backend directement.
- **Sparse index V2 mmap** : nécessaire pour passer au-delà de quelques milliers de docs. Remplacer la persistance bincode (full load/save) par mmap + LRU + WAL.

### Ce qu'on ne fait PAS maintenant

- Pas de trait abstrait multi-backend (on n'a qu'un backend)
- Pas de nœuds migration haut niveau exposés (AddFieldNode, etc.)
- Pas de refactor des nœuds pour découpler du Cypher
