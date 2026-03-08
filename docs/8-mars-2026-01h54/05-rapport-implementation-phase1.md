# Doc 05 — Rapport d'implémentation Phase 1 (pipeline simple)

Date : 8 mars 2026
Réf : Doc 03 (plan), Doc 04 (contexte technique)

## Résumé

Phase 1 complète : le pipeline simple (registerEntity → ingestEntities) est implémenté avec 511 tests (489 → 511, +22 nouveaux). Tous passent.

---

## Ce qui a été fait cette session

### 1.2b — Renommage ChunkRecordNode → KBChunkRecordNode

**Motivation** : `ChunkRecordNode` était KB-couplé (utilise `kb_metadata`, `_kb_name`, `content_for`). Renommé pour libérer le nom pour la version simple. Même pattern que KBEmbedNode / EmbedNode.

**Fichiers modifiés (5)** :
- `src/dataflow/record_nodes.rs` — struct, impl, Node trait, messages d'erreur (~15 occurrences)
- `src/dataflow/node_factories.rs` — import, factory macro, register_builtins
- `src/dataflow/mod.rs` — export
- `src/catalog.rs` — commentaires
- `src/dataflow/checkpoint.rs` — test fixture string

### 1.2c — Nouveau ChunkRecordNode (simple)

**Fichier** : `src/dataflow/record_nodes.rs` (après KBChunkRecordNode, avant KB Pipeline Nodes)

**Struct** : `ChunkRecordNode { name: String }`

**Services requis** : `entity_configs` (HashMap<String, EntityConfig>), `chunker_cache`

**Différences avec KBChunkRecordNode** :
| | KBChunkRecordNode | ChunkRecordNode |
|---|---|---|
| Source config | `kb_metadata` (+ `config`) | `entity_configs` |
| Champs à chunker | `kb_meta.content` filtrés par entity | `entity_config.content_fields()` (sorted) |
| `_kb_name` sur chunk | Oui | Non |
| `_title` sur chunk | Non | Oui (depuis champ `is_title` du parent) |
| `_embed_hash` | Non | Oui (initialisé à "") |
| `_content_offset` | Non | Oui (calculé par champ, séparateur "\n\n") |
| Relation | `{Entity}_HAS_CHUNK` | `{Entity}_CHUNKED_FROM` |

**Logique `_content_offset`** :
```
content_fields sorted = ["description", "details"]
description (100 chars) → offset = 0
details (50 chars) → offset = 100 + 2 ("\n\n") = 102
```

**Factory** : `ChunkRecordNodeFactory` (macro `named_factory!`) dans `node_factories.rs`
**Export** : ajouté dans `mod.rs`

### 1.2a — EmbedNode (nouveau nœud générique)

**Fichier** : `src/dataflow/record_nodes.rs` (après ChunkRecordNode, avant KB Pipeline Nodes)

**Struct** :
```rust
pub struct EmbedNode {
    name: String,
    text_field: String,      // "_text" par défaut
    embedding_col: String,   // "embedding" par défaut
    sparse_col: String,      // "sparse" (préfixe → sparse_indices, sparse_weights)
    signals: SearchSignals,  // configuré sur le nœud, pas via KB
    gpu_batch_size: usize,
    undo_data: Option<serde_json::Value>,
}
```

**Constructeur** : `EmbedNode::new(name, signals, gpu_batch_size)` + `.with_columns(text_field, embedding_col, sparse_col)`

**Services requis** : `conn`, `embedder`, `embedding_dim`, optionnels `sparse_embedder`, `dual_embedder`, `has_sparse`, `has_dual`

**Optimisations reprises de KBEmbedNode** :
1. **Hash idempotence** : compare `_text_hash` vs `_embed_hash` (+ `<> ''` car initialisé à "")
2. **GPU batching** : `for chunk in works.chunks(self.gpu_batch_size)`
3. **Grouped UNWIND** : batch par `entity_name` (pas par `(entity, kb)`)
4. **Dual embedder** : dense + sparse en un seul forward pass
5. **Signal routing** : 3 pipelines (dense, sparse, dual) selon `self.signals`
6. **Undo** : reset `_embed_hash` à "" (pas NULL, car simple entities)

**Différences avec KBEmbedNode** :
| | KBEmbedNode | EmbedNode |
|---|---|---|
| Colonnes | `{kb}_embedding`, `{kb}_sparse_*` | `embedding`, `sparse_indices/weights` (configurable) |
| Signaux | Via `config.knowledge_bases[kb].signals` | Via `self.signals` |
| Groupement batch | Par `(entity_name, kb_name)` | Par `entity_name` |
| `_kb_name` lookup | Oui | Non |
| Undo hash reset | `NULL` | `""` |

**Factory** : `EmbedNodeFactory` (manuelle, avec params config) dans `node_factories.rs`

### 1.3 — ingest_entities sur Catalog

**Fichier** : `src/catalog.rs` (après `entity_config()`, avant CRUD)

**Signature** : `pub async fn ingest_entities(&mut self, entity_name: &str, records: Vec<BTreeMap<String, CypherValue>>) -> Result<FlushResult, CatalogError>`

**Dataflow graph construit** :
```
InsertRecordNode("insert")
    →|inserted:entities| ChunkRecordNode("chunk")
        →|chunks| InsertRecordNode("chunk_insert")
            →|inserted:entities| EmbedNode("embed")
        →|chunk_links| LinkRecordNode("chunk_link")
            ←|trigger| chunk_insert.done
    →|done:trigger| FlushNode("flush_fts", tables=["{Entity}"])
```

