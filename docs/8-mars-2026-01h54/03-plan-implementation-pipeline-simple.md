# Doc 03 — Plan d'implémentation : Pipeline simple (sans KB)

Date : 8 mars 2026
Réf : Doc 22 (réflexion nœuds génériques sans KB)

## Objectif

Implémenter `registerEntity` + `ingestEntities` + unifier `search()` pour permettre un pipeline simple "entity → chunk → embed → search" sans passer par l'abstraction KB.

---

## Phase 1 — Ingestion simple

**But** : pouvoir faire `registerEntity("Product", config)` + `ingestEntities("Product", records)` et avoir des entités + chunks + embeddings en base.

### 1.1 register_entity sur Catalog

Nouvelle méthode `register_entity(entity, config)` qui :
1. Crée la table entité : `CREATE NODE TABLE IF NOT EXISTS Product(...)`
2. Crée la table chunks : `CREATE NODE TABLE IF NOT EXISTS Product_Chunk(_uuid, _text, _title, _embed_hash, _start_char, _end_char, _content_offset, _index, _parent_field, embedding DOUBLE[], ...)`
3. Crée la relation : `CREATE REL TABLE IF NOT EXISTS Product_CHUNKED_FROM(FROM Product_Chunk TO Product)`
4. Crée l'index FTS sur l'entité (contenu complet) : `CREATE_LUCIVY_INDEX('Product', ['description', 'details'])`
5. Stocke la config en mémoire (HashMap `entity_configs`)

**Config** :
```rust
pub struct SimpleFieldDef {
    pub field_type: FieldType,
    pub is_title: bool,
    pub is_content: bool,
}

pub struct EntityConfig {
    pub fields: HashMap<String, SimpleFieldDef>,
    pub signals: SearchSignals,
}
```

**Fichiers** :
- `src/catalog.rs` — nouvelle méthode + stockage config
- `src/schema.rs` — DDL pour table chunks (réutiliser le pattern de `generate_chunk_table_ddl`)

### 1.2 EmbedNode (nouveau nœud générique)

Nouveau nœud séparé de KBEmbedNode. Config explicite, pas de dépendance KB.

```rust
pub struct EmbedNode {
    name: String,
    text_field: String,      // "_text" par défaut
    embedding_col: String,   // "embedding" par défaut
    sparse_col: String,      // "" = pas de sparse
    signals: SearchSignals,
    gpu_batch_size: usize,
}
```

**Exécution** :
1. Reçoit des entités sur le port `entities`
2. Pour chaque entité, lit `text_field`, calcule hash → compare avec `_embed_hash` (skip si inchangé)
3. Batch GPU → dense embeddings (+ sparse si configuré)
4. `SET n.{embedding_col} = [...]` via Cypher
5. Émet `done` sur le port de sortie

**Logique partagée** : extraire les helpers d'embedding de KBEmbedNode (batching GPU, hash, Cypher SET) dans un module commun (`src/dataflow/embed_helpers.rs` ou similaire).

**Fichiers** :
- `src/dataflow/record_nodes.rs` — struct EmbedNode + impl Node
- `src/dataflow/node_factories.rs` — EmbedNodeFactory
- `src/dataflow/mod.rs` — export

### 1.3 ingest_entities sur Catalog

Nouvelle méthode `ingest_entities(entity, records)` qui construit et exécute un DataflowGraph :

```
InsertRecordNode → ChunkRecordNode → InsertRecordNode (chunks)
                                   → LinkRecordNode (CHUNKED_FROM)
                                   → EmbedNode (sur les chunks)
                                   → FlushNode (FTS sur l'entité)
```

