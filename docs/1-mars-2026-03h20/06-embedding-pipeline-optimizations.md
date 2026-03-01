# 06 — Optimisations pipeline embedding

## Contexte

Analyse du pipeline d'ingestion (drain) — les timings E2E montrent que l'embedding domine le temps de drain. Plusieurs optimisations identifiées.

## État actuel

### Architecture batching

```
Catalog::drain()
  → OperationQueue::flush()
    → Priority 0: Chunk (batch=10000)
    → Priority 1: Insert (batch=50) — Cypher UNWIND
    → Priority 2: Link (batch=50) — Cypher UNWIND
    → Priority 3: Embed (batch=32) — embedder.embed() + Cypher UNWIND
    → Priority 3: SparseEmbed (batch=32) — sparse_embedder.embed_sparse() + Cypher UNWIND
```

### Ce qui marche bien

- Batching GPU effectif : un seul `model.forward()` par batch de 32
- DB writes groupés via UNWIND
- Priority ordering respecte les dépendances

### Problèmes identifiés

1. **Double forward pass** — BGE-M3 et MiniLM+BM42 font 2 forward passes du même modèle (dense puis sparse)
2. **Sparse embed joint le titre** — `texts.join("\n")` dilue les poids sparse (OK pour dense, mauvais pour sparse)
3. **Pas de timing per-op** — impossible de savoir le temps embedding vs le temps UNWIND DB
4. **UNWIND trop fréquent** — un UNWIND par batch de 32, overhead réseau/transaction
5. **Batch size hardcodé** — 32 pas configurable

---

## Optimisation 1 : DualEmbedder — un seul forward pass

### Problème

BGE-M3 : `EmbedProcessor` appelle `embed()` → forward pass XLMRoberta → CLS dense.
Puis `SparseEmbedProcessor` appelle `embed_sparse()` → **même forward pass** → sparse_linear.

MiniLM + BM42 : `CandleEmbedder.embed()` → forward pass BERT → mean pooling dense.
Puis `Bm42Embedder.embed_sparse()` → **même forward pass** → CLS attention sparse.

Le forward pass est l'opération la plus coûteuse (~150-400ms/batch GPU). On la fait 2x.

### Solution

Nouveau trait :

```rust
#[async_trait]
pub trait DualEmbedder: Send + Sync {
    /// Un seul forward pass, retourne dense + sparse
    async fn embed_dual(&self, texts: &[String])
        -> Result<(Vec<Vec<f32>>, Vec<SparseVector>), EmbedError>;

    fn dim(&self) -> usize;
}
```

Implémentations :

**BGE-M3** :
```rust
impl DualEmbedder for BgeM3Embedder {
    async fn embed_dual(&self, texts: &[String]) -> Result<...> {
        let (hidden_states, input_ids, attention_mask) = self.forward_pass(texts)?;
        let dense = self.extract_dense(&hidden_states, &attention_mask)?;
        let sparse = self.extract_sparse(&hidden_states, &input_ids)?;
        Ok((dense, sparse))
    }
}
```

**MiniLM + BM42** :
```rust
impl DualEmbedder for CandleDualEmbedder {
    async fn embed_dual(&self, texts: &[String]) -> Result<...> {
        // Bm42Model.forward() retourne (hidden_states, attn_probs)
        let (hidden_states, attn_probs) = self.model.forward(&tokens, ...)?;
        let dense = mean_pool(&hidden_states, &attention_mask)?;
        let sparse = extract_cls_attention(&attn_probs, &input_ids)?;
        Ok((dense, sparse))
    }
}
```

### Impact dans le queue

Option A — **DualEmbedProcessor** : un seul processor qui produit dense + sparse en un appel, puis fait 2 UNWINDs.

Option B — **Cache hidden_states** : le premier processor (embed) cache les hidden_states, le second (sparse_embed) les réutilise. Plus complexe, moins propre.

→ **Option A préférée** : plus simple, moins de state partagé.

### Gain estimé

~40-50% du temps embedding. Sur un drain de 3 docs BGE-M3 : ~700ms → ~400ms.

---

## Optimisation 2 : Sparse sans join titre

### Problème

Actuellement `SparseEmbedProcessor` fait :
```rust
text: embed.texts.join("\n")  // titre + content concaténés
```

Pour le dense, joindre titre+content est raisonnable (le modèle capte le contexte global).

Pour le sparse, c'est contre-productif :
- Les tokens du titre diluent les poids d'attention des tokens du contenu
- Un titre court comme "Rust Programming" pèse autant qu'un long paragraphe
- L'index sparse en mémoire est plus gros (plus de tokens non-pertinents)

### Solution

`SparseEmbedOp` devrait avoir un champ `sparse_texts: Vec<String>` séparé qui exclut le titre, ou bien le processor filtre les champs de type `title`.

Alternative : dans `compute_chunk_ops()`, ne mettre que les champs `content` dans `SparseEmbedOp.texts` (pas les champs `title` ni `summary`).

### Impact

Qualité sparse améliorée, index plus compact. Pas d'impact perf direct.

---

