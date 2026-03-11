# Doc 07 — Progression : Phase 3 FAIT + debug e2e_drain_unified

Date : 11 mars 2026

## Phase 3 : FAIT ✓

Commit `f323fe1c2` — 14 fichiers, 2936 insertions.

### Ce qui a été fait

**Phase 1** : Types + ports
- `UpdateRecord`, `DeleteRecord` dans records.rs
- `PortType::Updates`, `PortType::Deletes` dans port.rs
- `PendingWork` étendu (is_empty, total_count check les 5 champs)

**Phase 2** : 3 nœuds dataflow dans record_nodes.rs
- `DeleteRecordNode` — cascade delete (KB titleFor/contentFor + simple) + aggregates
- `UpdateRecordNode` — batch hash read, change detection, batch SET, rechunk output
- `RechunkDeleteNode` — supprime vieux chunks avant re-chunking

**Phase 3** : Intégration dans build_ingestion_graph()
- DAG : DeleteRecordNode → UpdateRecordNode → InsertRecordNode → LinkRecordNode
- Rechunk pipeline : RechunkDeleteNode → ChunkRecordNode → InsertRecordNode → LinkRecordNode → EmbedNode → FlushNode
- KB pipeline : KBGatherNode lit depuis shared service `pending_aggregates`
- FlushResult étendu avec `update_results` / `delete_results`
- Shared services : `Arc<Mutex<Vec<T>>>` pour résultats + aggregates

**API temporaire** : `enqueue_update()` / `enqueue_delete()` dans catalog.rs

**Tests** : 6 e2e dans `tests/e2e_drain_unified.rs`, 544 unit + 126 e2e = 670 tests, zéro régression.

### Bugs corrigés pendant le debug

1. **Checkpoint** (checkpoint.rs) — `PortType::Deletes`/`Updates` n'avaient pas de serialize/deserialize → ajoutées
2. **Deadlock rechunk** (record_nodes.rs) — UpdateRecordNode n'émettait pas `rechunk_entities` quand contenu inchangé → envoie toujours un batch (même vide)
3. **EmbedNode signaux** — hardcodé HYBRID au lieu de lire `entity_configs` par entity → résolution per-entity dans le nœud
4. **warm_chunker_cache()** — ne chauffait que les KB configs → ajouté simple entity configs
5. **FlushNode rechunk** — manquait dans le rechunk pipeline → ajouté

## Phase 4 : À faire

Réf : doc 05. Décision simplifiée vs le plan original :

### API finale (sync, comme create/link)
```rust
pub fn update(&mut self, entity_name: &str, uuid: &str, data: BTreeMap<String, CypherValue>) -> Result<(), CatalogError>
pub fn delete(&mut self, entity_name: &str, uuid: &str) -> Result<(), CatalogError>
```

### Suppressions
- `batch_update()` / `batch_delete()` — plus nécessaires (le graph batch tout dans un seul drain)
- `enqueue_update()` / `enqueue_delete()` — remplacés par update()/delete()
- Code inline des anciens update()/delete() (queries + rechunk + KB logic)

### Call sites à adapter (~17)
- `tests/e2e_native.rs` — update/delete async → sync + drain()
- `tests/e2e_phase0b.rs` — idem
- `tests/e2e_search.rs` — idem
- `tests/e2e_simple_entity.rs` — idem

### Fichiers à modifier
| Fichier | Modification |
|---------|-------------|
| `src/catalog.rs` | Réécrire update(), delete(). Supprimer batch_*, enqueue_*, code inline |
| `tests/e2e_native.rs` | ~5 call sites |
| `tests/e2e_phase0b.rs` | ~4 call sites |
| `tests/e2e_search.rs` | ~3 call sites |
| `tests/e2e_simple_entity.rs` | ~5 call sites |
