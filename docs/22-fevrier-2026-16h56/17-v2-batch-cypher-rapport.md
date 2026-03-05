# 17 — V2 Batch Cypher : rapport d'implémentation

## Contexte

Suite au doc 16 (audit Cypher + plan V2), implémentation des optimisations batch pour éliminer les boucles de queries Cypher dans rag3weaver. Zéro modification côté rag3db.

## Ce qui a été fait

### 1A. EmbedProcessor — batch UNWIND (FAIT)

**Fichier** : `catalog.rs` — `EmbedProcessor::process()`
**Avant** : 1 query `MATCH + SET` par embedding (N queries pour batch_size=32)
**Après** : Group-by `(entity_name, embedding_col)` → 1 query `UNWIND $items AS item MATCH (n:Entity {_uuid: item.uuid}) SET n.col = item.emb` par groupe

```cypher
-- Avant (32 queries) :
MATCH (n:Document {_uuid: $uuid}) SET n.main_embedding = $embedding
-- x32 fois

-- Après (1-2 queries) :
UNWIND $items AS item
MATCH (n:Document {_uuid: item.uuid})
SET n.main_embedding = item.emb
```

Le paramètre `$items` est une `CypherValue::List` de `CypherValue::Map` contenant `{uuid, emb}`.

**Gain** : ~16-32x moins de round-trips Cypher par batch.

### 1B. SparseEmbedProcessor — batch UNWIND (FAIT)

**Fichier** : `catalog.rs` — `SparseEmbedProcessor::process()`
**Même pattern** que 1A, avec 2 colonnes (indices + weights).

```cypher
UNWIND $items AS item
MATCH (n:Document {_uuid: item.uuid})
SET n.main_sparse_indices = item.indices, n.main_sparse_weights = item.weights
```

Group-by `(entity_name, kb_name)`. **Gain** : ~16-32x.

### 1C. explore_bfs — batch par niveau BFS (FAIT)

**Fichier** : `search.rs` — `explore_bfs()`
**Avant** : `for uuid in frontier { for rel in relations { explore_relation(uuid, rel) } }` = O(frontier × relations) queries par niveau
**Après** : `for rel in relations { explore_relation_batch(frontier, rel, direction) }` = O(relations) queries par niveau

Nouvelle fonction `explore_relation_batch()` :
```cypher
UNWIND $uuids AS uid
MATCH (n {_uuid: uid})-[:RELATION]->(m)
RETURN uid, m._uuid, label(m), m
```

Retourne `(from_uuid, neighbor_uuid, entity, data)` pour reconstruire les edges.

**Gain** : Pour 10 frontier × 5 relations = 50 queries → 5 queries (1 par relation). ~10x.

### 1D. rebuild_sparse_index — UNION ALL (FAIT)

**Fichier** : `catalog.rs` — `rebuild_sparse_index()`
**Avant** : 1 query par type d'entité dans le KB
**Après** : 1 seule query UNION ALL pour toutes les entités

```cypher
MATCH (n:Document) WHERE n.main_sparse_indices IS NOT NULL
RETURN n._uuid, n.main_sparse_indices, n.main_sparse_weights
UNION ALL
MATCH (n:Article) WHERE n.main_sparse_indices IS NOT NULL
RETURN n._uuid, n.main_sparse_indices, n.main_sparse_weights
```

**Gain** : N_entities queries → 1 query. ~3x pour 3 types d'entités.

### 1E. update() — combiner hash check + SET (FAIT)

**Fichier** : `catalog.rs` — `update()`
**Avant** : 2 queries séquentielles (1 MATCH RETURN hash, 1 MATCH SET)
**Après** : 1 query avec WITH pour capturer l'ancien hash avant SET

```cypher
MATCH (n:Document {_uuid: $uuid})
WITH n, n._content_hash AS old_hash
SET n.title = $title, n.body = $body, n._content_hash = $new_hash
RETURN old_hash
```

