# Doc 30 — Session : Décomposer AggregateRecordNode en 3 nœuds KB

Date : 7 mars 2026

## Contexte

Phase C terminée (doc 29, 392 tests, 0 fail). AggregateRecordNode est monolithique (7 étapes, ~440 lignes). On le découpe en 3 nœuds pour lisibilité du graphe.

## Topologie cible

```
AggregateRecord[] → GatherKBNode("gather_kb")
                        └── kb_content → UpdateKBNode("update_kb")
                                             └── kb_content → ChunkKBNode("chunk_kb")
                                                                  ├── entities → InsertRecordNode("agg_inserts")
                                                                  ├── relations → LinkRecordNode("agg_links")
                                                                  └── agg_inserts.inserted → EmbedRecordNode("agg_embeds")
```

## Travail effectué

### E.1 — records.rs : KBContentRecord + RecordSourceContent ✅

Ajouté 2 structs publics dans `src/records.rs` (avant PendingWork) :

```rust
pub struct RecordSourceContent {
    pub entity_name: String,
    pub entity_uuid: String,
    pub field_name: String,
    pub text: String,
}

pub struct KBContentRecord {
    pub index_entry_uuid: String,
    pub kb_name: String,
    pub title_text: String,
    pub content_text: String,
    pub new_hash: String,
    pub sources: Vec<RecordSourceContent>,
}
```

### E.2 — port.rs : PortType::KBContent ✅

Ajouté variant `KBContent` dans l'enum `PortType` (après le commentaire "Note: Aggregates already exists").

### E.3 — record_nodes.rs : 3 nouveaux nœuds ✅

Remplacé tout le bloc AggregateRecordNode (lines 1051-1618) par :

1. **Fonctions standalone** (extraites de impl AggregateRecordNode) :
   - `find_relation_to_entity(config, title_entity, content_entity) -> Option<(String, bool)>`
   - `generate_chunk_records(kb_name, index_entry_uuid, sources, chunker, chunk_table) -> (Vec<EntityRecord>, Vec<RelationRecord>)` — NOTE: paramètre `_title_text` supprimé (inutilisé)

2. **GatherKBNode** (Steps 1-4) :
   - Input: `aggregates` (Aggregates, required) + `trigger` (Empty, optional)
   - Output: `kb_content` (KBContent) + `done` (Empty)
   - Services: conn, config, kb_metadata
   - Méthode `gather_batch()` — même logique que l'ancien process_batch steps 1-4
   - `execute()` : dedup par index_entry_uuid, group par (title_entity, kb_name), appelle gather_batch per group, output Vec<KBContentRecord>
   - Metrics: ops, unique_ops, groups, group_summary, queries, skipped, changed

3. **UpdateKBNode** (Steps 5-6) :
   - Input: `kb_content` (KBContent, required)
   - Output: `kb_content` (KBContent, pass-through) + `done` (Empty)
   - Services: conn
   - Groups par kb_name pour UNWIND batching
   - Step 5: UNWIND SET {kb_name}_Index._title, _content, _content_hash
   - Step 6: UNWIND DETACH DELETE {kb_name}_Index_Chunk
   - Metrics: items, updated, deleted

4. **ChunkKBNode** (Step 7) :
   - Input: `kb_content` (KBContent, required)
   - Output: `entities` (Entities) + `relations` (Relations) + `done` (Empty)
   - Services: chunker_cache, kb_metadata
   - Appelle `generate_chunk_records()` per KBContentRecord
   - Metrics: items, chunks, relations

5. **Struct interne gardée** : `RecordAggState` (utilisé uniquement par GatherKBNode::gather_batch)

6. **Import ajouté** : `use crate::records::{..., KBContentRecord, RecordSourceContent}`

7. **Module doc mis à jour** : remplacé `AggregateRecordNode` par les 3 nœuds

### E.3b — dataflow/mod.rs : exports ✅

```rust
pub use record_nodes::{
    InsertRecordNode, LinkRecordNode, EmbedRecordNode,
    ChunkRecordNode, GatherKBNode, UpdateKBNode, ChunkKBNode,
};
```

