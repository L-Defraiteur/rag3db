# 02 — Progression : BgeM3Embedder

## État : 1er test passé, reste 8 tests à lancer

---

## Ce qui est fait

### Fichier créé : `extension/rag3weaver/src/bge_m3_embedder.rs` (~280 lignes)

`BgeM3Embedder` qui implémente **les deux traits** `Embedder` (dense) + `SparseEmbedder` (sparse) :

- Utilise `XLMRobertaModel` de candle-transformers **directement** (pas de réécriture de modèle, contrairement à BM42)
- Charge `pytorch_model.bin` (~2.27GB) via `VarBuilder::from_pth()` (pas de safetensors dispo pour BGE-M3)
- Charge `sparse_linear.pt` (3.5KB) séparément — c'est un `Linear(1024→1)`
- Fallback prefix : essaie d'abord sans prefix, puis avec `"roberta"` prefix pour les poids
- Feature gate : `bge-m3` (séparé de `candle-embedder` et `candle-wasm`)

### Architecture interne

```rust
pub struct BgeM3Embedder {
    model: XLMRobertaModel,           // candle-transformers existant
    sparse_linear: candle_nn::Linear,  // 1024 → 1, chargé depuis sparse_linear.pt
    tokenizer: Mutex<Tokenizer>,
    device: Device,
    dim: usize,                        // 1024
}
```

### Méthodes clés

- `forward_pass()` — tokenize + XLMRobertaModel::forward() → `(hidden_states, input_ids, attention_mask)`
- `embed_dense_sync()` — CLS token (position 0) + L2 normalize → `Vec<f32>` (1024d)
- `embed_sparse_sync()` — `sparse_linear(hidden)` → ReLU → scatter par token_id (max si dupliqué) → zéro sur specials (cls=0, pad=1, eos=2, unk=3) → `SparseVector`

### Pipeline sparse BGE-M3 vs BM42

| | BM42 (session précédente) | BGE-M3 (cette session) |
|---|---|---|
| Source du signal | Attention weights CLS (hack) | Linear appris (1024→1) + ReLU |
| Modèle | Bm42Model custom (réécriture BERT) | XLMRobertaModel de candle-transformers |
| Agrégation | Somme des poids par token_id | Max des poids par token_id |
| Vocabulaire | ~30k (WordPiece) | ~250k (SentencePiece XLM-R) |
| Taille modèle | ~22MB (all-MiniLM-L6-v2) | ~2.27GB (XLM-RoBERTa-large) |
| Dense dims | 384 | 1024 |

### Fichiers modifiés

- `Cargo.toml` — ajouté feature `bge-m3 = ["dep:candle-core", "dep:candle-nn", "dep:candle-transformers", "dep:tokenizers", "dep:hf-hub"]`
- `src/lib.rs` — ajouté `#[cfg(feature = "bge-m3")] pub mod bge_m3_embedder;`

### Test passé

```
test bge_m3_embedder::tests::bge_m3_dense_basic ... ok (66.37s)
```

Le modèle a été téléchargé (~2.2GB, maintenant en cache HF) et le dense embedding fonctionne : 1024 dims, L2-normalized.

---

## Tests restants à lancer (9 tests total, 1 passé)

```bash
cd packages/rag3db/extension/rag3weaver
cargo test --features bge-m3 bge_m3 -- --ignored
```

| Test | Ce qu'il valide |
|---|---|
| `bge_m3_dense_basic` | ✅ 1024d, L2-normalized |
| `bge_m3_sparse_basic` | Non-vide, poids positifs, pas de special tokens |
| `bge_m3_batch` | 2 textes → 2 dense + 2 sparse |
| `bge_m3_empty_batch` | 0 texte → 0 résultat (dense et sparse) |
| `bge_m3_dense_cosine_similarity` | Textes proches > textes éloignés |
| `bge_m3_sparse_shared_tokens` | "rust programming" et "rust systems programming" partagent des token_ids |
| `bge_m3_deterministic` | Même texte → même résultat (dense et sparse) |
| `bge_m3_as_embedder_trait` | Fonctionne comme `Box<dyn Embedder>` |
| `bge_m3_as_sparse_embedder_trait` | Fonctionne comme `Box<dyn SparseEmbedder>` |

---

## Décisions prises cette session

1. **Stratégie par tiers** (doc 01) :
   - Natif → BGE-M3 (dense 1024d + sparse appris)
   - WASM défaut → multilingual-MiniLM-L12-v2 (~120MB, 384d)
   - WASM léger → all-MiniLM-L6-v2 (~22MB, 384d)
   - Sparse WASM → BM42 (gardé pour l'instant)

2. **Pas de réécriture de modèle** — candle-transformers a déjà XLMRobertaModel, et BGE-M3 n'a pas besoin des attention weights (contrairement à BM42)

3. **PyTorch pickle** — BGE-M3 n'a pas de safetensors, on charge via `VarBuilder::from_pth()`

4. **Feature gate séparé** — `bge-m3` indépendant de `candle-embedder` pour ne pas forcer le download de 2.2GB

---

## Prochaines étapes

1. **Lancer tous les tests BGE-M3** (les 8 restants)
2. **Si tout passe** : commit + push
3. **Phase 4** : Intégration rag3weaver — remplacer SparseIndex in-memory par l'extension sparse_vector via Cypher
4. **Plus tard** : Single forward pass (dense + sparse en un seul appel pour éviter 2 forward passes)

---

## Commandes utiles

```bash
# Compiler seul (vérif rapide)
cd packages/rag3db/extension/rag3weaver
cargo check --features bge-m3

# Tous les tests BGE-M3 (~2.2GB modèle en cache après 1er run)
cargo test --features bge-m3 bge_m3 -- --ignored

# Tests BM42 (modèle MiniLM ~22MB)
cargo test --features candle-embedder bm42 -- --ignored

# Tests unitaires (sans modèle)
cargo test
```
