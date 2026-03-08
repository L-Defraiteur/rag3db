# Doc 07 — Rapport E2E tests + bugfixes pipeline simple

Date : 8 mars 2026
Réf : Doc 05 (Phase 1), Doc 06 (Phase 2)

## Résumé

Tests E2E ajoutés pour le pipeline simple entity. 2 bugs trouvés et corrigés. Tous les tests passent (521 unit + 10 simple entity E2E + 37 KB E2E + 10 result_mode E2E).

---

## Ce qui a été fait cette session

### 7.1 — Export EntityConfig/SimpleFieldDef depuis lib.rs

Ajouté `EntityConfig` et `SimpleFieldDef` aux re-exports de `lib.rs` pour simplifier l'usage dans les tests :
```rust
pub use config::{CatalogConfig, EntityConfig, SimpleFieldDef};
```

### 7.2 — Fichier e2e_simple_entity.rs (10 tests)

**Fichier** : `tests/e2e_simple_entity.rs`

Tests couvrant le pipeline complet `register_entity → ingest_entities → search` :

| Test | Signal | Embedder | Ce qui est testé |
|------|--------|----------|------------------|
| `simple_register_and_ingest` | - | Mock | register → ingest → counts, chunks, CHUNKED_FROM, events |
| `simple_ingest_unknown_entity_fails` | - | Mock | Erreur si entité inconnue |
| `simple_register_duplicate_fails` | - | Mock | Erreur si double register |
| `simple_bm25_search_finds_results` | BM25 | Mock | FTS search, meta.target="Product" |
| `simple_bm25_no_results_for_nonsense` | BM25 | Mock | 0 résultats pour requête absurde |
| `simple_bm25_highlights_resolve_to_correct_chunks` | BM25 | Mock | Highlights multi-champs → bon chunk |
| `simple_vector_minilm_search` | SEMANTIC | MiniLM | Vector search avec vrais embeddings |
| `simple_hybrid_bgem3_search` | HYBRID | BGE-M3 | BM25 + Vector fusion |
| `simple_sparse_bgem3_search` | HYBRID+SPARSE | BGE-M3 | Sparse vector search |
| `simple_multiple_ingestions` | BM25 | Mock | 2 batches successifs + search across both |

**Infrastructure** :
- `catalog.subscribe()` utilisé pour capturer les `CatalogEvent::Error` pendant l'ingestion (debug)
- `execute_raw()` pour vérifier l'état DB (counts, tables, directions de relation)
- Helpers : `make_product_config()`, `make_product()`, `setup_simple_catalog()`

### 7.3 — Bug fix : direction RelationRecord dans ChunkRecordNode

**Fichier** : `src/dataflow/record_nodes.rs`

**Problème** : Le DDL crée `CREATE REL TABLE Product_CHUNKED_FROM(FROM Product_Chunk TO Product)`, mais le `RelationRecord` mettait `from: parent (entity_ref)` et `to: chunk (c_uuid)` — inversé.

**Résultat** : 0 relations créées (le MERGE échouait silencieusement car from/to ne matchaient pas le schéma).

**Fix** :
```rust
// Avant (FAUX)
from: RefOrUuid::Ref(entity_ref.clone()),  // parent
to: RefOrUuid::Uuid(c_uuid),              // chunk

// Après (CORRECT)
from: RefOrUuid::Uuid(c_uuid),            // chunk (FROM)
to: RefOrUuid::Ref(entity_ref.clone()),   // parent (TO)
```

### 7.4 — Bug fix : BM25 highlights per-field pour simple entities

**Fichier** : `src/search.rs` (dans `search_bm25_chunked`)

**Problème** : Le matching highlights → chunks ne regardait que `highlights.get("_content")` (pattern KB). Pour les simple entities, les highlights viennent sur les vrais noms de champs (`"description"`, `"details"`), pas `"_content"`.

**Résultat** : `chunks_matched=0` systématiquement, le search retournait des résultats sans chunk résolu.

**Fix** : Ajout d'un second path de matching par `chunk.parent_field` :
```rust
// KB mode: "_content" highlights use global offsets
if let Some(hl_offsets) = highlights.get("_content") { ... }

// Simple entity mode: per-field highlights matched by parent_field
if !chunk.parent_field.is_empty() {
    if let Some(hl_offsets) = highlights.get(&chunk.parent_field) {
        // Compare directement start_char/end_char (per-field offsets)
    }
}
```

**Vérifié** :
- `"borrow checker"` (description only) → chunks_matched=1, bon chunk
- `"kubernetes"` (details only) → chunks_matched=1, bon chunk
- `"performance"` (both fields, Detailed) → chunks_matched=2, les 2 chunks

---

## Fichiers modifiés cette session

| Fichier | Changements |
|---|---|
| `src/lib.rs` | Export EntityConfig, SimpleFieldDef |
| `src/dataflow/record_nodes.rs` | Fix from/to dans ChunkRecordNode RelationRecord |
| `src/search.rs` | Fix highlight matching per-field pour simple entities |
| `tests/e2e_simple_entity.rs` | **NOUVEAU** — 10 tests E2E pipeline simple |

---

## Non-régression

| Suite | Tests | Résultat |
|---|---|---|
| Unit tests (`cargo test --lib`) | 521 | 521 OK |
| `e2e_simple_entity` | 10 | 10 OK |
| `e2e_search` (KB) | 37 | 37 OK |
| `e2e_result_mode` (KB highlights) | 10 | 10 OK |

---

## Prochaine étape suggérée

**Tests highlights avec texte complexe (KB + simple entity)** : Les tests E2E actuels pour les KB n'utilisent que du texte court (~1 phrase par champ). On n'a jamais testé la résolution highlight→chunk sur du texte long multi-champs qui génère plusieurs chunks sur un même champ (chunking réel avec overlap). Il faudrait :

1. **KB** : créer un test E2E avec un document ayant un `body` de ~2000+ chars (pour forcer 2+ chunks par champ) et vérifier que les highlights BM25 résolvent vers le bon chunk via `_content_offset`
2. **Simple entity** : idem avec 2+ champs `is_content` longs, vérifier le matching per-field + content_offset
3. Vérifier le mode `Detailed` avec ces textes longs (plusieurs chunks attributed à un même résultat)

Cela validerait la résolution highlight→chunk dans des conditions réalistes, pas juste avec du texte court qui tient dans un seul chunk.

---

## Tasks

```
#173 ✅ Phase 1.1 — register_entity sur Catalog
#174 ✅ Phase 1.2 — EmbedNode + rename ChunkRecordNode
#175 ✅ Phase 1.3 — ingest_entities sur Catalog
#176 ✅ Phase 1.4 — Tests unitaires
#177 ✅ Phase 2.1 — SearchTarget + résolution noms de tables
#178 ✅ Phase 2.2 — Refactor search() pour SearchTarget
#179 ✅ Phase 2.3 — Tests search unifié
#181 ✅ E2E tests simple entity + bugfixes
#180 ⏳ Phase 3 — Nœuds search génériques + templates Mermaid (reporté)
```