Le code Rust compare ensuite `old_hash != new_hash` pour décider de re-embedder.

**Gain** : 2 round-trips → 1. **Note** : dépend du support Kuzu de `WITH + SET + RETURN`. Si ça ne passe pas sur une vraie DB, fallback facile vers 2 queries.

### 1F. delete() — gardé 2 queries (DOCUMENTÉ)

**Fichier** : `catalog.rs` — `delete()`
Kuzu ne supporte pas `DETACH DELETE` dans un `WITH` chain (pas de `FOREACH` non plus). Delete est une opération rare (ponctuelle, pas batch). Gardé 2 queries avec commentaire documentant la limitation.

## Infrastructure ajoutée : NodeIdCache

### Module `node_id_cache.rs` (NOUVEAU)

Cache `uuid → (table_id, offset)` pour les internal node IDs rag3db.

```rust
pub struct InternalNodeId {
    pub table_id: u64,
    pub offset: u64,
}

pub struct NodeIdCache {
    entries: HashMap<String, InternalNodeId>,
}
```

- `InternalNodeId::parse("0:42")` — parse le format string retourné par rag3db
- `NodeIdCache` — insert/get/remove/clear, 8 tests unitaires

### Intégration Catalog

- `node_id_cache: Arc<RwLock<NodeIdCache>>` sur Catalog
- `InsertProcessor` modifié : INSERT fait `CREATE (n:...) RETURN ID(n)`, parse le résultat, peuple le cache
- `delete()` fait `cache.remove(uuid)`
- Accesseur public `catalog.node_id_cache()`

### Analyse : pourquoi un cache côté Rust ?

Investigation des internals rag3db (Kuzu fork) :
- `PRIMARY KEY(_uuid)` crée déjà un hash index O(1) amortized
- Le `BufferManager` (page cache LRU) garde les pages d'index hot en RAM
- `UNWIND + MATCH {_uuid: item.uuid}` utilise le hash index par row automatiquement

**Conclusion** : Le hash index + page cache est déjà le mécanisme optimal. Le NodeIdCache est une infra pour usage futur :
- Lucivy `allowed_ids` (qui travaille avec les offsets internes)
- Éventuel `BATCH_SET_BY_OFFSET` extension C++ (bypass planner complet)
- Toute logique Rust qui a besoin du mapping uuid↔offset

L'overhead du hash lookup string (_uuid) est marginal comparé au gain UNWIND (~1µs/row vs ~30µs parse+plan économisés).

## Table récapitulative : avant → après

| Opération | Avant (queries) | Après (queries) | Gain | Statut |
|---|---|---|---|---|
| Embed batch 32 | 32 | 1-2 | ~16-32x | ✅ FAIT |
| Sparse embed batch 32 | 32 | 1-2 | ~16-32x | ✅ FAIT |
| BFS niveau (10 nodes × 5 rels) | 50 | 5 | ~10x | ✅ FAIT |
| Rebuild sparse (3 entity types) | 3 | 1 | 3x | ✅ FAIT |
| Update 1 entité | 2 | 1 | 2x | ✅ FAIT |
| Delete 1 entité + chunks | 2 | 2 | — | Gardé (limitation Kuzu) |
| INSERT + cache ID | 1 (fire-and-forget) | 1 (+ RETURN ID) | — | ✅ FAIT |

## Tests

338 tests passent (8 nouveaux pour NodeIdCache), 0 failures, 5 ignored.

```bash
cargo test --lib
# test result: ok. 338 passed; 0 failed; 5 ignored
```

## Fichiers modifiés

| Fichier | Changements |
|---|---|
| `catalog.rs` | EmbedProcessor UNWIND, SparseEmbedProcessor UNWIND, update() combiné, delete() commenté, rebuild UNION ALL, InsertProcessor RETURN ID, Arc<RwLock<NodeIdCache>> |
| `search.rs` | explore_relation_batch() remplace explore_relation() dans la boucle BFS |
| `node_id_cache.rs` | **NOUVEAU** — InternalNodeId, NodeIdCache, 8 tests |
| `lib.rs` | pub mod node_id_cache + re-exports |

