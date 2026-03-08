# Doc 12 — Rapport de progression : batch update/delete + CRUD simple entities

Date : 8 mars 2026
Réf : Doc 10 (plan CRUD), Doc 11 (analyse batch)

## Résumé

Implémentation en cours des fixes CRUD simple entities + batch_update/batch_delete.

## Ce qui est FAIT

### 1. Fix `build_content_text()` — bug hash corrigé ✅

Le hash entre `ingest_entities()` et `update()` était incohérent :
- `ingest_entities()` : utilisait `content_fields()` (is_content=true) avec `"\n\n"`
- `build_content_text()` : utilisait TOUS les champs Text/String avec `"|"`

Fix : `build_content_text()` utilise maintenant `content_fields()` pour les simple entities, garde l'ancien comportement pour les KB entities.

### 2. `rechunk_simple_entities()` helper ✅

Nouvelle méthode privée sur `Catalog` (après `delete()`). Batch re-chunk + re-embed :
1. Batch-delete vieux chunks via UNWIND `_parent_uuid`
2. Build `Vec<EntityRecord>` avec `EntityRef::pre_resolved()`
3. Warm chunker cache
4. Build mini dataflow graph : `ChunkRecordNode → InsertRecordNode → LinkRecordNode → EmbedNode → FlushNode`
5. Register mêmes services que `ingest_entities()`
6. Execute, return per-item `(deleted, created)`

### 3. Fix `delete()` pour simple entities ✅

Avant `DETACH DELETE n`, ajout cascade-delete chunks :
```rust
if is_simple {
    MATCH (c:{Entity}_Chunk {_parent_uuid: $uuid}) DETACH DELETE c
}
```
+ flush FTS après deletion.

### 4. Fix `update()` pour simple entities ✅

Après le KB loop, dans `if content_changed` :
- `chunks_deleted` et `chunks_created` rendus mutables
- Si simple entity + content changed → `self.get()` + `rechunk_simple_entities()`
- Set `reembedded = true`

### 5. `batch_delete()` ✅

Nouvelle méthode publique :
```rust
pub async fn batch_delete(&mut self, entity_name: &str, uuids: Vec<String>) -> Result<Vec<DeleteResult>>
```
- KB titleFor : batch delete index chunks + index entries via UNWIND
- KB contentFor : batch delete SOURCED chunks, batch find title entities, batch enqueue AggregateRecords
- Simple entity : batch delete chunks via UNWIND `_parent_uuid`
- Batch delete entities via UNWIND
- Batch remove from node_id_cache
- Flush FTS
- Return per-uuid DeleteResult

### 6. `batch_update()` ✅

Nouvelle méthode publique :
```rust
pub async fn batch_update(&mut self, entity_name: &str, updates: Vec<(String, BTreeMap<String, CypherValue>)>) -> Result<Vec<UpdateResult>>
```
- Batch compute new hashes
- Batch-read old hashes via UNWIND
- Detect content changes per item
- Batch SET fields via UNWIND
- KB : batch enqueue AggregateRecords
- Simple : batch `get_many()` + `rechunk_simple_entities()` (un seul appel GPU)
- Return per-uuid UpdateResult

### 7. Compilation + tests unitaires ✅

- `cargo check` : compile sans erreurs ni warnings
- 539 tests unitaires passent

### 8. E2E tests — 5 tests écrits ✅

Dans `tests/e2e_simple_entity.rs` :
1. `simple_delete_removes_chunks`
2. `simple_update_refreshes_chunks`
3. `simple_update_unchanged_no_rechunk`
4. `simple_batch_delete_multiple`
5. `simple_batch_update_multiple`

Helpers ajoutés : `query_count()`, `get_product_uuids()`

## Ce qui ÉCHOUE — à investiguer

### 4 tests E2E échouent

Les 10 tests existants passent, mais les 4 nouveaux CRUD tests échouent.

**Symptôme principal** : après `delete()` d'un produit, les **autres** produits ne sont plus trouvables par BM25. L'assertion qui échoue :
```
remaining product should still be searchable
```

**Debug observé** :
- Le delete fonctionne côté DB : entity supprimée, chunks supprimés (chunks_deleted=2)
- Les produits restants sont bien en DB (2 products, 4 chunks)
- Mais BM25 search retourne 0 résultats pour les produits restants

**Hypothèse** : le `FLUSH_LUCIVY_INDEX('Product')` après deletion ne suffit pas. Possible que :
1. Le FTS index Lucivy sur la table `Product` est corrompu/invalidé après un `DETACH DELETE`
2. Le mécanisme `flushIfDirty()` de Tantivy ne gère pas correctement les deletes depuis la couche graph rag3db
3. Il faudrait peut-être un `CALL DROP_LUCIVY_INDEX('Product')` + `CALL CREATE_LUCIVY_INDEX(...)` (rebuild)
4. Ou bien le problème est côté Tantivy internal state — le reader n'est pas correctement rechargé après un delete+flush

**Piste à explorer en premier** : tester manuellement dans un test si `QUERY_TANTIVY_INDEX('Product', 'beta')` retourne des résultats après le delete+flush. Ça isolera si le problème est dans le FTS Lucivy ou dans la couche search rag3weaver.

### Le test `simple_update_unchanged_no_rechunk` — probablement OK

Ce test pourrait passer si le bug FTS est résolu, car il ne fait pas de BM25 search après l'update.

## État des fichiers modifiés

| Fichier | Modifications |
|---------|--------------|
| `src/catalog.rs` | `build_content_text()` fixé, `delete()` + chunk cascade + FTS flush, `update()` + rechunk simple, `rechunk_simple_entities()` helper, `batch_delete()`, `batch_update()` |
| `tests/e2e_simple_entity.rs` | Import UpdateStatus, helpers query_count/get_product_uuids, 5 nouveaux tests |

## Prochaines étapes

1. **Investiguer le bug FTS post-delete** — tester `QUERY_TANTIVY_INDEX` directement en Cypher dans un test
2. Si c'est un bug Lucivy : fixer dans l'extension C++ `tantivy_fts`
3. Si c'est un bug de flush timing : ajuster l'ordre flush/search
4. Faire passer les 5 E2E tests
5. Non-régression complète (`./run_e2e.sh`)

## Tasks

```
#195 ✅ Fix build_content_text() hash consistency
#196 ✅ Add rechunk_simple_entities() helper
#197 ✅ Fix delete() — cascade-delete chunks
#198 ✅ Fix update() — rechunk on content change
#199 ✅ Implement batch_delete()
#200 ✅ Implement batch_update()
#201 🔧 E2E tests — écrits mais 4/5 échouent (bug FTS post-delete)
```