## Optimisation 3 : Timing per-phase via EventBus

### Problème

Le drain retourne `FlushResult { processed, failed }` mais pas de timing détaillé. Impossible de savoir :
- Combien de temps l'embedding a pris vs le UNWIND DB
- Si c'est l'extension vector (HNSW insert) ou le sparse_vector qui est lent
- Le coût du Cypher INSERT vs l'embedding

### Solution

Utiliser l'`EventBus` existant pour publier des événements de timing :

```rust
pub enum QueueEvent {
    // existants...
    ProcessingBatch { op_type, priority, items },
    BatchCompleted { op_type, priority, items },

    // nouveaux
    BatchTiming {
        op_type: &'static str,
        phase: &'static str,    // "embed", "db_write", "total"
        batch_size: usize,
        duration_ms: u64,
    },
}
```

Chaque processor mesure :
1. `t0` = avant `embedder.embed()`
2. `t1` = après embed, avant UNWIND
3. `t2` = après UNWIND

Publie :
- `BatchTiming { op_type: "embed", phase: "model", duration: t1-t0 }`
- `BatchTiming { op_type: "embed", phase: "db_write", duration: t2-t1 }`

Le consumer (côté Node.js/WASM) peut agréger et logger.

### Impact

Observabilité. Permet de diagnostiquer les bottlenecks en production.

---

## Optimisation 4 : UNWIND accumulé via EmbedBatchOp

### Problème

Actuellement : chaque batch de 32 embeddings fait son propre UNWIND Cypher. Avec 500 docs :
- 500/32 = ~16 UNWINDs pour dense
- ~16 UNWINDs pour sparse
- = 32 transactions auto-commit

Chaque UNWIND a un overhead fixe (parse Cypher, prepare statement, commit transaction).

### Findings — comparaison avec ragforge-core (Neo4j)

ragforge-core utilisait un **batch de 500** pour les UNWIND Neo4j, avec séparation claire embedding vs DB write et timing indépendant pour chaque phase :
```
[Embedding] openai took 1200ms for 500 embeddings
[Embedding] DB save took 340ms for 500 embeddings
```

### Findings — UNWIND + SET trigger HNSW

Vérifié dans le code rag3db : chaque `SET n.embedding = item.emb` dans un UNWIND déclenche `HNSWIndex::update()` (delete edges + KNN search + reinsert). Ce trigger se fait **dans la même transaction** que le UNWIND. Donc un gros UNWIND de 500 items = 500 HNSW updates atomiques dans 1 transaction. Pas de risque de partial update.

L'overhead par transaction = parse Cypher + prepare statement + commit + WAL flush. Passer de 16 transactions à 1 élimine 15x cet overhead.

### Design — EmbedBatchOp (2 niveaux d'opération)

Séparer le calcul GPU de la persistance DB avec deux niveaux d'opérations :

```
                    GPU batching (32)              DB batching (500)
                    ─────────────────              ─────────────────
EmbedOp(32 items) → embed() → accumulate ─┐
EmbedOp(32 items) → embed() → accumulate  ├──→ EmbedBatchOp(~500)
EmbedOp(32 items) → embed() → accumulate  │      → UNWIND(500)
...                                       ─┘      → mark all done
```

**EmbedOp** (batch=32) :
- Unité de calcul GPU — taille optimale pour VRAM
- Appelle `embedder.embed(&texts)` ou `embed_dual(&texts)`
- Accumule les résultats `(uuid, Vec<f32>)` en mémoire
- **Pas de persistance DB** — si crash, perdu mais recalculable

**EmbedBatchOp** (batch=~500, configurable) :
- Unité de persistance DB — taille optimale pour transaction overhead
- Déclenché quand l'accumulateur atteint `unwind_batch_size` OU fin du flush
- Fait le(s) UNWIND Cypher groupé(s) par (entity, column)
- **Unité de reprise** : si crash mid-UNWIND, tout le batch est rejoué

### Logique de reprise

La reprise se fait **uniquement au niveau EmbedBatchOp** :

1. Chaque EmbedBatchOp a un ID et un statut (`pending`, `in_progress`, `completed`)
2. Les EmbedOps individuels n'ont pas de logique de reprise — ce sont des calculs purs
3. Au redémarrage après crash :
   - EmbedBatchOps `completed` → rien à faire
   - EmbedBatchOps `in_progress` → les embeddings en mémoire sont perdues
     → on recalcule les EmbedOps du batch (GPU) puis on refait le UNWIND
   - Détection via `compute_chunk_ops()` : les nœuds sans embedding sont recalculés
4. Pas de corruption possible : le UNWIND est atomique (auto-commit transaction)

### Risque crash : 32 items perdus vs 500 items perdus

| Scénario | Actuel (UNWIND par 32) | EmbedBatchOp (UNWIND par 500) |
|----------|----------------------|-------------------------------|
| Crash pendant embedding | Perte du batch GPU en cours (~32) | Idem (~32) |
| Crash pendant UNWIND | Perte de 0 items (transaction rollback) | Perte de 0 items (transaction rollback) |
| Crash entre 2 UNWINDs | Items du batch précédent sauvés | Items accumulés en mémoire perdus (~500 max) |
| Coût de reprise | Re-embed ~32 items | Re-embed ~500 items |

