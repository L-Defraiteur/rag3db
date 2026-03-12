# Doc 12 — Undo actif : plan d'implémentation

Date : 12 mars 2026

## Contexte

Le système d'undo est **architecturé et partiellement implémenté** :

- Le trait `Node` définit `can_undo()`, `undo_context()`, `undo()` (node.rs:63-88)
- Le runtime capture `undo_context()` après chaque `execute()` réussi et le persiste dans le checkpoint (runtime.rs:509-535)
- Le rollback est implémenté dans `migrations.rs:638-760` : charge les checkpoints, appelle `undo()` en ordre topologique inverse
- **6 nœuds implémentent déjà undo** : InsertRecordNode, LinkRecordNode, KBUpdateNode, KBEmbedNode, EmbedNode, FlushNode

## Analyse du cascade delete

DeleteRecordNode fait **tout le cascade en interne** — pas de nœuds downstream séparés pour le cleanup :

| Cas | Ce que delete fait | Effets |
|-----|-------------------|--------|
| **titleFor KB** | Delete KB_Index_Chunk + Delete KB_Index + DETACH DELETE entity | Chunks, index entries, entity, relations — tout supprimé |
| **contentFor KB** | Delete SOURCED chunks + enqueue re-aggregation + DETACH DELETE entity | Chunks supprimés, re-agrégation déclenchée downstream |
| **Simple entity** | Delete Entity_Chunk + DETACH DELETE entity | Chunks, entity, relations — tout supprimé |

**Problème DETACH DELETE** : supprime l'entité ET toutes ses relations d'un coup, sans qu'on sache quelles relations existaient.

### Options évaluées

| Option | Approche | Verdict |
|--------|----------|---------|
| **A — Undo complet** | Capturer entity data + relations + chunks + KB entries avant delete | Trop coûteux (3+ queries supplémentaires par groupe), checkpoint JSON volumineux |
| **B — Undo entités + re-ingestion** | Capturer entity data uniquement, enqueuer la ré-ingestion après undo | **Retenu** — simple, réutilise le pipeline existant |
| **C — Graph-as-node** | Le undo exécute un sub-graph de ré-ingestion inline | Intéressant mais feature à part entière, à tester indépendamment |

### Design retenu : Option B — Restore + Enqueue

```
DeleteRecordNode.undo()
  1. Restore entities (CREATE avec données capturées)
  2. Enqueue InsertRecords dans pending_inserts
     → le pipeline d'ingestion normal recréera :
       - Chunks (ChunkRecordNode / KBChunkNode)
       - Relations (LinkRecordNode)
       - Embeddings (EmbedNode / KBEmbedNode)
       - KB_Index entries (KBUpdateNode)
       - FTS (FlushNode)
```

**Auto-drain après rollback** (`migrations.rs`) :
```rust
// Après rollback en reverse topo order :
if catalog.has_pending() {
    log::info!("rollback enqueued re-ingestion, running auto-drain...");
    catalog.drain().await?;
}
```

## Implémentation

### Étape 1 — DeleteRecordNode undo

**Fichier** : `src/dataflow/record_nodes.rs`

**Données à capturer** (avant les DETACH DELETE, dans execute()) :
```rust
// Read complet des entités avant suppression
let read_cypher = format!(
    "UNWIND $uuids AS uuid MATCH (n:{entity_name} {{_uuid: uuid}}) RETURN n"
);
```

**Undo context** :
```json
{
  "entities": {
    "User": [
      { "_uuid": "abc", "name": "Alice", "email": "a@b.com", "_content_hash": "..." },
      { "_uuid": "def", "name": "Bob", ... }
    ],
    "Document": [ ... ]
  }
}
```

**undo()** :
```rust
async fn undo(&mut self, ctx: &mut NodeContext, undo_ctx: serde_json::Value) -> Result<(), String> {
    let conn = ctx.service::<Arc<dyn DbConnection>>("conn")?;
    let pending = ctx.service::<Mutex<PendingWork>>("pending")?;
    let entities = undo_ctx["entities"].as_object()?;

    for (entity_name, items) in entities {
        let items_arr = items.as_array()?;
        // 1. Re-create entities
        let cypher = format!(
            "UNWIND $items AS item CREATE (n:{entity_name}) SET n = item"
        );
        conn.execute_with_params(&cypher, &items_to_params(items_arr)).await?;

        // 2. Enqueue for re-ingestion (chunks, relations, embeddings)
        let mut pending = pending.lock()?;
        for item in items_arr {
            pending.inserts.push(InsertRecord::from_map(entity_name, item));
        }

        ctx.info(format!("restored {} {entity_name}(s), re-ingestion enqueued", items_arr.len()));
    }
    Ok(())
}
```