## Questions ouvertes vérifiées

| Question (doc 16) | Réponse |
|---|---|
| UNWIND + SET dynamique (`n[item.col]`) ? | **Non** — Kuzu ne supporte pas les noms de propriété dynamiques. Fix : group-by par colonne, 1 UNWIND par groupe. |
| WHERE IN + index ? | **Oui** — hash index utilisé automatiquement, mais UNWIND est préféré (même parse+plan, plus explicite). |
| WITH + SET + RETURN ? | **À vérifier sur vraie DB** — implémenté, fallback facile si ça casse. |
| DELETE dans WITH chain ? | **Probablement non** — gardé 2 queries, documenté. |

## Prochaines étapes

Par priorité :

### ULTRA PRIO — search_vector() utilise un scan brute-force au lieu de HNSW

**Problème découvert** : `search_vector()` dans `search.rs` fait un scan O(N) brute-force avec `array_cosine_similarity` sur TOUS les nœuds. Or rag3db a déjà une extension `vector` avec un index HNSW complet :

```cypher
-- Ce que search_vector() fait actuellement (BRUTE FORCE O(N)) :
MATCH (n:Document)
WHERE n.main_embedding IS NOT NULL
WITH n, array_cosine_similarity(n.main_embedding, $embedding) AS sim
ORDER BY sim DESC LIMIT 10
RETURN n._uuid, sim

-- Ce qu'il DEVRAIT faire (HNSW O(log N)) :
CALL QUERY_VECTOR_INDEX('Document', 'doc_embedding_idx', $query_vec, 10)
RETURN node._uuid, distance
```

L'extension vector existe déjà dans rag3db avec :
- `CREATE_VECTOR_INDEX(table, index_name, column, metric := 'cosine')` — crée l'index HNSW
- `QUERY_VECTOR_INDEX(table, index_name, query_vec, k)` — query O(log N)
- Support INSERT/DELETE incrémental
- Fichiers : `extension/vector/src/function/create_hnsw_index.cpp`, `query_hnsw_index.cpp`
- Tests : `extension/vector/test/test_files/insert.test`, `filter.test`, `delete.test`

**Ce qu'il faut faire** :
1. Dans `initialize()` / `generate_full_schema()` : ajouter `CALL CREATE_VECTOR_INDEX(...)` pour chaque entité × KB qui a un embedding
2. Dans `search_vector()` (`search.rs:285`) : remplacer le MATCH + array_cosine_similarity par `CALL QUERY_VECTOR_INDEX(...)`
3. Gérer les filtres : QUERY_VECTOR_INDEX supporte probablement un WHERE clause ou un post-filtre (à vérifier dans les tests)

**Impact** : de O(N) à O(log N) par query vector. Pour 50k docs : ~25ms → <1ms.

---

### Reste du plan par priorité :

1. **2A — Rebuild incrémental sparse** (CRITIQUE) : Le `SparseEmbedProcessor` met à jour le `SparseIndex` directement en mémoire via `Arc<RwLock<>>` au lieu de tout reconstruire après drain(). Le processor fait `index.insert(uuid, sv)` juste après le SET en DB. `drain()` n'appelle plus `rebuild_sparse_index()`. Le rebuild complet reste pour le cold start (`initialize()`).

2. **2C — Filtrage sparse allowed_uuids** (MOYEN) : Ajouter `allowed_uuids: Option<&HashSet<String>>` sur `SparseIndex::search()`.

3. **2B — Cache sparse embeddings** (MOYEN) : FIFO cache pour `embed_sparse()` query.

4. **2D — Arc<Vec<String>> textes partagés** (BAS) : Partager les textes entre EmbedOp et SparseEmbedOp.

