# 07 — Plan : ChunkOp parallèle (dépend de doc 06)

## Prérequis

Le pattern d'expansion de queue (doc 06) doit être implémenté et testé d'abord.

## Contexte

Avec le QueueSender en place, on peut créer un `ChunkOp` (priorité 0) qui contient les données brutes d'un document. Le `ChunkProcessor` reçoit un batch de ChunkOps, parallélise le chunking via rayon, et injecte les ops downstream (InsertOp, LinkOp, EmbedOp, SparseEmbedOp) via le sender.

Le caller continue d'appeler `create()` normalement. Le chunking est 100% transparent et parallélisé.

## Design

### Nouveau type d'op : ChunkOp

```rust
pub struct ChunkOp {
    pub entity_name: String,
    pub parent_uuid: String,
    pub entity_ref: EntityRef,
    pub data: HashMap<String, CypherValue>,
}
```

Priority : **0** (avant tout le reste).
Batch size : **illimité** (ou très grand, ex: 10_000) — on veut tout le batch d'un coup pour maximiser le parallélisme rayon.

### Ajout au CatalogOp enum

```rust
pub enum CatalogOp {
    Insert(InsertOp),
    Link(LinkOp),
    Embed(EmbedOp),
    SparseEmbed(SparseEmbedOp),
    Chunk(ChunkOp),       // NOUVEAU
}
```

### ChunkProcessor

```rust
struct ChunkProcessor {
    config: CatalogConfig,
    kb_metadata: HashMap<String, KBMetadata>,
    chunker_cache: HashMap<ChunkerConfig, Chunker>,
    has_sparse: bool,
}

#[async_trait]
impl Processor for ChunkProcessor {
    async fn process(
        &self,
        items: &mut [OperationItem],
        sender: &QueueSender,
    ) -> Result<(), String> {
        // Extraire les ChunkOps du batch
        let chunk_ops: Vec<&ChunkOp> = items.iter()
            .filter_map(|item| match &item.op {
                CatalogOp::Chunk(c) => Some(c),
                _ => None,
            })
            .collect();

        // Paralléliser le chunking via rayon
        let all_downstream_ops: Vec<Vec<CatalogOp>> = chunk_ops
            .par_iter()
            .map(|chunk_op| {
                compute_chunk_ops_standalone(
                    &chunk_op.entity_name,
                    &chunk_op.parent_uuid,
                    &chunk_op.entity_ref,
                    &chunk_op.data,
                    &self.config,
                    &self.kb_metadata,
                    &self.chunker_cache,
                    self.has_sparse,
                )
            })
            .collect();

        // Injecter les ops downstream via sender
        for ops in all_downstream_ops {
            sender.emit_all(ops);
        }

        Ok(())
    }
}
```

### Modification de create()

```rust
pub fn create(&mut self, entity_name: &str, data: HashMap<String, CypherValue>)
    -> Result<EntityRef, CatalogError>
{
    // ... UUID, content_hash, full_data, entity_ref (inchangé)

    let insert_op = CatalogOp::Insert(InsertOp::new(...));

    // Remplacer build_chunk_ops() par un ChunkOp
    let chunk_op = CatalogOp::Chunk(ChunkOp {
        entity_name: entity_name.to_string(),
        parent_uuid: uuid.clone(),
        entity_ref: entity_ref.clone(),
        data: data.clone(),  // données brutes pour chunking lazy
    });

    self.queue.enqueue_all(vec![insert_op, chunk_op]);
    Ok(entity_ref)
}
```

`create()` ne fait plus de chunking du tout. C'est instantané.

### Modification de update()

Même pattern : quand le contenu change, delete les vieux chunks + enqueue un `ChunkOp` au lieu d'appeler `build_chunk_ops()`.

### Initialisation du ChunkProcessor

Dans `initialize()`, le ChunkProcessor est construit avec les refs nécessaires :

```rust
// Dans initialize(), après build KB metadata :
self.warm_chunker_cache();

let chunk_processor = ChunkProcessor {
    config: self.config.clone(),
    kb_metadata: self.kb_metadata.clone(),
    chunker_cache: std::mem::take(&mut self.chunker_cache), // move, le processor est le owner
    has_sparse: self.sparse_embedder.is_some(),
};

self.queue.register_processor("chunk", Box::new(chunk_processor));
```

Note : le `chunker_cache` est **transféré** au processor. Le Catalog n'en a plus besoin puisque `build_chunk_ops()` n'est plus appelé directement.

### Rayon

Rendre rayon non-optional dans `Cargo.toml` :

```toml
rayon = "1.10"   # était optional
```

## Flow complet

```
create("Document", {title, body}) × 10_000
  ├─ UUID + content_hash              (rapide)
  ├─ Enqueue InsertOp (prio 1)
  └─ Enqueue ChunkOp (prio 0)         (pas de chunking ici)

drain()
  flush() single-pass :
    prio 0 : ChunkProcessor
      ├─ Reçoit 10_000 ChunkOps en 1 batch
      ├─ rayon::par_iter → chunk tous les docs en parallèle
      └─ sender.emit_all() → N×4 ops (insert/link/embed/sparse)
         → mergées dans les groupes prio 1/2/3

    prio 1 : InsertProcessor
      ├─ Insert entity nodes
      └─ Insert chunk nodes (injectés par ChunkProcessor)

    prio 2 : LinkProcessor
      └─ parent → chunk relations

    prio 3 : EmbedProcessor + SparseEmbedProcessor
      └─ embed tous les chunks (déjà batché)
```

## Avantages

- **Transparent** : le caller fait `create()` comme avant
- **Parallèle** : 10K docs chunkés en parallèle via rayon, utilise tous les cores
- **Extensible** : le pattern QueueSender permet d'autres processors expansifs
- **Propre** : le chunking est un processor comme les autres, pas un cas spécial dans create()

## Tests à ajouter

- `create_then_drain_produces_chunk_ops` — vérifier que le ChunkProcessor génère les bons ops
- `chunk_processor_parallel_correctness` — 100 docs, vérifier que le résultat est identique au séquentiel
- `update_reemits_chunk_op` — vérifier qu'un update avec contenu changé émet un nouveau ChunkOp
- Les 345+ tests existants passent toujours

## Fichiers impactés

| Fichier | Changement |
|---|---|
| `Cargo.toml` | rayon non-optional |
| `ops.rs` | `ChunkOp` struct + ajout au `CatalogOp` enum |
| `catalog.rs` | `ChunkProcessor`, `create()` simplifié, `update()` simplifié, `compute_chunk_ops_standalone()` extrait |
| `queue.rs` | (déjà modifié par doc 06) |
