# Rag3Weaver — L3c catalog.rs termine (15 fevrier 2026)

Date : 15 fevrier 2026
Statut : catalog.rs termine, search.rs pas encore commence

---

## Bilan : 232 tests, 19 modules

```
cargo test → 232 passed, 0 failed
```

### Modules par etape

| Etape | Modules | Tests |
|-------|---------|:-----:|
| Etape 0 | events, config, embedder, connection | 35 |
| L1-L2 | schema, query, hash, uuid, chunker, fusion | 85 |
| L3a | filter, validator | 39 |
| L3b | refs, ops, persistence, queue | 47 |
| **L3c** | **catalog** | **26** |
| **Total** | **19 modules** | **232** |

---

## L3c — catalog.rs (26 tests)

Fichier : `src/catalog.rs` (~600 lignes)

### Types publics

- `KBMetadata` — metadata resolue d'un Knowledge Base (title, content, entities, search mode, weights, chunking config). Construit a `initialize()` a partir de `validate_schema()` + `config.knowledge_bases`.
- `CatalogError` — 8 variantes : NotInitialized, UnknownEntity, UnknownRelation, UnknownKB, NotFound, ValidationFailed, SchemaError, DbError, EmbedError.
- `UpdateStatus` — enum Updated / Unchanged.
- `UpdateResult` — uuid, entity, status, reembedded, chunks_created, chunks_deleted.
- `DeleteResult` — uuid, entity, chunks_deleted, relations_deleted.

### Struct Catalog

Le Catalog possede tout et est le point d'entree unique :

```rust
pub struct Catalog {
    conn: Arc<dyn DbConnection>,
    embedder: Arc<dyn Embedder>,
    config: CatalogConfig,
    queue: OperationQueue,
    event_bus: EventBus,
    kb_metadata: HashMap<String, KBMetadata>,
    initialized: bool,
}
```

`conn` et `embedder` sont en `Arc` pour etre partages avec les processors enregistres sur la queue.

### API publique

#### Lifecycle

| Methode | Sync/Async | Description |
|---------|:----------:|-------------|
| `new(conn, embedder, config)` | sync | Construit le Catalog, cree la queue avec FlushConfig depuis config |
| `initialize()` | async | Valide schema → DDL → indexes → KB metadata → enregistre 3 processors |

#### CRUD (synchrones, enqueue dans la queue)

| Methode | Description |
|---------|-------------|
| `create(entity_name, data)` → `EntityRef` | Genere UUID (hashsafe ou random), compute content_hash, enqueue InsertOp + EmbedOp(s) |
| `link(rel_name, from, to, properties)` → `RelationRef` | Enqueue LinkOp, from/to acceptent `impl Into<RefOrUuid>` |

#### Lectures directes (async, via DbConnection)

| Methode | Description |
|---------|-------------|
| `get(entity, uuid)` → `Option<HashMap>` | MATCH + RETURN |
| `get_many(entity, uuids)` → `Vec<HashMap>` | MATCH WHERE IN |
| `exists(entity, uuid)` → `bool` | COUNT |
| `count(entity)` → `usize` | COUNT total |

#### Update / Delete (async, direct DB + optional re-embed)

| Methode | Description |
|---------|-------------|
| `update(entity, uuid, data)` → `UpdateResult` | Compare content_hash, SET fields, re-enqueue EmbedOps si content change |
| `delete(entity, uuid)` → `DeleteResult` | Supprime chunks d'abord si chunked, puis DETACH DELETE |

#### Queue control

| Methode | Description |
|---------|-------------|
| `drain()` → `FlushResult` | Flush toutes les priorities |
| `flush_insertions()` → `FlushResult` | Flush priority <= 1 seulement |
| `has_pending()` → `bool` | Items en attente |
| `queue_stats()` → `QueueStats` | Snapshot complet |

#### Schema queries

| Methode | Description |
|---------|-------------|
| `get_kb_metadata(kb_name)` | Metadata KB resolue |
| `get_entity_def(name)` | Definition d'entite depuis config |
| `get_relation_def(name)` | Definition de relation depuis config |
| `get_kbs_for_entity(entity)` | Liste des KBs qui contiennent cette entite |

#### Event bus

| Methode | Description |
|---------|-------------|
| `subscribe()` | Retourne un `Receiver<CatalogEvent>` |

### 3 Processors internes

Enregistres sur la queue a `initialize()`. Chacun recoit `Arc<dyn DbConnection>` (et embedder pour Embed).

**InsertProcessor** :
1. Pour chaque InsertOp : tri des colonnes, `generate_insert_cypher()`, execute via conn
2. Extrait `_uuid` du data, `take_resolver().resolve(uuid)`

**LinkProcessor** :
1. Pour chaque LinkOp : `link.from.resolve().await` + `link.to.resolve().await`
2. Build `MATCH (a), (b) CREATE (a)-[:REL {props}]->(b)`
3. `take_resolver().resolve(from_uuid, to_uuid)`

**EmbedProcessor** :
1. Pour chaque EmbedOp : `entity_ref.ready().await` pour obtenir le UUID
2. Concatene les textes, appelle `embedder.embed()`
3. Verifie dimension, stocke via `SET n.kb_embedding = $embedding`