Tous les nœuds sauf EmbedNode existent déjà. Le graph est construit programmatiquement (pas de template Mermaid pour l'instant).

**Fichiers** :
- `src/catalog.rs` — nouvelle méthode `ingest_entities`

### 1.4 Tests Phase 1

- Test unitaire : `register_entity` crée les tables + index
- Test unitaire : `ingest_entities` insère entités + chunks + embeddings
- Test E2E : flow complet register → ingest → vérifier données en base
- Vérifier : `_content_offset`, `_start_char`, `_end_char` correctement remplis sur les chunks

---

## Phase 2 — Unifier search()

**But** : `catalog.search("Product", "red shoes")` fonctionne pour les entités simples, avec le même code que pour les KB.

### 2.1 Résolution de noms de tables

Le `catalog.search()` existant utilise hardcodé `{kb}_Index` / `{kb}_Index_Chunk`. Refactorer pour abstraire la résolution :

```rust
struct SearchTarget {
    parent_table: String,      // "Products_Index" ou "Product"
    chunk_table: String,       // "Products_Index_Chunk" ou "Product_Chunk"
    chunk_relation: String,    // "Products_Index_HAS_CHUNK" ou "Product_CHUNKED_FROM"
    fts_fields: Vec<String>,   // ["_content"] ou ["description", "details"]
    embedding_col: String,     // "{kb}_embedding" ou "embedding"
    signals: SearchSignals,
}
```

`catalog.search(name, query)` :
1. Cherche `name` dans `kb_metadata` → si trouvé, c'est un KB → résolution KB
2. Sinon cherche dans `entity_configs` → si trouvé, c'est une entité simple → résolution simple
3. Sinon erreur

### 2.2 Refactor search interne

Extraire la logique de recherche pour qu'elle prenne un `SearchTarget` au lieu de hardcoder les noms de tables KB :
- `search_bm25_raw(target.parent_table, target.fts_fields, ...)` — déjà paramétrique sur le nom de table
- `resolve_bm25_to_chunks(target.chunk_table, target.parent_table, ...)` — idem
- `search_vector(target.chunk_table, target.embedding_col, ...)` — idem
- `fuse_results(...)` — déjà générique

En pratique, le code existant prend déjà des noms de tables en paramètre (`entity`, `chunk_entity`). Le refactor est surtout au niveau du **dispatch** dans `catalog.search()` qui construit les bons noms.

### 2.3 Tests Phase 2

- Test E2E : `register_entity` → `ingest_entities` → `search()` → résultats corrects
- Test : BM25 highlights résolus vers les bons chunks
- Test : vector search sur chunks fonctionne
- Test : hybrid (BM25 + vector + RRF) fonctionne
- Test : filtrage via `SearchOptions.filters` fonctionne
- Comparer résultats KB vs entité simple (même données) → cohérence

---

## Phase 3 — Nœuds search génériques (composabilité Mermaid)

**But** : les power users peuvent composer des pipelines search custom via templates Mermaid, sans passer par `catalog.search()`.

### 3.1 Deserialize sur types search

Prérequis : les types `Query`, `Results`, `UnifiedResult` doivent implémenter Deserialize pour transiter via les ports du dataflow.

### 3.2 SearchSourceNode

Nouveau nœud qui émet un `Query` sur son port de sortie. Config : `query` (texte ou variable template).

### 3.3 VectorSearchNode

Recherche vectorielle directe sur une table de chunks.
- Input : `query` (Query)
- Output : `results` (Results, chunk-level)
- Config : `entity` (table chunks), `embedding_col`, `limit`

### 3.4 BM25SearchNode

BM25 sur entité parent + résolution highlights → chunks.
- Input : `query` (Query)
- Output : `results` (Results, chunk-level)
- Config : `entity` (table parent), `chunk_entity` (table chunks), `fields`, `limit`
- Encapsule `search_bm25_raw()` + `resolve_bm25_to_chunks()`

### 3.5 ResolveSourceNode

Résout chunks → entité source via une relation.
- Input/Output : `results` (Results)
- Config : `relation` (ex: "CHUNKED_FROM"), `direction`

### 3.6 FuseResultsNode

Fusionne plusieurs flux de résultats (fan-in) via RRF ou weighted.
- Input : `results` (Results, fan-in)
- Output : `results` (Results)
- Config : `strategy` (rrf/weighted)

### 3.7 Templates

- `templates/simple_ingestion.mmd`
- `templates/simple_search.mmd`
- `templates/hybrid_search.mmd`

### 3.8 Tests Phase 3

- Tests unitaires par nœud
- Test E2E : template simple_search fonctionne
- Test E2E : template hybrid_search fonctionne

---

## Ordre d'exécution recommandé

```
Phase 1.1 (register_entity)
    ↓
Phase 1.2 (EmbedNode) — peut commencer en parallèle
    ↓
Phase 1.3 (ingest_entities) — dépend de 1.1 + 1.2
    ↓
Phase 1.4 (tests ingestion)
    ↓
Phase 2.1 (SearchTarget) — dépend de 1.1 (entity_configs)
    ↓
Phase 2.2 (refactor search)
    ↓
Phase 2.3 (tests search)
    ↓
Phase 3 (nœuds + templates) — indépendant, peut être reporté
```

**Phases 1+2 donnent un pipeline simple fonctionnel end-to-end.**
Phase 3 ajoute la composabilité pour les power users — peut venir plus tard.