**Struct** : ajouter `undo_data: Option<serde_json::Value>` au struct DeleteRecordNode.

### Étape 2 — UpdateRecordNode undo

**Fichier** : `src/dataflow/record_nodes.rs`

**Données à capturer** : étendre le read existant (step 1, batch-read old hashes) pour lire l'entité complète :
```rust
// Actuellement : RETURN n._uuid, n._content_hash
// Après : RETURN n._uuid, n  (entité complète)
```

**undo()** :
```rust
async fn undo(&mut self, ctx: &mut NodeContext, undo_ctx: serde_json::Value) -> Result<(), String> {
    let conn = ctx.service::<Arc<dyn DbConnection>>("conn")?;
    let entities = undo_ctx.as_object()?;

    for (entity_name, items) in entities {
        let cypher = format!(
            "UNWIND $items AS item MATCH (n:{entity_name} {{_uuid: item._uuid}}) SET n = item"
        );
        conn.execute_with_params(&cypher, &items_to_params(items)).await?;
        ctx.info(format!("restored {} {entity_name}(s) to pre-update state", items.as_array().map(|a| a.len()).unwrap_or(0)));
    }
    Ok(())
}
```

Pattern identique à KBUpdateNode qui implémente déjà undo avec capture d'anciennes valeurs (record_nodes.rs:2466-2505).

### Étape 3 — Auto-drain après rollback

**Fichier** : `src/migrations.rs`

Après la boucle de rollback (ligne ~730), vérifier si du pending work a été enqueué et lancer un drain :
```rust
// After all undo() calls complete:
if catalog.has_pending() {
    // Auto-drain to re-ingest restored entities
    catalog.drain().await.map_err(|e| MigrationError::ExecutionError(e.to_string()))?;
}
```

### Étape 4 — Nœuds read-only : undo no-op

Les nœuds read-only (ChunkRecordNode, RechunkDeleteNode, KBGatherNode, KBChunkNode) bloquent le rollback car `can_undo() = false` → `migrations.rs` refuse avec `NotReversible`.

**Fix** : marquer ces nœuds comme `can_undo() = true` avec un `undo()` no-op :
```rust
fn can_undo(&self) -> bool { true }
async fn undo(&mut self, _ctx: &mut NodeContext, _undo_ctx: serde_json::Value) -> Result<(), String> {
    Ok(()) // Read-only: nothing to undo
}
```

Nœuds concernés : ChunkRecordNode, RechunkDeleteNode, KBGatherNode, KBChunkNode.

### Étape 5 (optionnel) — PendingWork comme service

Pour que `undo()` puisse enqueuer du pending work, il faut que `PendingWork` soit accessible en tant que service dans le NodeContext. Vérifier si c'est déjà le cas, sinon l'ajouter au ServiceRegistry lors du rollback dans `migrations.rs`.

## Fichiers concernés

| Fichier | Étape | Modification |
|---------|-------|-------------|
| `src/dataflow/record_nodes.rs` | 1, 2 | undo_data + can_undo + undo() sur Delete et Update |
| `src/migrations.rs` | 3 | Auto-drain après rollback |
| `src/dataflow/record_nodes.rs` | 4 | can_undo() no-op sur nœuds read-only |
| `src/dataflow/node.rs` | 5 | Vérifier ServiceRegistry pour PendingWork |

## Vérification

```bash
cargo check --lib --features "rag3db-native,candle-embedder"
cargo test --lib --features "rag3db-native,candle-embedder"
./run_e2e.sh
```

Tests spécifiques à vérifier :
- Rollback d'un pipeline avec DeleteRecordNode → entités restaurées + chunks recréés
- Rollback d'un pipeline avec UpdateRecordNode → anciennes valeurs restaurées
- Rollback d'un pipeline mixte (insert + update + delete)

## Risques

- **PendingWork service** : si PendingWork n'est pas accessible dans le contexte undo, il faut l'ajouter au ServiceRegistry. Le rollback dans migrations.rs construit son propre ServiceRegistry — vérifier qu'il a accès au catalog/pending.
- **Relations** : les relations ne sont PAS restaurées directement. Le re-drain via InsertRecords devrait les recréer SI les InsertRecords contiennent les données de relation (ref fields). À vérifier.
- **Idempotence** : si le auto-drain échoue après le rollback, le système est dans un état intermédiaire (entités restaurées mais pas de chunks/embeddings). Un deuxième drain devrait résoudre ça.

## Priorité

**Moyenne.** Utile pour crash recovery et rollback de migrations. UpdateRecordNode est faible effort (étendre un read existant). DeleteRecordNode est moyen effort (read supplémentaire + enqueue).