### Decisions de design

- **Arc pour conn/embedder** : les processors sont enregistres comme `Box<dyn Processor>` sur la queue, ils ont besoin de leur propre reference a la connexion.
- **UUID genere dans create(), pas dans le processor** : le Catalog connait la config hashsafe et les donnees, il genere le UUID et le met dans `full_data["_uuid"]`. Le processor l'extrait apres INSERT.
- **Textes pre-remplis dans EmbedOp** : `create()` extrait les textes title/content depuis le data HashMap et les met dans `EmbedOp.texts`. Le processor n'a pas besoin de relire la DB.
- **update() re-embed via queue** : si le content_hash change, update() cree des EntityRef deja resolus (`resolver.resolve(uuid)` immediat) et enqueue des EmbedOps. Il faut appeler `drain()` apres.
- **Pas de chunking dans EmbedProcessor (v1)** : les chunks nodes ne sont pas crees. L'embedding est stocke au niveau entite. Le chunking sera ajoute dans une version ulterieure.
- **Events minimaux** : `EntityUpdated` et `EntityDeleted` emis dans update/delete. DrainCompleted non encore emis (a ajouter).
- **FlushConfig mapping** : `config::FlushConfig` (serde) → `queue::FlushConfig` (runtime). Les champs `auto_flush` → `auto`, `max_count` → `max_count`, `completed_retention_ms` → `completed_retention_ms`.

### Tests (26)

| Test | Verifie |
|------|---------|
| `new_catalog` | Construction sans erreur |
| `initialize_success` | DDL execute, KB metadata construite |
| `initialize_validates_schema` | Schema invalide → CatalogError::ValidationFailed |
| `create_before_init_errors` | NotInitialized |
| `link_before_init_errors` | NotInitialized |
| `get_before_init_errors` | NotInitialized |
| `create_returns_pending_ref` | EntityRef pending avant drain |
| `create_unknown_entity_errors` | UnknownEntity |
| `create_enqueues_insert_and_embed` | 2 ops (insert + embed) dans la queue |
| `create_hashsafe_deterministic` | Meme titre → meme UUID sur 2 catalogs |
| `link_returns_pending_ref` | RelationRef pending |
| `link_unknown_relation_errors` | UnknownRelation |
| `drain_resolves_inserts` | EntityRef ready apres drain, UUID 36 chars |
| `drain_resolves_links` | 5 ops traitees (2 insert + 2 embed + 1 link), refs resolus |
| `drain_empty_queue` | 0 processed, 0 failed |
| `get_returns_none_empty_mock` | MockConnection → None |
| `exists_false_empty_mock` | MockConnection → false |
| `count_zero_empty_mock` | MockConnection → 0 |
| `get_many_empty_uuids` | Liste vide → resultat vide |
| `update_not_found` | MockConnection retourne vide → NotFound |
| `delete_succeeds_with_mock` | Delete sans erreur |
| `get_kb_metadata_after_init` | name, title, content, search, keyword_weight corrects |
| `get_kbs_for_entity_after_init` | Document → ["main"], Ghost → [] |
| `get_entity_def_and_relation_def` | Delegation vers config |
| `has_pending_and_stats` | Stats avant/apres create/drain |
| `flush_insertions_only` | Flush priority 1 seulement, embed reste pending |

---

## Fichiers crees/modifies

| Fichier | Action |
|---------|--------|
| `src/catalog.rs` | Cree — ~600 lignes, 26 tests |
| `src/lib.rs` | Modifie — ajout `pub mod catalog` + re-exports (Catalog, CatalogError, etc.) + re-export KBFieldRef depuis validator |

---

## Re-exports ajoutes a lib.rs

```rust
pub use catalog::{Catalog, CatalogError, DeleteResult, KBMetadata, UpdateResult, UpdateStatus};
pub use validator::{validate_schema, KBFieldRef};  // KBFieldRef ajoute
```

---

## Prochaines etapes

### search.rs (~15 tests)

Fonctions libres pour la recherche hybride, appelees par des methodes sur Catalog :

- `search_vector(conn, entity, kb, embedding, limit)` — MATCH + ORDER BY cosine_distance
- `search_bm25(conn, entity, kb, query, limit)` — CALL QUERY_LUCIVY_INDEX
- `fuse_results(vector, bm25, strategy, keyword_weight)` — delegue a fusion.rs
- `embed_query(embedder, query, cache)` — embedding avec cache HashMap

Types a definir :
- `SearchOptions` (limit, offset, consistency, hybrid_strategy, filters)
- `SearchResponse` (results, meta)
- `SearchResult` (uuid, score, entity, data, chunk)
- `SearchMeta` (query, kb, search_type, timing, counts)
- `ExploreOptions`, `ExploreResult`, `ExploreGraph`, `GraphNode`, `GraphEdge`

Methodes a ajouter sur Catalog :
- `search(kb_name, query, options)` → `SearchResponse`
- `search_with_explore(kb_name, query, options)` → `ExploreResult`

### Apres L3c

Integration Node.js (Phase C) : wrapper rag3weaver pour exposition via NAPI/WASM.
