# Doc 06 — Rapport d'implémentation Phase 2 (search unifié)

Date : 8 mars 2026
Réf : Doc 04 (contexte), Doc 05 (rapport Phase 1)

## Résumé

Phase 2 complète : `search()` accepte maintenant KB et simple entities via `SearchTarget`. 521 tests (511 → 521, +10 nouveaux). Tous passent.

---

## Ce qui a été fait cette session

### 2.1 — SearchTarget struct + resolve_search_target()

**Fichier** : `src/search.rs`

**Struct SearchTarget** :
```rust
pub struct SearchTarget {
    pub name: String,              // KB name ou entity name
    pub parent_table: String,      // {KB}_Index ou {Entity}
    pub chunk_table: String,       // {KB}_Index_Chunk ou {Entity}_Chunk
    pub chunk_rel: String,         // {KB}_Index_HAS_CHUNK ou {Entity}_CHUNKED_FROM
    pub chunk_rel_fwd: bool,       // true = parent→chunk (KB), false = chunk→parent (simple)
    pub bm25_fields: Vec<String>,  // ["_title","_content"] ou content_fields()
    pub enrich_fields: Vec<String>,
    pub default_signals: SearchSignals,
    pub default_fusion: FusionConfig,
    pub has_source_refs: bool,     // KB chunks ont _source_entity/_source_uuid
    pub filter_indirection: Option<(String, String)>,  // KB: (title_entity, in_rel)
}
```

**Helpers Cypher** :
- `parent_to_chunk_match("n", "c")` → KB: `MATCH (n:X_Index)-[:X_Index_HAS_CHUNK]->(c:X_Index_Chunk)`, Simple: `MATCH (n:Product)<-[:Product_CHUNKED_FROM]-(c:Product_Chunk)`
- `chunk_to_parent_match("p", "c")` → KB: `MATCH (p:X_Index)-[:X_Index_HAS_CHUNK]->(c)`, Simple: `MATCH (c)-[:Product_CHUNKED_FROM]->(p:Product)`

**Fichier** : `src/catalog.rs`

**`resolve_search_target(name)`** : cherche dans `kb_metadata` (→ KB target) puis `entity_configs` (→ simple target). Erreur `UnknownKB` si ni l'un ni l'autre.

**Renommage** : `SearchMeta.kb` → `SearchMeta.target`

### 2.2 — Refactor search() pour SearchTarget

**Fonctions modifiées dans search.rs** :

| Fonction | Avant | Après |
|---|---|---|
| `resolve_and_enrich_chunked()` | `entity, chunk_entity` | `&SearchTarget` |
| `resolve_vector_chunks()` | `chunk_entity, parent_entity` | `&SearchTarget` |
| `search_bm25_chunked()` | `entity, chunk_entity` | `&SearchTarget` |

**Changements clés** :
- Pattern rel dynamique (plus de hardcode `{entity}_HAS_CHUNK`)
- Colonnes `_source_entity`, `_source_uuid` conditionnelles (`target.has_source_refs`)
- Offsets colonnes ajustés dynamiquement (`base_chunk_cols + source_cols`)

**`catalog.search()` refactoré** :
- Param `kb_name` → `name`
- Début : `resolve_search_target(name)?` au lieu de `kb_metadata.get(kb_name)`
- Signals : `options.signals.unwrap_or(target.default_signals)` au lieu de `kb_config.signals`
- Filtres BM25 : dispatch via `target.filter_indirection` (KB: title entity JOIN, simple: filtre direct)
- Fusion : `target.default_fusion` au lieu de `kb_config.fusion_config()`
- Source resolve : conditionnel `target.has_source_refs`
- Enrich fields, bm25_fields : depuis `target`

### 2.3 — Tests

**10 nouveaux tests** (511 → 521) :

**Dans catalog.rs** (8 tests) :
- `resolve_search_target_kb` — KB → parent_table = "main_Index", chunk_rel_fwd = true, has_source_refs = true
- `resolve_search_target_simple_entity` — Simple → parent_table = "Product", chunk_rel = "Product_CHUNKED_FROM", chunk_rel_fwd = false
- `resolve_search_target_unknown_fails` — UnknownKB
- `search_target_parent_to_chunk_match_kb` — pattern Cypher KB correct
- `search_target_parent_to_chunk_match_simple` — pattern Cypher simple correct (direction inversée)
- `search_target_chunk_to_parent_match_kb` — pattern Cypher KB correct
- `search_target_chunk_to_parent_match_simple` — pattern Cypher simple correct
- `search_target_signals_default` — HYBRID (bm25 + vector)

**Dans search.rs** (2 tests E2E smoke) :
- `catalog_search_simple_entity_smoke` — register → search("Product", ...) → OK, meta.target = "Product"
- `catalog_search_simple_entity_with_ingest_smoke` — register → ingest → search → OK

---

## Fichiers modifiés cette session

| Fichier | Changements |
|---|---|
| `src/search.rs` | SearchTarget struct + helpers, SearchMeta.kb→target, resolve_and_enrich_chunked/resolve_vector_chunks/search_bm25_chunked refactorés pour SearchTarget, 2 tests E2E |
| `src/catalog.rs` | resolve_search_target(), search() refactoré pour SearchTarget, filtres BM25 dispatchés, 8 tests unitaires |
| `docs/.../04-contexte-implementation-suite.md` | Tasks 174-176 marquées ✅ |

---

## Résolution des différences KB vs Simple

| Aspect | KB | Simple |
|---|---|---|
| parent_table | `{KB}_Index` | `{Entity}` |
| chunk_table | `{KB}_Index_Chunk` | `{Entity}_Chunk` |
| chunk_rel | `{KB}_Index_HAS_CHUNK` (fwd) | `{Entity}_CHUNKED_FROM` (rev) |
| bm25_fields | `["_title", "_content"]` | `content_fields()` |
| filter BM25 | Via title_entity JOIN | Direct sur entity |
| _source_entity/_uuid | Oui (sur chunks) | Non |
| Source resolve | Oui | Non |
| Signals default | `KBConfig.signals` | `EntityConfig.signals` |
| Fusion default | `KBConfig.fusion_config()` | `FusionConfig::default()` |

---

## Tasks

```
#173 ✅ Phase 1.1 — register_entity sur Catalog
#174 ✅ Phase 1.2 — EmbedNode + rename ChunkRecordNode → KBChunkRecordNode + nouveau ChunkRecordNode simple
#175 ✅ Phase 1.3 — ingest_entities sur Catalog
#176 ✅ Phase 1.4 — Tests unitaires
#177 ✅ Phase 2.1 — SearchTarget + résolution noms de tables
#178 ✅ Phase 2.2 — Refactor search() pour accepter SearchTarget
#179 ✅ Phase 2.3 — Tests search unifié + tests E2E smoke
#180 ⏳ Phase 3 — Nœuds search génériques + templates Mermaid (reporté)
```

## Prochaine étape

**Phase 3 — Nœuds search génériques + templates Mermaid** (reporté) : créer des nœuds dataflow search génériques (non KB-spécifiques) et les templates Mermaid correspondants. Pas prioritaire tant que le pipeline simple fonctionne via `ingest_entities` + `search()`.

**Alternative immédiate** : exposer le pipeline simple via l'API Node.js/WASM (register_entity, ingest_entities, search avec nom d'entité).
