# Doc 04 — Contexte technique pour la suite de l'implémentation

Date : 8 mars 2026
Réf : Doc 03 (plan implémentation), Doc 22 (réflexion nœuds génériques)

## État actuel

### Phase 1.1 — register_entity ✅ FAIT

**Fichiers modifiés :**
- `src/config.rs` — Ajout `SimpleFieldDef`, `EntityConfig` (lignes ~312-370)
- `src/schema.rs` — Ajout `generate_simple_chunk_table_ddl()`, `generate_simple_chunk_rel_ddl()` (entre `generate_index_chunk_table_ddl` et `generate_index_chunk_rel_ddl`)
- `src/catalog.rs` — Ajout champ `entity_configs: HashMap<String, EntityConfig>`, méthode `register_entity()`, helpers `is_simple_entity()`, `entity_config()` (entre `initialize()` et le bloc CRUD)

**Ce que `register_entity("Product", config)` fait :**
1. `CREATE NODE TABLE IF NOT EXISTS Product(_uuid, _content_hash, name, description, price, ...)`
2. `CREATE NODE TABLE IF NOT EXISTS Product_Chunk(_uuid, _parent_uuid, _parent_field, _text, _title, _text_hash, _embed_hash, _index, _start_char, _end_char, _start_line, _end_line, _core_start_char, _core_end_char, _core_start_line, _core_end_line, _content_offset, embedding FLOAT[dim], [sparse_indices, sparse_weights])`
3. `CREATE REL TABLE IF NOT EXISTS Product_CHUNKED_FROM(FROM Product_Chunk TO Product)`
4. `CREATE_LUCIVY_INDEX('Product', ['description', 'details'])` — FTS sur l'entité (contenu complet)
5. `CREATE_VECTOR_INDEX('Product_Chunk', 'Product_Chunk_vec', 'embedding', cosine)`
6. Optionnel : `CREATE_SPARSE_VECTOR_INDEX('Product_Chunk', 'sparse_indices', 'sparse_weights')`
7. Stocke `EntityConfig` dans `self.entity_configs` + crée un `EntityDef` dans `self.config.entities`

489 tests unitaires + 0 failed.

---

## Prochaines étapes (dans l'ordre)

### Phase 1.2 — EmbedNode (nouveau nœud générique)

**Problème** : KBEmbedNode (ex-EmbedRecordNode) est couplé KB : il utilise `_kb_name` pour nommer les colonnes (`{kb}_embedding`, `{kb}_sparse_indices`), et résout les signaux via `config.knowledge_bases`.

**Solution** : Nouveau `EmbedNode` avec des noms de colonnes configurables.

**Struct :**
```rust
pub struct EmbedNode {
    name: String,
    text_field: String,      // "_text" par défaut
    embedding_col: String,   // "embedding" par défaut
    sparse_col: String,      // "sparse_indices" / "sparse_weights" (préfixe)
    signals: SearchSignals,
    gpu_batch_size: usize,
    undo_data: Option<serde_json::Value>,
}
```

**Optimisations à reprendre de KBEmbedNode** (src/dataflow/record_nodes.rs lignes 473-924) :
1. **Hash idempotence** : comparer `_text_hash` vs `_embed_hash` → skip si identique (~80-90% saved on incremental)
2. **GPU batching** : `for chunk in works.chunks(self.gpu_batch_size)` → évite OOM
3. **Grouped UNWIND** : un seul Cypher `UNWIND $items AS item MATCH (n) WHERE n._uuid = item.uuid SET n.embedding = item.emb` par groupe `(entity_name)` au lieu d'un SET par entité
4. **Dual embedder** : si disponible, `embed_dual()` fait dense + sparse en un seul forward pass
5. **Signal routing** : 3 pipelines indépendants (dense_works, sparse_works, dual_works) selon les signaux

**Services requis :**
```rust
let embedder = ctx.service::<Arc<dyn Embedder>>("embedder");
let sparse_embedder = ctx.service::<Arc<dyn SparseEmbedder>>("sparse_embedder");  // optionnel
let dual_embedder = ctx.service::<Arc<dyn DualEmbedder>>("dual_embedder");  // optionnel
let embedding_dim = ctx.service::<usize>("embedding_dim");
let conn = ctx.service::<Arc<dyn DbConnection>>("conn");
```