Le coût de reprise max passe de ~32 à ~500 re-embeddings, soit ~15 forward passes GPU (~5s BGE-M3). Acceptable vu que le crash est un événement rare et que le gain en throughput normal est significatif.

### Implémentation dans le queue

```rust
pub struct FlushConfig {
    pub auto: bool,
    pub max_count: usize,
    pub completed_retention_ms: u64,
    pub unwind_batch_size: usize,  // NEW — default 500
}
```

Le `DualEmbedProcessor` (optim 1) intègre naturellement ce pattern :

```rust
impl Processor for DualEmbedProcessor {
    async fn process(&self, items: &mut [OperationItem], _sender: &QueueSender) -> Result<(), String> {
        // Phase 1 : GPU embedding (par batch de 32, via embed_dual)
        let mut accumulator: Vec<EmbedResult> = Vec::new();
        for gpu_batch in items.chunks(self.gpu_batch_size) {
            let texts = collect_texts(gpu_batch);
            let (dense, sparse) = self.embedder.embed_dual(&texts).await?;
            accumulator.extend(zip_results(gpu_batch, dense, sparse));
        }

        // Phase 2 : DB persistence (tout d'un coup = 1 transaction)
        self.write_dense_unwind(&accumulator).await?;
        self.write_sparse_unwind(&accumulator).await?;

        Ok(())
    }
}
```

Le queue passe des mega-batches de ~500 au processor, qui subdivise en GPU batches de 32 en interne.

### Questions ouvertes

- **Taille optimale UNWIND rag3db** : 500 était optimal pour Neo4j, faut benchmarker pour rag3db (→ dépend de l'optim 3 timing). Possible que rag3db, étant embedded (pas de réseau), ait un overhead transaction plus faible → le gain serait moindre.
- **Memory pressure** : 500 embeddings de dim 1024 = ~2MB en mémoire. Négligeable.
- **HNSW lock contention** : 500 HNSW updates dans une seule transaction — est-ce que le graph HNSW supporte bien ? À vérifier mais a priori oui puisque c'est séquentiel dans la même transaction.

### Impact estimé

À mesurer via optim 3. Hypothèse : 10-30% sur le temps DB write. Sur un drain de 500 docs avec BGE-M3 (embed ~60s, DB write ~5-15s), gain absolu de ~1-5s.

---

## Optimisation 5 : Batch size configurable

### Problème

`OP_EMBED.batch_size = 32` hardcodé. Optimal pour GPU moyen, mais :
- Gros GPU (24GB+) : pourrait monter à 64-128
- CPU / WASM : 8-16 serait mieux (moins de mémoire)
- Le batch size optimal dépend du modèle (MiniLM = petit → gros batch, BGE-M3 = gros → petit batch)

### Solution

Ajouter dans `CatalogConfig` :

```rust
pub struct FlushConfig {
    // ...existant...
    pub embed_batch_size: Option<usize>,  // override OP_EMBED.batch_size
}
```

Le queue utilise `config.embed_batch_size.unwrap_or(OP_EMBED.batch_size)`.

### Impact

Flexibilité. Pas de gain par défaut mais permet le tuning.

---

## Priorités

| # | Optim | Gain | Effort | Priorité |
|---|-------|------|--------|----------|
| 3 | Timing per-phase | Observabilité — débloque les mesures | Faible | **P0** |
| 1 | DualEmbedder | ~40-50% embed time | Moyen | **P0** |
| 4 | EmbedBatchOp (mega-UNWIND) | 10-30% DB write + reprise propre | Moyen | **P1** |
| 2 | Sparse sans titre | Qualité sparse | Faible | **P1** |
| 5 | Batch size config | Flexibilité GPU/CPU/WASM | Faible | **P2** |

### Ordre d'implémentation recommandé

1. **Timing** (#3) d'abord — mesurer l'état actuel (embed vs DB write)
2. **DualEmbedder** (#1) — le plus gros gain, refactor embedder + processor
3. **EmbedBatchOp** (#4) — refactor queue pour séparer GPU batching (32) de DB batching (500)
   - L'optim 1 (DualEmbedProcessor) s'intègre naturellement avec l'optim 4 (le processor reçoit ~500 items, subdivise en GPU batches de 32, accumule, puis fait 1-2 UNWINDs)
4. **Sparse sans titre** (#2) — changement dans `compute_chunk_ops()`, indépendant
5. **Batch size config** (#5) — trivial une fois le reste en place

### Dépendances

```
#3 (timing) ──→ mesures baseline
                    ↓
#1 (DualEmbedder) + #4 (EmbedBatchOp) ──→ peuvent se faire ensemble
                    ↓                       (DualEmbedProcessor intègre les 2)
              mesures post-optim
                    ↓
#2 (sparse titre) ──→ indépendant, peut se faire à tout moment
#5 (batch config) ──→ trivial, fin
```
