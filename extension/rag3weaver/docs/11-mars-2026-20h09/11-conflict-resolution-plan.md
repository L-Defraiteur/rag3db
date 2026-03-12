# Doc 11 — Conflict Resolution : plan d'implémentation

Date : 12 mars 2026

## Objectif

Résoudre les conflits entre opérations dans `PendingWork` avant/pendant l'exécution du dataflow graph d'ingestion.

## Décisions

### 1. Delete + Update même UUID → delete gagne

**Où** : queue level, dans `build_ingestion_graph()` après `std::mem::take(&mut self.pending)`.

**Implémentation** :
- Construire un `HashSet<(entity_name, uuid)>` des deletes
- Filtrer `pending.updates` en retirant ceux dont le `(entity_name, uuid)` est dans le set
- Logger le nombre d'updates supprimés via `eprintln!` ou metric

### 2. Updates dupliqués même UUID → merge au node level

**Où** : début de `UpdateRecordNode::execute()`, avant le groupage par entity_name.

**Implémentation** :
- Scanner les `Vec<UpdateRecord>` pour les `(entity_name, uuid)` en double
- Fusionner les `data` BTreeMap dans l'ordre (`extend()` → dernier champ gagne)
- Mettre `new_content_hash = ""` (sentinel) sur l'update fusionné → force `content_changed = true`
- Logger les merges via `ctx.info()`

### 3. Fix bug changed_uuids Vec → HashSet

**Où** : `UpdateRecordNode::execute()`, step 4 (handle content changes).

**Implémentation** :
- Remplacer `let changed_uuids: Vec<&str>` par `let changed_uuids: HashSet<&str>`
- Adapter le UNWIND downstream (`.iter()` → `.iter()`, transparent)

### 4. Delete + Create même UUID → rien à faire

Le graph exécute deletes → inserts. Pas de conflit.

## Fichiers modifiés

| Fichier | Modification |
|---------|-------------|
| `src/catalog.rs` | Filtrage delete-vs-update dans `build_ingestion_graph()` |
| `src/dataflow/record_nodes.rs` | Merge updates dupliqués + fix `changed_uuids` HashSet |

## Vérification

```bash
cargo test --lib --features "rag3db-native,candle-embedder"
./run_e2e.sh
```

Tests spécifiques à vérifier :
- `e2e_drain_unified` — delete + update dans le même drain
- Tests unitaires UpdateRecordNode — double update même UUID
