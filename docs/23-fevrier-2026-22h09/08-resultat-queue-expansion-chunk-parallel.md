# 08 — Résultat : Queue expansion pattern + ChunkOp parallèle

## Ce qui a été implémenté

### 1. Queue expansion pattern (doc 06)

**`queue.rs`** — nouveau pattern réutilisable : un processor peut injecter des ops downstream via un channel.

**QueueSender / QueueReceiver** :
- `tokio::sync::mpsc::unbounded_channel` — `send()` sync, `try_recv()` sync, fonctionne en WASM
- `QueueSender` : clone-able, `Send`, passé aux processors. `emit()` / `emit_all()`
- `QueueReceiver` : privé côté queue, drainé après chaque groupe de priorité

**Trait Processor modifié** :
```rust
async fn process(&self, items: &mut [OperationItem], sender: &QueueSender) -> Result<(), String>;
```
Les 4 processors existants (Insert, Link, Embed, SparseEmbed) ignorent `_sender`.

**flush() refactoré** — single-pass via `BTreeMap<u8, Vec<OperationItem>>` :
1. Groupe les items par priorité (BTreeMap = ordre garanti)
2. Traite chaque niveau de priorité
3. Après chaque niveau, drain le channel et merge les ops injectées dans les groupes restants
4. `assert!(new_prio > current_prio)` — empêche les boucles infinies

Les ops injectées au-delà de `max_priority` (ex: flush_insertions prio ≤ 1 mais embeds injectés à prio 3) sont gardées comme pending pour un flush ultérieur.

**6 tests ajoutés** :
- `sender_emit_and_drain`, `sender_emit_all` — channel basique
- `expand_processor_injects_downstream` — insert processor émet embeds → 4 processés
- `expand_respects_flush_up_to_priority` — embeds injectés hors scope → pending
- `expand_same_priority_panics`, `expand_lower_priority_panics` — gardes panic

### 2. ChunkOp parallèle (doc 07)

**`ops.rs`** — nouveau type `ChunkOp` :
```rust
pub struct ChunkOp {
    pub entity_name: String,
    pub parent_uuid: String,
    pub entity_ref: EntityRef,
    pub data: HashMap<String, CypherValue>,
}
```
Priorité 0, batch_size 10 000, max_retries 0.

**`catalog.rs`** :

- **`compute_chunk_ops()`** — fonction standalone (pas de `&self`), prend des refs explicites vers config/kb_metadata/chunker_cache. Utilisable depuis rayon sans conflit de borrow.

- **`ChunkProcessor`** — processor à priorité 0 :
  - Reçoit un batch de ChunkOps (potentiellement 10K docs)
  - `rayon::par_iter()` sur le batch → chunking parallèle sur tous les cores
  - Émet les ops downstream (InsertOp, LinkOp, EmbedOp, SparseEmbedOp) via `sender.emit_all()`

- **`create()` simplifié** — enqueue 2 ops (1 InsertOp + 1 ChunkOp). Pas de chunking à create time. Instantané.

- **`update()` simplifié** — quand le contenu change : delete vieux chunks + enqueue 1 ChunkOp.

- **`warm_chunker_cache()`** — pré-remplit le cache pour toutes les configs KB. Le cache est transféré au ChunkProcessor via `std::mem::take` dans `initialize()`.

**`Cargo.toml`** — rayon non-optional (était gated derrière `wasm-emscripten`).

**`cypher_persistence.rs`** — gestion du nouveau variant ChunkOp dans le match.

## Flow complet

```
create("Document", data) × 10 000        ~instantané
  ├─ UUID + content_hash
  ├─ Enqueue InsertOp (prio 1)
  └─ Enqueue ChunkOp (prio 0)

drain()
  prio 0 → ChunkProcessor
    └─ rayon::par_iter sur 10K ChunkOps
       └─ compute_chunk_ops() par doc → N InsertOps + N LinkOps + N EmbedOps
       └─ sender.emit_all() → mergés dans prio 1/2/3

  prio 1 → InsertProcessor
    └─ 10K entity inserts + Σ chunk inserts

  prio 2 → LinkProcessor
    └─ Σ parent→chunk links

  prio 3 → EmbedProcessor + SparseEmbedProcessor
    └─ Σ chunk embeddings (déjà batché, 1 appel embedder par batch de 32)
```

## Tests

351 tests, 0 failures, 5 ignored.

Les tests existants ont été adaptés :
- `ops_enqueued_per_create()` = 2 (InsertOp + ChunkOp)
- `ops_per_create()` = 2 + N×3 (ChunkOp + entity insert + N × (chunk insert + link + embed))
- `flush_insertions_only` : prio ≤ 1 traite ChunkOp (prio 0) + tous les inserts (prio 1)

## Fichiers modifiés

| Fichier | Changement |
|---|---|
| `queue.rs` | QueueSender, QueueReceiver, queue_channel(), Processor trait, flush() BTreeMap, new_operation_item(), 6 tests |
| `ops.rs` | ChunkOp, OP_CHUNK, CatalogOp::Chunk variant |
| `catalog.rs` | compute_chunk_ops(), ChunkProcessor, warm_chunker_cache(), create()/update() simplifiés, tests adaptés |
| `Cargo.toml` | rayon non-optional |
| `cypher_persistence.rs` | CatalogOp::Chunk match arm |

## Propriétés du design

- **Transparent** : le caller fait `create()` comme avant, aucun changement d'API
- **Parallèle** : rayon utilise tous les cores pour le chunking
- **Extensible** : le pattern QueueSender/expansion est réutilisable pour tout preprocessing futur
- **Sûr** : l'assert `new_prio > current_prio` garantit la terminaison du flush
- **Compatible WASM** : tokio::sync::mpsc fonctionne en WASM, rayon est configuré séparément pour emscripten