### E.4 — catalog.rs : rewire build_ingestion_graph() — EN COURS

**Import mis à jour** ✅ :
```rust
use crate::dataflow::record_nodes::{
    ChunkKBNode, EmbedRecordNode, GatherKBNode, InsertRecordNode, LinkRecordNode,
    UpdateKBNode,
};
```

**Rewire du graphe** ❌ PAS ENCORE FAIT — le code catalog.rs référence encore `AggregateRecordNode` dans :
- `build_ingestion_graph()` lignes 911-936 : section `if has_aggregates` — doit être réécrite
- Ligne 362 : commentaire "Sentinel hash: empty string forces AggregateRecordNode" → à adapter
- Ligne 872 : commentaire topology doc → à adapter

**Ce qu'il faut écrire** (section `if has_aggregates` dans build_ingestion_graph) :

```rust
// 3. KB pipeline: gather → update → chunk, triggered after links
if has_aggregates {
    self.warm_chunker_cache();

    graph.add_node(Box::new(GatherKBNode::new("gather_kb"))).unwrap();
    graph.set_initial_input("gather_kb", "aggregates",
        PortValue::Batch(BatchPayload::new(PortType::Aggregates, pending.aggregates)));
    if has_relations {
        graph.connect("links", "done", "gather_kb", "trigger").unwrap();
    } else if has_entities {
        graph.connect("inserts", "done", "gather_kb", "trigger").unwrap();
    }

    graph.add_node(Box::new(UpdateKBNode::new("update_kb"))).unwrap();
    graph.connect("gather_kb", "kb_content", "update_kb", "kb_content").unwrap();

    graph.add_node(Box::new(ChunkKBNode::new("chunk_kb"))).unwrap();
    graph.connect("update_kb", "kb_content", "chunk_kb", "kb_content").unwrap();

    // Downstream standard: insert chunks → link chunks → embed chunks
    graph.add_node(Box::new(InsertRecordNode::new("agg_inserts"))).unwrap();
    graph.connect("chunk_kb", "entities", "agg_inserts", "entities").unwrap();

    graph.add_node(Box::new(LinkRecordNode::new("agg_links"))).unwrap();
    graph.connect("chunk_kb", "relations", "agg_links", "relations").unwrap();
    graph.connect("agg_inserts", "done", "agg_links", "trigger").unwrap();

    graph.add_node(Box::new(EmbedRecordNode::new("agg_embeds", 32))).unwrap();
    graph.connect("agg_inserts", "inserted", "agg_embeds", "entities").unwrap();
    graph.connect("agg_links", "done", "agg_embeds", "trigger").unwrap();
}
```

## État final

- **records.rs** : ✅ KBContentRecord + RecordSourceContent ajoutés
- **port.rs** : ✅ PortType::KBContent ajouté
- **record_nodes.rs** : ✅ 3 nœuds remplacent AggregateRecordNode
- **dataflow/mod.rs** : ✅ exports mis à jour
- **catalog.rs** : ✅ imports + `build_ingestion_graph()` réécrit + commentaires mis à jour
- **cargo check --lib** : ✅ compile clean
- **cargo test --lib** : ✅ 392 pass, 0 fail

## Fichiers modifiés

| Fichier | Changement |
|---|---|
| `src/records.rs` | +KBContentRecord, +RecordSourceContent (publics) |
| `src/dataflow/port.rs` | +PortType::KBContent |
| `src/dataflow/record_nodes.rs` | -AggregateRecordNode, +GatherKBNode, +UpdateKBNode, +ChunkKBNode, +find_relation_to_entity standalone, +generate_chunk_records standalone |
| `src/dataflow/mod.rs` | Exports: -AggregateRecordNode, +GatherKBNode, +UpdateKBNode, +ChunkKBNode |
| `src/catalog.rs` | Import (AggregateRecordNode → 3 KB nodes) + build_ingestion_graph() réécrit (3 KB nodes + 3 downstream) + commentaires topology/sentinel mis à jour |