**Différences avec KBEmbedNode :**
- Pas de `_kb_name` lookup — l'entité est déduite des records reçus
- Colonnes : `embedding` au lieu de `{kb}_embedding`
- Pas de `config.knowledge_bases` — les signaux sont dans la config du nœud
- Mêmes optis batch, mêmes traits Embedder/SparseEmbedder/DualEmbedder

**Fichiers à modifier :**
- `src/dataflow/record_nodes.rs` — struct EmbedNode + impl Node (placer après KBEmbedNode)
- `src/dataflow/node_factories.rs` — EmbedNodeFactory
- `src/dataflow/mod.rs` — export

**Logique partagée à extraire** (optionnel mais recommandé) :
- `batch_embed_dense(conn, embedder, items, entity, col)` — UNWIND SET pattern
- `batch_embed_sparse(conn, sparse_emb, items, entity, indices_col, weights_col)`
- `check_embed_hashes(conn, uuids, entity)` — fetch existing _embed_hash values
Pourrait aller dans `src/dataflow/embed_helpers.rs` ou rester inline.

---

### Phase 1.2b — Renommer ChunkRecordNode → KBChunkRecordNode + nouveau ChunkRecordNode simple

**Problème critique** : `ChunkRecordNode` (src/dataflow/record_nodes.rs lignes 966-1188) est **KB-couplé** malgré son nom "générique" :
- Ligne 1000 : `entity_has_chunks()` vérifie `content_for` (attribut KB)
- Ligne 1004-1011 : filtre par `kb_metadata` pour trouver les KBs de l'entité
- Ligne 1016 : boucle sur chaque KB name
- Ligne 1066 : met `_kb_name` sur les chunks
- Ligne 1022-1031 : utilise `kb_meta.chunking` pour la config chunker

**Solution** : Même pattern que KBEmbedNode / EmbedNode :
1. **Renommer** `ChunkRecordNode` → `KBChunkRecordNode` (5 fichiers, ~25 occurrences)
2. **Créer** un nouveau `ChunkRecordNode` simple (sans dépendance KB)

