# Doc 15 — Session Recap: E2E Phase 0b Tests + Bug Fixes

**Date**: 3 mars 2026
**Branche**: `feature/kb-index-architecture`
**Fichier test créé**: `extension/rag3weaver/tests/e2e_phase0b.rs`

---

## Résumé

Écriture de 13 tests E2E pour Phase 0b (cross-entity KB, AggregateProcessor, SOURCED rels, _content_offset, title truncation, delete/update contentFor-only, link incremental). Découverte et correction d'un bug critique de collision UUID dans les chunks.

---

## Tests E2E créés (e2e_phase0b.rs)

Config de test : TreeKB (multi-entity, FULLTEXT) + FileKB (single-entity, HYBRID).
Entités : Directory (titleFor TreeKB), File (titleFor FileKB, contentFor TreeKB).
Relation : HAS_FILE (Directory → File).

| # | Test | Statut | Description |
|---|------|--------|-------------|
| 1 | `phase0b_ingest_and_schema` | **PASS** | Ingestion complète, vérification schema, KB metadata, Index entries, chunks, SOURCED rels |
| 2 | `phase0b_bm25_search_multi_entity` | **FAIL** | BM25 search sur TreeKB — raw Lucivy fonctionne, mais `search_bm25_chunked()` retourne 0 |
| 3 | `phase0b_bm25_highlight_chunk_single_entity` | **FAIL** | FileKB search crash: vector index name mismatch (`FileKB_Index_Chunk_FileKB_vec` vs `FileKB_Index_Chunk_vec`) |
| 4 | `phase0b_vector_chunk_to_source_entity` | **PASS** | Chunks FileKB liés aux bons File via SOURCED |
| 5 | `phase0b_content_offset_arithmetic` | **PASS** | `_content_offset` arithmétiquement correct (global start/end = offset + local) |
| 6 | `phase0b_delete_content_for_only` | **PASS** | Delete File → re-aggregate TreeKB → contenu File disparu, hash changé, SOURCED supprimés |
| 7 | `phase0b_update_content_for_only` | **FAIL** | `drain after update: processed=0` — update() n'enqueue pas d'AggregateOp pour contentFor-only |
| 8 | `phase0b_title_truncation` | **PASS** | title_max_chars=20 → titre tronqué à 20 chars, chunks OK |
| 9 | `phase0b_sourced_rels_multi_entity` | **PASS** | SOURCED rels correctes par entité (Directory, Button.tsx, Modal.tsx) |
| 10 | `phase0b_aggregate_skip_unchanged` | **PASS** | Re-drain avec même contenu → hash inchangé, chunks identiques |
| 11 | `phase0b_link_incremental_aggregate` | **PASS** | link() après drain → AggregateOp → contenu File intégré dans TreeKB |
| 12 | `phase0b_delete_one_of_multiple_files` | **PASS** | Delete alpha.ts → beta.ts reste, search cohérent |
| 13 | `phase0b_debug_trace_pipeline` | **PASS** | Test debug avec subscribe_queue(), dump DB complet, raw Lucivy query |

**Score : 10/13 pass**

---

## Bug corrigé : Collision UUID chunks multi-entity

### Problème
`chunk_uuid(parent_uuid, field_name, index)` — quand 2 entités différentes contribuent le même `field_name` (ex: Directory.`absolute_path` et File.`absolute_path`), les chunks obtenaient le même UUID → `BatchFailed` duplicate primary key → **0 chunks créés** pour TOUS les KBs (même batch).

### Fix (catalog.rs:2143-2147)
```rust
// AVANT
let c_uuid = chunk_uuid(&agg.index_entry_uuid, &source.field_name, chunk.index);

// APRÈS
let source_key = format!("{}:{}", source.entity_uuid, source.field_name);
let c_uuid = chunk_uuid(&agg.index_entry_uuid, &source_key, chunk.index);
```

Inclure `entity_uuid` dans la clé garantit l'unicité même avec des field names identiques.

---

## 3 bugs restants à investiguer

