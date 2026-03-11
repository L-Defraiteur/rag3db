# Doc 05 — Spécifications Phase 4 : API sync unifiée

Date : 11 mars 2026
Réf : Doc 04 (Phase 3 FAIT), Doc 02 (plan queue/drain unifié)

## Phase 3 : FAIT

Résumé : les 3 nœuds (DeleteRecordNode, UpdateRecordNode, RechunkDeleteNode) sont câblés dans `build_ingestion_graph()`. KBGatherNode lit depuis le service `pending_aggregates`. FlushResult étendu avec `update_results` / `delete_results`. 537 unit + 120 e2e = 657 tests, zéro régression.

---

## Phase 4 : Design — API sync unifiée

### Philosophie

> Le framework est là pour optimiser des opérations d'ingestion dans rag3db.
> L'utilisateur fait son CRUD comme il veut, puis appelle `drain()` pour tout flush.
> S'il veut un résultat immédiat → il appelle `drain()` juste après.

### Principe : tout sync sauf drain()

| Méthode | Avant (Phase 3) | Après (Phase 4) |
|---------|-----------------|-----------------|
| `create()` | sync, enqueue | inchangé |
| `link()` | sync, enqueue | inchangé |
| `update()` | **async**, exécution inline | **sync**, enqueue UpdateRecord |
| `delete()` | **async**, exécution inline | **sync**, enqueue DeleteRecord |
| `batch_update()` | **async**, exécution inline | **sync**, enqueue N UpdateRecords |
| `batch_delete()` | **async**, exécution inline | **sync**, enqueue N DeleteRecords |
| `drain()` | async, graph dataflow | inchangé (mais traite tout) |

### Signatures cibles

```rust
/// Enqueue un update. Exécuté au prochain drain().
pub fn update(
    &mut self,
    entity_name: &str,
    uuid: &str,
    data: BTreeMap<String, CypherValue>,
) -> Result<(), CatalogError>

/// Enqueue un delete. Exécuté au prochain drain().
pub fn delete(
    &mut self,
    entity_name: &str,
    uuid: &str,
) -> Result<(), CatalogError>

/// Enqueue N updates. Exécuté au prochain drain().
pub fn batch_update(
    &mut self,
    entity_name: &str,
    updates: Vec<(String, BTreeMap<String, CypherValue>)>,
) -> Result<(), CatalogError>

/// Enqueue N deletes. Exécuté au prochain drain().
pub fn batch_delete(
    &mut self,
    entity_name: &str,
    uuids: Vec<String>,
) -> Result<(), CatalogError>
```

### Logique d'enqueue

**update()** :
```rust
pub fn update(&mut self, entity_name: &str, uuid: &str, data: BTreeMap<String, CypherValue>) -> Result<(), CatalogError> {
    // 1. Validation: entity registered
    if !self.config.has_entity(entity_name) {
        return Err(CatalogError::EntityNotFound(entity_name.into()));
    }

    // 2. Compute content hash (même logique que l'ancien update inline)
    let new_content_hash = if let Some(ec) = self.entity_configs.get(entity_name) {
        let text = ec.build_content_text(&data);
        content_hash(&text)
    } else {
        content_hash("")  // KB entities sans entity_config
    };

    // 3. Enqueue
    self.pending.updates.push(UpdateRecord {
        entity_name: entity_name.to_string(),
        uuid: uuid.to_string(),
        data,
        new_content_hash,
    });

    Ok(())
}
```

**delete()** :
```rust
pub fn delete(&mut self, entity_name: &str, uuid: &str) -> Result<(), CatalogError> {
    if !self.config.has_entity(entity_name) {
        return Err(CatalogError::EntityNotFound(entity_name.into()));
    }

    self.pending.deletes.push(DeleteRecord {
        entity_name: entity_name.to_string(),
        uuid: uuid.to_string(),
    });

    Ok(())
}
```

**batch_update()** : boucle sur `updates`, appelle `self.update()` pour chaque.

**batch_delete()** : boucle sur `uuids`, appelle `self.delete()` pour chaque.

### Résultats

Les résultats reviennent via `FlushResult` après `drain()` :
```rust
let result = catalog.drain().await;
for ur in &result.update_results {
    println!("{}: status={:?}, reembedded={}", ur.uuid, ur.status, ur.reembedded);
}
for dr in &result.delete_results {
    println!("{}: chunks_deleted={}", dr.uuid, dr.chunks_deleted);
}
```

### Cas d'erreur

- Entity non enregistrée → erreur immédiate à l'enqueue
- UUID inexistant en DB → géré dans le nœud pendant drain (skip ou erreur partielle)
- Update sur entity déjà marquée pour delete dans le même batch → résolution dans `build_ingestion_graph()` (Phase 5)

### Fichiers à modifier

| Fichier | Modification |
|---------|-------------|
| `src/catalog.rs` | Réécrire update(), delete(), batch_update(), batch_delete() |
| `src/catalog.rs` | Supprimer helpers inline (build_content_text usage, rechunk_simple_entities calls) |
| `tests/e2e_native.rs` | 5 call sites → sync + drain() |
| `tests/e2e_phase0b.rs` | 4 call sites → sync + drain() |
| `tests/e2e_search.rs` | 3 call sites → sync + drain() |
| `tests/e2e_simple_entity.rs` | 5 call sites → sync + drain() |

### Tests e2e nouveaux (tests/e2e_drain_unified.rs)

```
test drain_delete_simple_entity        — create+drain, delete+drain, vérifier suppression
test drain_update_simple_entity        — create+drain, update+drain, vérifier rechunk
test drain_update_unchanged            — update même contenu → status=Unchanged
test drain_mixed_create_update_delete  — 3 creates + 1 update + 1 delete → 1 seul drain
test drain_batch_update                — batch_update 3 entities → drain
test drain_batch_delete                — batch_delete 2 entities → drain
test drain_delete_kb_entity            — delete KB contentFor → vérifie re-aggregation
test drain_update_kb_entity            — update KB contentFor → vérifie re-aggregation
```

### Ordre d'implémentation

1. Ajouter `enqueue_update()` / `enqueue_delete()` temporaires (sans toucher update/delete existants)
2. Écrire les tests e2e avec enqueue_* + drain() → valider le pipeline
3. Une fois validé : réécrire update/delete/batch_update/batch_delete en sync
4. Mettre à jour les 17 call sites dans les tests existants
5. Supprimer enqueue_* temporaires + dead code inline
6. Run complet : cargo test --lib + run_e2e.sh

### Code mort à supprimer (Phase 5)

Après validation, ces fonctions/blocs deviennent dead code :
- `rechunk_simple_entities()` — remplacé par le pipeline rechunk dans le DAG
- Logique inline de `update()` (MATCH SET, KB aggregation, rechunk)
- Logique inline de `delete()` (cascade delete, KB aggregation)
- `batch_update()` / `batch_delete()` inline
- Possiblement `build_content_text()` si extraite ailleurs
