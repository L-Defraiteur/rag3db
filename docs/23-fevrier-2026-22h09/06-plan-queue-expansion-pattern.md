# 06 — Plan : Pattern d'expansion de queue (Processor → QueueSender)

## Contexte

Actuellement, un `Processor` consomme des ops et interagit avec la DB. Il ne peut pas générer de nouvelles ops. On a besoin d'un pattern où un processor peut **injecter des ops downstream** dans la queue — par exemple un `ChunkProcessor` (prio 0) qui génère des InsertOps/LinkOps/EmbedOps (prio 1/2/3).

Ce pattern est réutilisable pour tout preprocessing futur : validation, transformation, enrichissement, etc.

## Design

### QueueSender

Un handle injectable dans les processors via `tokio::sync::mpsc::unbounded_channel`. Séparation claire producer/consumer :

```rust
use tokio::sync::mpsc;

/// Producer handle — passé aux processors pour injecter des ops downstream.
/// Clone-able, Send, fonctionne en contexte sync (send ne bloque jamais sur unbounded).
#[derive(Clone)]
pub struct QueueSender {
    tx: mpsc::UnboundedSender<CatalogOp>,
}

impl QueueSender {
    /// Inject a new op to be processed in a subsequent priority round.
    pub fn emit(&self, op: CatalogOp) {
        let _ = self.tx.send(op);
    }

    /// Inject multiple ops at once.
    pub fn emit_all(&self, ops: Vec<CatalogOp>) {
        for op in ops {
            let _ = self.tx.send(op);
        }
    }
}

/// Consumer handle — privé, côté queue uniquement.
/// Drain sync via try_recv() après chaque batch de processor.
struct QueueReceiver {
    rx: mpsc::UnboundedReceiver<CatalogOp>,
}

impl QueueReceiver {
    /// Drain all pending ops (non-blocking).
    fn drain(&mut self) -> Vec<CatalogOp> {
        let mut ops = Vec::new();
        while let Ok(op) = self.rx.try_recv() {
            ops.push(op);
        }
        ops
    }
}

/// Create a sender/receiver pair.
fn queue_channel() -> (QueueSender, QueueReceiver) {
    let (tx, rx) = mpsc::unbounded_channel();
    (QueueSender { tx }, QueueReceiver { rx })
}
```

Pourquoi `tokio::sync::mpsc::unbounded_channel` :
- `tokio = { features = ["sync"] }` déjà dans les deps
- `send()` est sync et ne bloque jamais (unbounded)
- `try_recv()` est sync — pas besoin de runtime async pour drain
- Fonctionne en WASM (`tokio::sync` est du pure Rust, pas de syscall)
- Séparation producer/consumer naturelle (vs `Arc<Mutex<Vec>>` qui expose tout)
- Compatible multi-thread : `UnboundedSender` est `Send + Clone`, chaque thread rayon reçoit son clone

### Nouveau trait Processor

```rust
#[async_trait]
pub trait Processor: Send + Sync {
    async fn process(
        &self,
        items: &mut [OperationItem],
        sender: &QueueSender,      // NOUVEAU
    ) -> Result<(), String>;
}
```

Les processors existants (Insert, Link, Embed, SparseEmbed) ignorent `sender` — aucun changement de comportement. Seuls les nouveaux processors "expansifs" l'utilisent.

### Modification de flush()

Le flow actuel :
```
flush():
  partition pending items (une fois)
  group by op_type, sort by priority
  for each group: process batches
```

Le nouveau flow — **single-pass avec merge** :
```
flush():
  let (sender, mut receiver) = queue_channel();

  take all pending items
  let mut groups: BTreeMap<u8, Vec<OperationItem>> = group_by_priority(items);

  for prio in groups.keys().sorted() {
    let group = groups.remove(&prio);
    process_group(group, &sender);

    // Drain le channel et merger les ops injectées dans les groupes restants
    let new_ops = receiver.drain();
    for op in new_ops {
      let new_prio = op.config().priority;
      assert!(new_prio > prio, "processor must not inject ops at same or lower priority");
      groups.entry(new_prio).or_default().push(OperationItem::new(op));
    }
  }
```

**Avantage** : un seul pass, O(groups).
**Contrainte** : un processor ne peut injecter que des ops de priorité **strictement supérieure**. Sinon → panic (bug de design). C'est le bon invariant : un preprocessor (prio 0) génère des inserts (prio 1), pas l'inverse.

**Garde contre boucle infinie** : l'assert `new_prio > prio` garantit la terminaison — on avance toujours vers des priorités plus hautes.

## Implémentation

### Fichiers impactés

| Fichier | Changement |
|---|---|
| `queue.rs` | `QueueSender`, `QueueReceiver`, `queue_channel()`, trait `Processor` modifié, `flush()` refactoré |
| `catalog.rs` | Adapter les 4 processors existants (ajouter `sender: &QueueSender` ignoré) |
| `ops.rs` | Aucun changement |

### Étapes

1. **Ajouter `QueueSender` + `QueueReceiver` + `queue_channel()`** dans `queue.rs`
2. **Modifier le trait `Processor`** — ajouter `sender: &QueueSender`
3. **Adapter les 4 processors existants** — ajouter le param, l'ignorer (préfixer `_sender`)
4. **Refactorer `flush()`** — single-pass avec channel drain + merge des ops injectées
5. **Tests** :
   - Test unitaire QueueSender/Receiver : emit, emit_all, drain
   - Test unitaire flush avec un MockExpandProcessor qui injecte des ops
   - Test : un processor injecte une op de prio inférieure ou égale → panic
   - Test : les 345 tests existants passent toujours (aucun changement de comportement)

### Vérification

```bash
cd packages/rag3db/extension/rag3weaver && cargo test --lib
cargo test --lib -- queue
cargo test --lib -- sender
```