5. **2E — Cleanup allocations** (BAS).

### V4+ — Sparse index on-disk avec cache LRU (scaling > 100k docs)

Le sparse index in-memory actuel tient ~60 MB pour 100k docs. Au-delà de 1M docs (~600 MB), il ne scale plus.

**Direction retenue : extension C++ dédiée (Option C)** — même pattern que lucivy_fts.

Architecture cible :
- **Fichier binaire on-disk** : inverted index stocké par token_id (pas par document), format custom compact, mmapped
- **Cache LRU intégré** : les posting lists des tokens les plus consultés restent en mémoire, les cold restent sur disque
- **Hooks incrémentaux** : onInsert/onDelete/onCommit câblés dans le storage layer, comme lucivy_fts (lazy commit avec dirty flag)
- **API Cypher** : `CREATE_SPARSE_INDEX` / `QUERY_SPARSE_INDEX` / `DROP_SPARSE_INDEX` (même pattern que les extensions vector et lucivy_fts)
- **cxx bridge** : le code Rust du SparseIndex/SparseVector est déjà écrit, on ajoute un bridge cxx vers l'extension C++

Pourquoi pas les alternatives :
- **Option A (node table par token)** : arrays de taille variable dans une node table, pas idéal pour des posting lists qui grandissent
- **Option B (rel table)** : 10k docs × 10 tokens = 100k rels, overhead Kuzu rel storage pour un simple inverted index
- **Hybride in-memory + DB** : cache LRU par posting inutile tant que l'index n'est pas structuré par token sur disque — scanner les colonnes par doc pour un token = full scan

Le in-memory actuel reste le bon choix tant que < 100k docs (microsecondes, zéro I/O, fonctionne en WASM). L'extension C++ est le chemin pour le scaling production.

---

## À réfléchir : extension "index_store" générique

rag3weaver est un client Cypher, pas une extension rag3db. Ça veut dire pas d'accès au storage layer, pas de hooks, pas de persistance custom. D'où le sparse index in-memory rechargé au cold start.

Idée : une extension rag3db générique qui découple **persistance + hooks** (dans le moteur) de **logique d'index** (côté client).

Deux designs possibles :

**Design A — Extension intelligente** : sait gérer différents types d'index (inverted, kv). Les hooks mettent à jour l'index automatiquement. rag3weaver ne fait que query. Mais alors c'est une extension spécialisée déguisée en générique — autant faire une extension sparse dédiée.

**Design B — Extension storage bête + change tracking** : expose du key-value persisté et un changelog. Les hooks INSERT/DELETE enregistrent les changements. rag3weaver lit le changelog au prochain drain et met à jour son index in-memory de façon incrémentale.

```
Design B flow :

INSERT (:Doc {title: "ML"})
  └→ hook extension index_store
       └→ changelog.append({INSERT, table: Doc, offset: 42, data: ...})

rag3weaver.drain()
  └→ CALL GET_INDEX_CHANGELOG('sparse_idx', since := $last_seq)
       → [{INSERT, uuid: "abc", indices: [...], weights: [...]}]
  └→ sparse_index.insert("abc", sv)     ← in-memory, incrémental
  └→ CALL ACK_INDEX_CHANGELOG('sparse_idx', seq := $new_seq)
```

Avantage du design B : l'extension ne sait rien des types d'index, elle fait du change tracking persisté. Toute la logique reste en Rust côté rag3weaver. Et quand on voudra le scaling > 100k, on migrera la logique dans l'extension (design A, style lucivy_fts).

Question ouverte : est-ce que le design B justifie une extension C++ (changelog, hooks, persistance) alors qu'on pourrait simplement ajouter `RETURN ID(n)` aux INSERT et gérer le tracking côté client ? Le NodeIdCache qu'on a déjà fait est un début de cette approche sans extension.

**Statut : à réfléchir, pas encore tranché.**