**Étape 1 — Renommage** (fichiers touchés) :
- `src/dataflow/record_nodes.rs` — struct, impl, Node trait, messages d'erreur
- `src/dataflow/node_factories.rs` — import, factory macro, register_builtins
- `src/dataflow/mod.rs` — export
- `src/catalog.rs` — commentaires (pas d'instantiation directe actuellement)
- `src/dataflow/checkpoint.rs` — test fixture string

**Étape 2 — Nouveau ChunkRecordNode simple** :

**Struct :**
```rust
pub struct ChunkRecordNode {
    name: String,
}
```

**Services requis :**
```rust
let config = ctx.service::<CatalogConfig>("config");
let entity_configs = ctx.service::<HashMap<String, EntityConfig>>("entity_configs");
let chunker_cache = ctx.service::<HashMap<ChunkerConfig, Chunker>>("chunker_cache");
```

**Logique `compute_chunks()` simple :**
1. Chercher l'entité dans `entity_configs` (pas `kb_metadata`)
2. Itérer les champs `is_content` triés (via `entity_config.content_fields()`)
3. Pour chaque champ content : chunker → générer chunks
4. Calculer `_content_offset` pour chaque champ (offset dans la concaténation)
5. Mettre `_title` depuis le champ `is_title` de l'entité parent
6. Pas de `_kb_name`
7. Initialiser `_embed_hash` à "" (pour le tracking idempotent par EmbedNode)
8. Relation : `{Entity}_CHUNKED_FROM` (au lieu de `{Entity}_HAS_CHUNK`)

**Inputs/Outputs** (identiques à KBChunkRecordNode) :
- Input : `entities` (BatchPayload<EntityRecord>)
- Outputs : `chunks` (Vec<EntityRecord>), `chunk_links` (Vec<RelationRecord>), `done` (Empty)

**Patron du `_content_offset`** :
```
Champs isContent triés : ["description", "details"]
description = "Le produit est..." (100 chars)
details = "Fabriqué en..." (50 chars)
→ concaténation : "Le produit est...\n\nFabriqué en..."
→ description chunks : _content_offset = 0
→ details chunks : _content_offset = 100 + 2 (séparateur "\n\n")
```

**Colonnes chunk output** (pour INSERT dans `{Entity}_Chunk`) :
```
_uuid, _parent_uuid, _parent_field, _text, _title, _text_hash, _embed_hash,
_index, _start_char, _end_char, _start_line, _end_line,
_core_start_char, _core_end_char, _core_start_line, _core_end_line,
_content_offset
```
(embedding, sparse_indices, sparse_weights sont ajoutés plus tard par EmbedNode)

---

### Phase 1.3 — ingest_entities sur Catalog

**Méthode** : `ingest_entities(entity_name, records)` construit et exécute un DataflowGraph :

```
InsertRecordNode("insert")
    →|inserted:entities| ChunkRecordNode("chunk")
        →|chunks| InsertRecordNode("chunk_insert")
            →|inserted:entities| EmbedNode("embed")
            →|done:trigger| LinkRecordNode("chunk_link")
        →|chunk_links| LinkRecordNode("chunk_link")  [fan-in sur chunk_links + trigger]
    →|done:trigger| FlushNode("flush_fts", tables=["{Entity}"])
```

**Services à enregistrer dans le graph :**
```rust
services.register::<Arc<dyn DbConnection>>("conn", self.conn.clone());
services.register::<Arc<dyn Embedder>>("embedder", self.embedder.clone());
services.register::<CatalogConfig>("config", self.config.clone());
services.register::<HashMap<String, EntityConfig>>("entity_configs", self.entity_configs.clone());
services.register::<HashMap<String, KBMetadata>>("kb_metadata", self.kb_metadata.clone());
services.register::<HashMap<ChunkerConfig, Chunker>>("chunker_cache", self.chunker_cache.clone());
services.register::<usize>("embedding_dim", self.config.embedding_dim);
// + sparse/dual si configurés
```

**Pattern à suivre** : voir `build_ingestion_graph()` existant (catalog.rs ~ligne 930-1100) pour le pattern de construction de graph programmatique (add_node, add_edge, etc.).

Le chunker_cache doit contenir un Chunker pour la config de l'EntityConfig. Si pas dans le cache, le créer et l'ajouter avant de lancer le graph.

---

### Phase 2 — Unifier search()

**Principe** : `catalog.search(name, query)` fonctionne pour KB et entités simples. Le catalog résout les noms de tables en interne.

**Résolution :**
| | KB | Simple |
|---|---|---|
| Table parent (BM25) | `{KB}_Index` | `{Entity}` |
| Table chunks (vector) | `{KB}_Index_Chunk` | `{Entity}_Chunk` |
| Relation chunks | `{KB}_Index_HAS_CHUNK` | `{Entity}_CHUNKED_FROM` |
| FTS fields | `['_title', '_content']` | `['description', 'details']` (isContent fields) |
| Embedding col | `{kb}_embedding` | `embedding` |
| Sparse cols | `{kb}_sparse_indices/weights` | `sparse_indices/weights` |

**Implémentation** : Créer un struct `SearchTarget` qui encapsule ces noms, et modifier `catalog.search()` pour :
1. Chercher `name` dans `kb_metadata` → résolution KB
2. Sinon chercher dans `entity_configs` → résolution simple
3. Passer `SearchTarget` aux fonctions de recherche existantes

Les fonctions de recherche (`search_bm25_raw`, `resolve_bm25_to_chunks`, `search_vector`) prennent déjà des noms de tables en paramètre. Le refactor est surtout au niveau du dispatch.

**Flow de recherche (identique KB et simple) :**
1. BM25 sur table parent (contenu complet) → highlights (byte offsets)
2. Résolution highlights → chunks via `_content_offset + _start_char/_end_char`
3. Vector search sur table chunks (embeddings)
4. RRF fusion au niveau chunk
5. Résolution chunks → entité source (optionnel)

---

## Patterns du code existant (référence rapide)

### Construction de graph programmatique (catalog.rs)
```rust
let mut graph = DataflowGraph::new();
let mut services = ServiceRegistry::new();
services.register::<Arc<dyn DbConnection>>("conn", self.conn.clone());
// ... register services ...

graph.add_node(Box::new(InsertRecordNode::new("insert"))).unwrap();
graph.add_node(Box::new(ChunkRecordNode::new("chunk"))).unwrap();  // nouveau ChunkRecordNode simple
graph.add_edge("insert", "inserted", "chunk", "entities").unwrap();
// ... add edges ...

let mut runtime = GraphRuntime::new(services);
// Feed input
runtime.set_input("insert", "entities", PortValue::Batch(...));
runtime.execute(&mut graph).await?;
```

### KBEmbedNode batch pattern (record_nodes.rs lignes 679-734)
```rust
// Group by (entity_name, kb) for batch UNWIND
let mut groups: HashMap<(String, String), Vec<(String, Vec<f32>)>> = HashMap::new();
for (work, emb) in works.iter().zip(embeddings.iter()) {
    groups.entry((work.entity_name.clone(), work.kb_name.clone()))
        .or_default()
        .push((work.uuid.clone(), emb.clone()));
}

for ((entity, kb), items) in &groups {
    let col = format!("{kb}_embedding");
    let params = items.iter().map(|(uuid, emb)| {
        serde_json::json!({"uuid": uuid, "emb": emb})
    }).collect::<Vec<_>>();

    let cypher = format!(
        "UNWIND $items AS item \
         MATCH (n:{entity}) WHERE n._uuid = item.uuid \
         SET n.{col} = item.emb, n._embed_hash = item.hash"
    );
    conn.execute_with_params(&cypher, params).await?;
}
```

### Trait Embedder (embedder.rs)
```rust
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError>;
    fn dim(&self) -> usize;
}

#[async_trait]
pub trait SparseEmbedder: Send + Sync {
    async fn embed_sparse(&self, texts: &[String]) -> Result<Vec<SparseVector>, EmbedError>;
}

#[async_trait]
pub trait DualEmbedder: Send + Sync {
    async fn embed_dual(&self, texts: &[String]) -> Result<(Vec<Vec<f32>>, Vec<SparseVector>), EmbedError>;
    fn dim(&self) -> usize;
}
```

### KBChunkRecordNode chunk output (ex-ChunkRecordNode, record_nodes.rs lignes 1059-1098)
Chaque chunk EntityRecord contient :
```
_uuid, _parent_uuid, _parent_field, _kb_name, _text, _text_hash,
_index, _start_char, _end_char, _start_line, _end_line,
_core_start_char, _core_end_char, _core_start_line, _core_end_line
```

### ChunkRecordNode chunk output (nouveau, simple)
Chaque chunk EntityRecord contient :
```
_uuid, _parent_uuid, _parent_field, _text, _title, _text_hash, _embed_hash,
_index, _start_char, _end_char, _start_line, _end_line,
_core_start_char, _core_end_char, _core_start_line, _core_end_line,
_content_offset
```

### Colonnes de Product_Chunk (simple)
```
_uuid, _parent_uuid, _parent_field, _text, _title, _text_hash, _embed_hash,
_index, _start_char, _end_char, _start_line, _end_line,
_core_start_char, _core_end_char, _core_start_line, _core_end_line,
_content_offset, embedding FLOAT[dim], [sparse_indices INT64[], sparse_weights DOUBLE[]]
```

### Colonnes de {KB}_Index_Chunk (KB)
```
_uuid, _parent_uuid, _parent_field, _kb_name, _source_field, _source_entity, _source_uuid,
_text, _text_hash, _embed_hash,
_index, _start_char, _end_char, _start_line, _end_line,
_core_start_char, _core_end_char, _core_start_line, _core_end_line,
_content_offset, {kb}_embedding FLOAT[dim], [{kb}_sparse_indices, {kb}_sparse_weights]
```

---

## Décisions architecturales prises

1. **Chunks toujours présents** — minimum 1 chunk par entité, même pour texte court
2. **FTS sur entité** (pas chunks) — BM25 a besoin du contenu complet pour bon scoring TF-IDF, highlights résolus vers chunks
3. **search() unifié** — même méthode pour KB et entités simples, dispatch interne
4. **Pas de searchEntities()** — supprimé, search() suffit
5. **EmbedNode séparé** de KBEmbedNode — pas de refactor risqué
6. **ChunkRecordNode séparé** de KBChunkRecordNode — même pattern que EmbedNode/KBEmbedNode, pas de refactor risqué
7. **Noms de colonnes simples** — `embedding` au lieu de `{kb}_embedding`

## Tasks actives

```
#173 ✅ Phase 1.1 — register_entity sur Catalog
#174 ✅ Phase 1.2 — EmbedNode + rename ChunkRecordNode → KBChunkRecordNode + nouveau ChunkRecordNode simple
#175 ✅ Phase 1.3 — ingest_entities sur Catalog
#176 ✅ Phase 1.4 — Tests unitaires (511 tests, +22)
#177 ⏳ Phase 2.1 — SearchTarget + résolution noms de tables
#178 ⏳ Phase 2.2 — Refactor search() pour accepter SearchTarget (bloqué par 177)
#179 ⏳ Phase 2.3 — Tests search unifié + tests E2E (bloqué par 176+178)
#180 ⏳ Phase 3 — Nœuds search génériques + templates Mermaid (reporté)
```
