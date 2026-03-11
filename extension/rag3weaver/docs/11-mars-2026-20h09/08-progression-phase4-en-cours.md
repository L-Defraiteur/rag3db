# Doc 08 — Progression Phase 4 : en cours

Date : 11 mars 2026

## Ce qui est FAIT dans cette session

### Réécriture update()/delete() en sync ✓
- `update()` = sync, enqueue dans `pending.updates` (anciennement `enqueue_update()`)
- `delete()` = sync, enqueue dans `pending.deletes` (anciennement `enqueue_delete()`)
- Signatures : `pub fn update(&mut self, entity_name, uuid, data) -> Result<(), CatalogError>`
- Signatures : `pub fn delete(&mut self, entity_name, uuid) -> Result<(), CatalogError>`

### Suppression dead code ✓
- ~1050 lignes supprimées de catalog.rs :
  - Ancien `update()` async inline (lines 981-1151)
  - Ancien `delete()` async inline (lines 1153-1319)
  - `batch_delete()` (lines 1325-1562)
  - `batch_update()` (lines 1566-1847)
  - `rechunk_simple_entities()` (lines 1854-2037)
  - `find_relation_to_entity()` (dead helper doublon de record_nodes.rs)
- `enqueue_update()`/`enqueue_delete()` renommés en `update()`/`delete()`
- Commentaires records.rs mis à jour

### Tests unitaires adaptés ✓ (544 passent)
- `update_not_found` → `update_enqueues_sync` (vérifie que pending.updates est rempli)
- `delete_succeeds_with_mock` → `delete_enqueues_sync` (vérifie pending.deletes)

### Tests e2e adaptés (partiellement)
- `e2e_drain_unified.rs` : `enqueue_*` → `update`/`delete` ✓
- `e2e_simple_entity.rs` : 5 call sites adaptés (sync + drain) ✓
- `e2e_native.rs` : 5 call sites adaptés ✓
- `e2e_phase0b.rs` : 4 call sites adaptés ✓
- `e2e_search.rs` : 3 call sites adaptés ✓

### Résultat des tests
- 544 unit tests : ✓
- 125/126 e2e : ✓
- 1 échec : `phase0_error_cases` dans e2e_search.rs

## Ce qui reste à faire

### 1. Fixer phase0_error_cases (e2e_search.rs)

Erreur de compilation (pas un échec runtime) :
```
error[E0433]: ...
```
Le test a été réécrit par l'agent mais il y a une erreur d'import ou de syntaxe. Il faut :
- Relire le test autour de la ligne 598-615
- Fixer l'erreur de compil (probablement un import manquant pour `UpdateStatus` ou `QueryParam`)
- Le test vérifie qu'un update d'un uuid inexistant est un no-op silencieux

**Comportement actuel** : update("fake-uuid") → enqueue OK → drain OK (MATCH trouve 0 rows, SET s'exécute sur rien) → UpdateResult avec status=Updated (car pas d'ancien hash → considéré "changed"). L'entité n'est pas créée.

### 2. Event bus comme service (suggestion pour Phase 5)

Idée : passer `EventBus` (ou un `Sender<CatalogEvent>`) comme service dans le ServiceRegistry, pour que les nœuds dataflow puissent émettre des events.

**Motivation immédiate** :
- UpdateRecordNode/DeleteRecordNode pourraient émettre un warning quand un uuid n'existe pas en DB
- Plus propre que de juste ignorer silencieusement

**Motivation future** :
- Émettre `CatalogEvent::EntityUpdated` / `EntityDeleted` depuis les nœuds au lieu de catalog.rs
- Le doc 02 prévoyait déjà ça pour Phase 5

**Implémentation** :
```rust
// Dans build_ingestion_graph() :
services.register::<async_broadcast::Sender<CatalogEvent>>(
    "event_bus", Arc::new(self.event_bus.sender()));

// Dans les nœuds :
if let Some(bus) = ctx.service::<async_broadcast::Sender<CatalogEvent>>("event_bus") {
    bus.broadcast(CatalogEvent::Warning { ... }).await;
}
```

### 3. Nettoyage restant
- Vérifier que `batch_update`/`batch_delete` ne sont référencés nulle part (API publique, WASM FFI, docs)
- Confirmer que le WASM FFI n'expose pas update/delete (vérifié : non)
- Run complet final : `cargo test --lib` + `./run_e2e.sh`

## Fichiers modifiés (non committés)
| Fichier | Statut |
|---------|--------|
| `src/catalog.rs` | Réécrit update/delete, supprimé ~1050 lignes dead code |
| `src/records.rs` | Commentaires mis à jour |
| `tests/e2e_drain_unified.rs` | enqueue_* → update/delete |
| `tests/e2e_simple_entity.rs` | 5 call sites sync + drain |
| `tests/e2e_native.rs` | 5 call sites sync + drain |
| `tests/e2e_phase0b.rs` | 4 call sites sync |
| `tests/e2e_search.rs` | 3 call sites sync + drain (1 erreur compil à fixer) |

## API finale (Phase 4 terminée)
```rust
// Toutes sync, enqueue dans PendingWork :
pub fn create(&mut self, entity_name, data) -> Result<EntityRef, CatalogError>
pub fn link(&mut self, rel_name, from, to, data) -> Result<RelationRef, CatalogError>
pub fn update(&mut self, entity_name, uuid, data) -> Result<(), CatalogError>
pub fn delete(&mut self, entity_name, uuid) -> Result<(), CatalogError>

// Async, exécute tout le DAG dataflow optimisé :
pub async fn drain(&mut self) -> FlushResult

// FlushResult contient :
//   processed, failed, update_results: Vec<UpdateResult>, delete_results: Vec<DeleteResult>
```

Plus de `batch_update()`/`batch_delete()` — l'utilisateur fait N appels sync + 1 `drain()`.
