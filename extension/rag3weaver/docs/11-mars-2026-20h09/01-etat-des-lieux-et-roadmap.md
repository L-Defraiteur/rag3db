# Doc 01 — État des lieux et roadmap

Date : 11 mars 2026
Réf : Docs 09-15 du 8 mars 2026

## Ce qui est fait

### Ingestion (create)
- `register_entity()` + `ingest_entities()` — simple entities, pipeline dataflow complet
- `create()` + `link()` + `drain()` — KB entities, PendingWork queue
- Chunking, embedding (dense + sparse + dual en un forward pass), FTS indexation
- `DualEmbedder` opérationnel : `CandleDualEmbedder` (BM42 model) et `BgeM3Embedder`

### CRUD simple entities
- `update()` — détection content change via hash, rechunk + re-embed si changé
- `delete()` — cascade-delete chunks + flush FTS
- `batch_update()` / `batch_delete()` — batching GPU, un seul appel embed pour N items
- `rechunk_simple_entities()` — helper dataflow (delete old chunks → re-chunk → re-embed)
- 15/15 tests E2E `e2e_simple_entity` verts

### CRUD KB entities
- `create()` / `link()` / `drain()` / `update()` / `delete()` — via AggregateRecords
- Pipeline dataflow complet avec KB resolution

### Search
- 22 nœuds dataflow composables (6 generic search, 5 KB search, 8 ingestion, 2 migration, 1 Cypher)
- BM25 multi-field avec highlights + chunk resolution
- Vector search, sparse search, fusion (RRF)
- 120/120 tests E2E non-régression

### FTS (tantivy_fts)
- Extension C++ complète (CREATE, QUERY, DROP)
- Hooks insert/update/delete fonctionnels (bug update() hook fixé le 8 mars)
- Lazy commit (dirty_ flag)
- Filter fields natifs
- 15 tests GTest E2E

### Builds
- Node.js natif : rag3dbjs.node + LOAD EXTENSION
- WASM : rag3db_wasm.js 17MB, tantivy_fts statiquement linké

---

## Ce qui reste à faire

### 1. Queue/drain unifié pour update/delete

**Priorité : haute** — Simplification architecturale + évite les bugs FTS hook

L'implémentation actuelle de `update()` / `delete()` est immédiate (pas de queue). `batch_update()` et `batch_delete()` groupent mais restent des méthodes séparées. Trois bugs ont été nécessaires pour stabiliser ça (Doc 15).

L'approche queue unifie tout dans `PendingWork` + `drain()` :

```rust
pub struct PendingWork {
    pub entities: Vec<EntityRecord>,      // create (existant)
    pub relations: Vec<RelationRecord>,   // link (existant)
    pub aggregates: Vec<AggregateRecord>, // KB rebuild (existant)
    pub updates: Vec<UpdateRecord>,       // NEW
    pub deletes: Vec<DeleteRecord>,       // NEW
}
```

```
catalog.update("Product", uuid, data)  → enqueue UpdateRecord
catalog.delete("Product", uuid)        → enqueue DeleteRecord
catalog.drain()                        → process ALL :
  1. Apply deletes (batch DETACH DELETE) — libère d'abord
  2. Apply field updates (batch UNWIND SET)
  3. Detect content changes → batch re-chunk
  4. Batch re-embed (un seul appel GPU)
  5. FTS flush
```

**Avantages clés** :
- Plus besoin de `batch_update()` / `batch_delete()` séparés — un seul chemin
- Évite le bug FTS update hook : DELETE old + INSERT new au lieu de MERGE+SET → le hook C++ `update()` n'est jamais appelé
- Cohérent avec `ingest_entities()` qui utilise déjà PendingWork + drain
- Un seul point de commit → plus simple à raisonner

**Challenges** :
1. API breaking — `update()` ne retourne plus `UpdateResult` synchrone. Options : `oneshot::Receiver` ou statuts agrégés dans `DrainResult`.
2. Conflits — create puis delete même UUID, ou deux updates avant drain → logique merge/dedup.
3. Ordering KB — update sur content entity doit trigger re-aggregate → AggregateRecords générés au drain, pas à l'enqueue.

**Estimé** : 2-3 sessions.

Réf : Doc 11 (analyse batch), Doc 15 (discussion queue approach)

### 2. Deserialize sur les types search

**Priorité : moyenne** — Bloque ScriptNode round-trip

`UnifiedResult`, `ChildSummary`, `ChunkInfo`, `SearchMeta` n'ont que `Serialize`, pas `Deserialize`. Empêche la désérialisation depuis les ports typés du dataflow et bloque le round-trip dans un futur ScriptNode.

~50 lignes, derive `Deserialize` + éventuels ajustements.

### 3. E2E tests pour les generic search nodes

**Priorité : moyenne** — Couverture manquante

Les 6 nœuds de recherche générique (SearchSourceNode, VectorSearchNode, BM25SearchNode, SparseSearchNode, FuseResultsNode, ResolveParentNode) sont testés unitairement mais pas via un pipeline E2E complet sur une vraie DB.

### 4. Phase C : wrapper Node.js pour rag3weaver

**Priorité : à définir** — Intégration finale

Wrapper Node.js exposant les fonctions Catalog (register, ingest, search, update, delete) depuis l'API Node rag3dbjs. Pas commencé.

### 5. Extensibilité dataflow (Phase 5)

**Priorité : basse** — Dépend des besoins concrets

- **ScriptNode (Rhai)** — Transformations sandboxées dans le dataflow (filter, reranking, multi-step Cypher). Feature flag `rhai-script`. Bloqué par #2 (Deserialize).
- **HttpNode** — Appels HTTP déclaratifs (REST, LLM, APIs). Feature flag `http-node`.

### 6. Sparse index : état actuel et améliorations possibles

**Priorité : basse** — Scale suffisamment pour l'instant

**État actuel** (`extension/sparse_vector/`) :
- Index inversé in-memory : `HashMap<u32, Vec<(u64, f32)>>` (token_id → posting list)
- Persistance **bincode** : `open()` = deserialize tout le fichier `sparse.bin` en RAM, `commit()` = serialize tout d'un coup
- Pas de mmap, pas de WAL, pas de compression des posting lists
- Fonctionne correctement pour des volumes raisonnables (milliers de docs)
- Bridge cxx vers C++ (même pattern que tantivy_fts)
- Embeddings fournis par `CandleDualEmbedder` (BM42 CLS attention weights) ou `BgeM3Embedder` — dense + sparse en un seul forward pass BERT (DÉJÀ FAIT, pas un TODO)

**Améliorations possibles** (si les volumes l'exigent) :
- **V2 persistance mmap** — Format on-disk avec mmap + LRU cache + WAL au lieu de bincode full-load. Élimine le coût O(N) à chaque open/commit. ~3-5 jours.
- **Sub-word merging** — Fuse WordPiece sub-tokens → mots complets dans les vecteurs sparse. Réduit la taille des posting lists et améliore la précision. ~1-2 jours.
- **SPLADE** — Modèle d'expansion de termes dédié (génère des tokens hors du texte original). Requiert V2 mmap (vecteurs plus denses). ~5-7 jours.

---

## Prochaine action recommandée

**Queue/drain unifié (#1)** — C'est le refactor le plus impactant : simplifie l'architecture, élimine une classe de bugs FTS, et rend `batch_update/batch_delete` obsolètes. 2-3 sessions.