**Détails** :
- UUID déterministe : `hashsafe_uuid` si configuré, sinon hash de toutes les données triées
- `_content_hash` calculé à partir des champs `is_content`
- Chunker cache réchauffé pour la config de l'EntityConfig
- Resolver passé dans EntityRecord (corrigé : était `None`, causait `channel closed`)
- Support checkpoint pour crash-recovery
- Services : conn, node_id_cache, embedder, embedding_dim, config, entity_configs, chunker_cache, has_sparse, has_dual + sparse/dual embedders

### 1.4 — Tests unitaires

**22 nouveaux tests** (489 → 511) :

**Dans `src/dataflow/record_nodes.rs`** (12 tests ChunkRecordNode + 3 tests EmbedNode) :
- `chunk_simple_entity_produces_chunks` — vérifie que des chunks sont produits
- `chunk_entity_names_correct` — `Product_Chunk` et `Product_CHUNKED_FROM`
- `chunk_has_title_from_parent` — `_title` = "Red Shoes"
- `chunk_has_embed_hash_empty` — `_embed_hash` = ""
- `chunk_has_text_hash` — `_text_hash` non vide
- `chunk_parent_field_set_correctly` — champs "description" et "details"
- `chunk_content_offset_multi_fields` — offset 0 pour description, len+2 pour details
- `chunk_unknown_entity_returns_empty` — entité inconnue → ([], [])
- `chunk_empty_content_returns_empty` — contenu vide → ([], [])
- `chunk_uuid_deterministic` — mêmes données → mêmes UUIDs
- `chunk_has_all_required_fields` — 17 champs obligatoires présents
- `embed_node_default_columns` — text_field="_text", embedding_col="embedding"
- `embed_node_custom_columns` — with_columns() fonctionne
- `embed_node_ports` — 2 inputs, 1 output

**Dans `src/catalog.rs`** (7 tests register/ingest) :
- `register_entity_stores_config` — is_simple_entity() true
- `register_entity_adds_to_catalog_entities` — config.entities contient l'entité
- `register_entity_content_fields` — content_fields() = ["description", "details"]
- `register_entity_before_init_fails` — NotInitialized
- `ingest_entities_before_init_fails` — NotInitialized
- `ingest_entities_unknown_entity_fails` — UnknownEntity
- `ingest_entities_empty_records_ok` — processed = 0
- `ingest_entities_returns_processed_count` — processed = 1

**Bug corrigé pendant les tests** : `ingest_entities` créait les EntityRecord avec `resolver: None` → le channel EntityRef était immédiatement fermé → `LinkRecordNode` échouait avec "channel closed". Fix : passer `resolver: Some(resolver)` pour que `InsertRecordNode` puisse résoudre la ref.

---

## État des nœuds (16 types)

| Nœud | Type | Nouveau? |
|---|---|---|
| InsertRecordNode | Générique | - |
| LinkRecordNode | Générique | - |
| FlushNode | Générique | - |
| **ChunkRecordNode** | **Simple** | **Nouveau** |
| **EmbedNode** | **Simple** | **Nouveau** |
| KBChunkRecordNode | KB | **Renommé** (ex-ChunkRecordNode) |
| KBEmbedNode | KB | - |
| KBGatherNode | KB | - |
| KBUpdateNode | KB | - |
| KBChunkNode | KB | - |
| KBSearchNode | Search | - |
| KBQuerySourceNode | Search | - |
| ComposeNode | Search | - |
| FetchRelatedNode | Search | - |
| CypherNode | Migration | - |
| ValidateNode | Migration | - |

---

## Fichiers modifiés cette session

| Fichier | Changements |
|---|---|
| `src/dataflow/record_nodes.rs` | Renommage ChunkRecordNode → KBChunkRecordNode, nouveau ChunkRecordNode simple, nouveau EmbedNode, 15 tests unitaires |
| `src/dataflow/node_factories.rs` | Renommage, 2 nouvelles factories (ChunkRecordNodeFactory, EmbedNodeFactory), compteur 14→16 |
| `src/dataflow/mod.rs` | Exports : +ChunkRecordNode, +EmbedNode |
| `src/catalog.rs` | Renommage, nouvelle méthode `ingest_entities()`, 7 tests unitaires, fix resolver |
| `src/dataflow/checkpoint.rs` | Renommage + 2 entrées test (ChunkRecordNode, EmbedNode) |

---

## Tasks actives

```
#173 ✅ Phase 1.1 — register_entity sur Catalog
#174 ✅ Phase 1.2 — EmbedNode + rename ChunkRecordNode → KBChunkRecordNode + nouveau ChunkRecordNode simple
#175 ✅ Phase 1.3 — ingest_entities sur Catalog
#176 ✅ Phase 1.4 — Tests unitaires
#177 ⏳ Phase 2.1 — SearchTarget + résolution noms de tables
#178 ⏳ Phase 2.2 — Refactor search() pour accepter SearchTarget (bloqué par 177)
#179 ⏳ Phase 2.3 — Tests search unifié + tests E2E (bloqué par 176+178)
#180 ⏳ Phase 3 — Nœuds search génériques + templates Mermaid (reporté)
```

## Prochaine étape

**Phase 2 — Unifier search()** : voir Doc 04 section "Phase 2 — Unifier search()" pour les détails.

Résumé : créer un struct `SearchTarget` qui encapsule les noms de tables (parent, chunk, relation, FTS fields, embedding col, sparse cols), puis modifier `catalog.search()` pour dispatcher entre KB et simple entity en construisant le bon `SearchTarget`. Les fonctions de recherche internes (`search_bm25_raw`, `resolve_bm25_to_chunks`, `search_vector`) prennent déjà des noms de tables en paramètre — le refactor est surtout au niveau du dispatch.