### Bug A : `search_bm25_chunked` retourne 0 sur TreeKB
- **Symptôme** : Raw `QUERY_LUCIVY_INDEX` retourne 1 résultat (score=0.396), mais `catalog.search()` retourne 0.
- **Cause probable** : `search_bm25_chunked()` utilise le mode `Contains` par défaut (NgramContainsQuery), pas `Parse`. La query "auth" en mode Contains cherche un substring continu, mais le test `phase0b_bm25_search_multi_entity` fait une recherche classique.
- **Piste** : Le debug test utilise BM25Mode::default (Contains) — tester avec `BM25Mode::Parse` ou `BM25Mode::ContainsSplit`, OU le raw Lucivy retourne bien un résultat mais `resolve_and_enrich_chunked` échoue car le chunk entity name est construit différemment.
- **Autre piste** : Le mode Contains génère `{"type":"contains","field":"_title","value":"auth"}` qui fait du fuzzy ngram, peut-être que ça ne matche pas "auth.ts" comme attendu.

### Bug B : Vector index name mismatch sur FileKB
- **Symptôme** : `Table FileKB_Index_Chunk doesn't have an index with name FileKB_Index_Chunk_FileKB_vec`
- **Analyse** : Le schema crée l'index avec nom `{kb}_Index_Chunk_vec` (e.g. `FileKB_Index_Chunk_vec`), mais le code search construit un nom différent `{chunk_entity}_{kb}_vec` ou similaire.
- **Fix nécessaire** : Aligner le nom d'index dans `search_vector()` avec celui généré par `generate_vector_index_ddl()` dans schema.rs. Vérifier la construction du nom dans search.rs ligne ~1248 (search_vector).

### Bug C : update() contentFor-only ne déclenche pas d'AggregateOp
- **Symptôme** : `drain after update: processed=0` — aucun AggregateOp enqueué après update d'un File (contentFor-only pour TreeKB).
- **Cause probable** : Le code update() (catalog.rs ~719-754) devrait détecter que File est contentFor-only pour TreeKB, trouver le Directory lié via HAS_FILE, et enqueuer un AggregateOp. Soit la condition `content_changed` est false (hash identique car on update name+absolute_path qui ne sont peut-être pas dans le content hash), soit `find_relation_to_entity()` ne trouve pas la relation.
- **Piste** : Vérifier que `build_content_text()` inclut les champs contentFor dans le hash. Sinon le hash ne change pas → `content_changed = false` → pas d'AggregateOp.

---

## Infrastructure de debug ajoutée

Le test `phase0b_debug_trace_pipeline` démontre un pattern réutilisable :
1. `catalog.subscribe_queue()` → écouter les QueueEvent (Enqueued, ProcessingBatch, BatchCompleted/Failed, Injected)
2. Dump DB état après drain (query raw sur Index, Chunk, SOURCED rels)
3. Raw Lucivy query pour isoler FTS vs search code
4. Très efficace pour diagnostiquer les problèmes de pipeline

---

## Fichiers modifiés

| Fichier | Changement |
|---------|-----------|
| `extension/rag3weaver/src/catalog.rs:2143-2147` | Fix chunk UUID collision (include entity_uuid) |
| `extension/rag3weaver/tests/e2e_phase0b.rs` | **NOUVEAU** — 13 tests E2E Phase 0b |

---

## Commandes pour reproduire

```bash
# Lancer tous les tests Phase 0b
cd packages/rag3db/extension/rag3weaver
./run_e2e.sh --test e2e_phase0b

# Ou manuellement
RAG3DB_SHARED=1 \
RAG3DB_LIBRARY_DIR=../../build/native-test/src \
RAG3DB_INCLUDE_DIR=../../build/native-test/src \
LD_LIBRARY_PATH=../../build/native-test/src \
RAG3DB_ROOT=../.. \
cargo test --features rag3db-native --test e2e_phase0b -- --ignored --nocapture

# Un seul test
cargo test ... -- --ignored --nocapture phase0b_debug_trace
```

---

## Prochaines étapes

1. **Fixer Bug A** : Investiguer `search_bm25_chunked` — probablement un problème de BM25Mode ou de résolution chunks
2. **Fixer Bug B** : Aligner le nom d'index vector entre schema.rs et search.rs
3. **Fixer Bug C** : update() contentFor-only — vérifier content_changed et find_relation_to_entity
4. Relancer les 13 tests → objectif 13/13
5. Vérifier que les tests e2e_search.rs existants ne régressent pas (le chunk UUID change pour tous les KBs)
