# 10 — Visions : Évolutions futures sparse/BM42

Classées par ordre de complexité croissante (de la plus simple à la plus incertaine).

---

## 1. Single Forward Pass — dense + sparse BM42 en un passage

### Complexité : faible
### Inconnues design : aucune

### Le problème

Aujourd'hui on a deux embedders séparés :
- `CandleEmbedder` : forward BERT → mean pooling → `Vec<f32>` (dense, 384 dims)
- `Bm42Embedder` : forward BERT → attention CLS → `SparseVector` (sparse, ~10-30 dims actives)

Le même modèle (all-MiniLM-L6-v2, ~22MB) est chargé deux fois en mémoire, et chaque texte subit deux forward passes BERT identiques. Le forward est le goulot (~2-5ms/texte sur CPU).

### La solution

Un seul `HybridEmbedder` qui fait :

```
Texte → Tokenizer → BERT forward (Bm42Model)
                        ↓
              (hidden_states, attention_probs)
                   ↓              ↓
         mean pooling        CLS attention row
                   ↓              ↓
           Vec<f32>          SparseVector
           (dense)           (sparse BM42)
```

### Interface

```rust
pub struct HybridEmbedder {
    model: Bm42Model,         // déjà retourne (hidden, attn_probs)
    tokenizer: Mutex<Tokenizer>,
    device: Device,
}

pub struct HybridResult {
    pub dense: Vec<f32>,       // 384 dims, normalized L2
    pub sparse: SparseVector,  // ~10-30 dims actives, token_ids
}

impl HybridEmbedder {
    pub fn embed_hybrid_sync(&self, texts: &[String]) -> Result<Vec<HybridResult>, EmbedError>;
}

// Implémente les deux traits existants
impl Embedder for HybridEmbedder { ... }        // appelle embed_hybrid, retourne .dense
impl SparseEmbedder for HybridEmbedder { ... }  // appelle embed_hybrid, retourne .sparse
```

### Changements

| Fichier | Action |
|---|---|
| `src/hybrid_embedder.rs` | Nouveau (~150 lignes) — combine le mean pooling de `candle_embedder.rs` et l'extraction CLS de `bm42_embedder.rs` |
| `src/lib.rs` | Ajouter `pub mod hybrid_embedder` |
| `src/catalog.rs` | Remplacer `embedder: Box<dyn Embedder>` + `sparse_embedder: Box<dyn SparseEmbedder>` par un seul `HybridEmbedder` (ou garder les deux traits, le `HybridEmbedder` implémente les deux) |

### Mean pooling (rappel `candle_embedder.rs`)

```rust
// hidden_states: [batch, seq_len, hidden_size]
// attention_mask: [batch, seq_len]
let mask = attention_mask.unsqueeze(2)?.broadcast_as(hidden.shape())?;
let masked = (hidden * mask)?;
let sum = masked.sum(1)?;
let count = attention_mask.sum(1)?.unsqueeze(1)?;
let mean = (sum / count)?;
// L2 normalize
let norm = mean.sqr()?.sum(1)?.sqrt()?.unsqueeze(1)?;
mean / norm
```

### Ce qui ne change pas

- Le modèle est le même (all-MiniLM-L6-v2)
- `Bm42Model` retourne déjà les deux (`hidden_states` + `attention_probs`)
- L'API pipeline (queue, processors) ne change pas
- Zéro impact sur le format de stockage

### Gain

- **2x moins de mémoire modèle** (~22MB au lieu de 44MB)
- **2x moins de forward passes** (le plus gros coût CPU)
- Même qualité exacte (mêmes poids, même calcul)

### Risques

Aucun risque technique. C'est du refactoring pur — les deux calculs sont déjà implémentés et testés séparément.

### Tests

- Vérifier que `embed_hybrid` produit les mêmes vecteurs dense que `CandleEmbedder` (à l'erreur flottante près)
- Vérifier que `embed_hybrid` produit les mêmes sparse que `Bm42Embedder`
- Benchmark : mesurer le speedup (devrait être ~1.8-2x)

---

## 2. Sub-word Merging — fusionner les sous-mots WordPiece

### Complexité : moyenne
### Inconnues design : stratégie d'agrégation des poids

### Le problème

Le tokenizer WordPiece découpe en sous-mots :
```
"programming" → ["program", "##ming"]     → token_ids [2565, 5765]
"hello"       → ["hello"]                 → token_id  [7592]
"unbelievable"→ ["un", "##bel", "##ie", "##va", "##ble"] → 5 token_ids
```

Aujourd'hui chaque sous-mot est une dimension séparée dans le sparse vector. Ça fonctionne car query et document passent par le même tokenizer → mêmes token_ids → dot product correct.

Mais :
- Le vocabulaire WordPiece est ~30k tokens → le sparse vector peut théoriquement avoir 30k dimensions
- Des mots différents partagent des sous-mots ("programming" et "programmer" partagent "program")
- Le nombre de dimensions actives est plus élevé que nécessaire (les `##` fragments sont du bruit sémantique)

### La solution

Après extraction des attention weights, regrouper les sous-mots en mots entiers :

```
Tokens:   [CLS]  "program"  "##ming"  "language"  [SEP]
Weights:   skip    0.15       0.08      0.12       skip
                    ↓           ↓
               "programming" = merge(0.15, 0.08)
```

Le sparse vector final utilise des **word_ids** (hash du mot entier) au lieu de token_ids.

### Stratégies d'agrégation

C'est la principale inconnue design. Options :

| Stratégie | Formule | Avantage | Inconvénient |
|---|---|---|---|
| **Sum** | 0.15 + 0.08 = 0.23 | Simple, conserve l'énergie totale | Les mots longs (plus de sous-mots) ont un avantage injuste |
| **Max** | max(0.15, 0.08) = 0.15 | Prend le sous-mot le plus "important" | Perd l'information des autres sous-mots |
| **Mean** | (0.15 + 0.08) / 2 = 0.115 | Équitable quelle que soit la longueur | Dilue le signal pour les mots importants |
| **First** | 0.15 (le stem) | Privilégie la racine sémantique | Ignore complètement les suffixes |
| **Weighted first** | 0.7 × 0.15 + 0.3 × 0.08 | Compromis stem/suffix | Un hyperparamètre de plus |

**Recommandation** : commencer par **Sum** (c'est ce que fait SPLADE). Si les mots longs dominent injustement, passer à **Mean**.

### Le word_id

On ne peut plus utiliser les token_ids WordPiece comme dimensions (ils sont par sous-mot). Options :

1. **Hash du mot** : `djb2("programming") % 2^32` → u32. Risque de collision négligeable sur ~30k mots de vocabulaire.
2. **Index dans un vocabulaire custom** : maintenir un mapping `word → id`. Complique la sérialisation.
3. **Hash du premier token_id** : utiliser le token_id du stem comme clé. Ambigu si deux mots partagent le même stem.

**Recommandation** : hash du mot complet (option 1). Simple, déterministe, sans état.

### Accès aux word boundaries

Le tokenizer HuggingFace fournit `encoding.get_word_ids()` → `Option<u32>` par token :
```
Tokens:   [CLS]  "program"  "##ming"  "language"  [SEP]
Word IDs:  None    Some(0)    Some(0)    Some(1)    None
```

Les tokens avec le même `word_id` appartiennent au même mot. Les special tokens ont `None`.

### Reconstruction du mot entier

```rust
// encoding.get_offsets() donne les (start, end) dans le texte original
let offsets = encoding.get_offsets();
for word_id in unique_word_ids {
    let (first_start, _) = offsets[first_token_of_word];
    let (_, last_end) = offsets[last_token_of_word];
    let word = &original_text[first_start..last_end];  // "programming"
}
```

### Pipeline modifié

```
Texte → Tokenizer → BERT forward → attention CLS weights
  → pour chaque position:
      - skip special tokens
      - group by word_id (via encoding.get_word_ids())
      - merge weights per word (sum/mean/max)
      - compute word hash: djb2(original_word) as u32
  → sort by word hash → SparseVector { indices: word_hashes, values: merged_weights }
```

### Changements

| Fichier | Action |
|---|---|
| `src/bm42_embedder.rs` | Modifier `embed_sparse_sync()` — regrouper par `word_id` au lieu d'accumuler par `token_id` (~30 lignes changées) |
| Rien d'autre | Le format `SparseVector { indices: Vec<u32>, values: Vec<f32> }` reste identique |

### Impact sur l'extension sparse_vector

Aucun. L'extension stocke/cherche par `indices: Vec<u32>` sans interpréter la sémantique des indices. Que ce soit des token_ids ou des word_hashes, le dot product fonctionne pareil.

### Gains attendus

- **Moins de dimensions actives** : "unbelievable" = 1 dimension au lieu de 5
- **Meilleure sémantique** : le mot entier est l'unité de sens, pas les fragments
- **Index plus compact** : moins d'entrées dans les posting lists

### Risques

- **Collisions de hash** : théoriquement possible mais négligeable (32 bits, <100k mots uniques en pratique)
- **Perte de précision sub-word** : "program" et "programming" ne partagent plus de dimensions. C'est un **trade-off** : on gagne en précision sémantique mais on perd le matching partiel par stem. En pratique c'est un gain net car le dense vector capture déjà la similarité sémantique.

### Tests

- Comparer le nnz (nombre de dimensions actives) avant/après : devrait baisser de ~30-50%
- Vérifier que deux textes avec le même mot ("rust" dans les deux) partagent la même dimension
- Vérifier la stabilité (déterminisme du hash)
- A/B test qualité de recherche sur un jeu de données réel

---

## 3. V2 Persistance — format on-disk par token + mmap

### Complexité : élevée
### Inconnues design : format fichier, stratégie de cache, concurrency

### Le problème

La persistance V1 utilise bincode : `serialize(&SparseIndex)` → un gros blob `sparse.bin`.

Pour N documents avec M tokens uniques et ~20 dims actives/doc :
- Taille mémoire : `N × 20 × 12 bytes` (posting: u64 + f32 = 12 bytes/entry)
- 10k docs × 20 = 200k entries × 12 = 2.4 MB → OK
- 100k docs × 20 = 2M entries × 12 = 24 MB → gros blob, slow load
- 1M docs × 20 = 20M entries × 12 = 240 MB → problème

Au-delà de ~50-100k docs, charger tout en mémoire au cold start devient lent et gourmand.

### Architecture cible

```
{index_path}/
├── meta.bin          # header (version, num_docs, num_tokens)
├── posting/
│   ├── index.bin     # token_id → (offset, length) dans data.bin
│   └── data.bin      # posting lists concaténées: [(node_id: u64, weight: f32), ...]
└── vectors/
    ├── index.bin     # node_id → (offset, length) dans data.bin
    └── data.bin      # vecteurs concaténés: [(token_id: u32, weight: f32), ...]
```

### Format posting/data.bin

```
┌─────────────────────────────────────────────┐
│ token_42 postings (offset=0, len=3)         │
│ [node_7: 0.35] [node_12: 0.58] [node_99: 0.21] │
├─────────────────────────────────────────────┤
│ token_108 postings (offset=36, len=2)       │
│ [node_3: 0.72] [node_45: 0.11]             │
├─────────────────────────────────────────────┤
│ ...                                         │
└─────────────────────────────────────────────┘
```

Chaque entry = 12 bytes (u64 node_id + f32 weight). Les posting lists sont triées par node_id pour binary search rapide.

### Format posting/index.bin

```
┌────────────────────────────────┐
│ num_tokens: u32                │
├────────────────────────────────┤
│ token_id: u32 | offset: u64 | count: u32  │  ← 16 bytes/entry
│ token_id: u32 | offset: u64 | count: u32  │
│ ...                                        │
└────────────────────────────────┘
```

Trié par token_id → binary search O(log M) pour trouver la posting list.

### Accès mmap

```rust
use memmap2::Mmap;

struct OnDiskSparseIndex {
    posting_index: Mmap,   // posting/index.bin
    posting_data: Mmap,    // posting/data.bin
    vector_index: Mmap,    // vectors/index.bin
    vector_data: Mmap,     // vectors/data.bin
    // Cache LRU pour les posting lists hot
    cache: Mutex<LruCache<u32, Vec<(u64, f32)>>>,
}
```

Le kernel gère le page cache — seules les pages accédées sont en RAM physique. Pour un index de 240MB, si seuls 1000 tokens sont "hot", on consomme ~quelques MB de RAM réelle.

### Recherche

```rust
fn search(&self, query: &SparseVector, limit: usize) -> Vec<(u64, f32)> {
    let mut scores: HashMap<u64, f32> = HashMap::new();

    for (i, &token_id) in query.indices.iter().enumerate() {
        let postings = self.get_posting_list(token_id);  // mmap read ou cache hit
        let q_weight = query.values[i];
        for &(node_id, d_weight) in &postings {
            *scores.entry(node_id).or_default() += q_weight * d_weight;
        }
    }

    // top-k avec min-heap
    top_k(scores, limit)
}

fn get_posting_list(&self, token_id: u32) -> Vec<(u64, f32)> {
    // 1. Check LRU cache
    if let Some(cached) = self.cache.lock().unwrap().get(&token_id) {
        return cached.clone();
    }
    // 2. Binary search dans posting_index mmap
    let (offset, count) = binary_search_index(&self.posting_index, token_id);
    // 3. Slice posting_data mmap
    let data = &self.posting_data[offset..offset + count * 12];
    let postings = parse_postings(data);
    // 4. Insert in LRU cache
    self.cache.lock().unwrap().put(token_id, postings.clone());
    postings
}
```

### Mutations (insert/delete)

Les mutations ne modifient pas le fichier mmap directement (trop coûteux — il faudrait réécrire toute la posting list). À la place :

```rust
struct OnDiskSparseIndex {
    // ... mmaps ...

    // WAL (Write-Ahead Log) en mémoire
    pending_inserts: HashMap<u64, SparseVector>,  // node_id → vector
    pending_deletes: HashSet<u64>,                 // node_ids supprimés
    dirty: bool,
}
```

- **Insert** : ajouter dans `pending_inserts`
- **Delete** : ajouter dans `pending_deletes`
- **Search** : merger les résultats mmap + pending (overlay)
- **Commit** : réécrire les fichiers on-disk (merge sort des posting lists)

### Compaction

Le commit est un **merge compaction** :
1. Lire toutes les posting lists depuis mmap
2. Appliquer les deletes (skip) et inserts (merge)
3. Réécrire posting/index.bin + posting/data.bin
4. Réécrire vectors/index.bin + vectors/data.bin
5. `msync` + renommer atomiquement (pour crash safety)
6. Re-ouvrir les mmaps

C'est O(N) mais ne se fait qu'au commit, pas à chaque mutation. Avec le dirty flag + lazy commit existant, ça arrive rarement.

### Cache LRU

```rust
use lru::LruCache;

// Taille du cache = nombre de posting lists en mémoire
// 1000 entries × ~1KB/posting = ~1MB de cache
let cache: LruCache<u32, Vec<(u64, f32)>> = LruCache::new(
    NonZeroUsize::new(1000).unwrap()
);
```

Les tokens fréquents (stopwords, termes communs) restent en cache. Les tokens rares sont lus depuis mmap (page cache kernel).

### Filtered search avec mmap

Pour `allowed_ids`, deux approches :
1. **Post-filter** : calculer tous les scores, puis filtrer. Simple mais O(N) si la posting list est longue.
2. **Skip-scan** : les posting lists sont triées par node_id → binary search pour trouver les allowed_ids dans chaque posting list. O(K × log P) où K = |allowed_ids| et P = taille posting list.

**Recommandation** : post-filter pour V2 (simple), skip-scan en V3 si profiling montre un bottleneck.

### Changements

| Fichier | Action |
|---|---|
| `sparse_vector/rust/src/ondisk.rs` | Nouveau (~400-500 lignes) — OnDiskSparseIndex, mmap, binary search, LRU cache, WAL |
| `sparse_vector/rust/src/handle.rs` | Modifier — choisir V1 (bincode) ou V2 (ondisk) selon un flag ou la taille |
| `sparse_vector/rust/src/bridge.rs` | Possiblement inchangé — l'API bridge reste la même (add, delete, search, commit) |
| `sparse_vector/rust/Cargo.toml` | Ajouter deps: `memmap2`, `lru` |

### Migration V1 → V2

Au `open_sparse_index()` :
1. Si `sparse.bin` existe et `posting/` n'existe pas → lire V1, réécrire en V2, supprimer `sparse.bin`
2. Si `posting/` existe → ouvrir V2 directement

### Dépendances nouvelles

- `memmap2` (~30KB, mature, cross-platform)
- `lru` (~10KB, simple LRU cache)
- Ni l'un ni l'autre ne posent de problème pour WASM (on peut feature-gate mmap et utiliser le V1 bincode en WASM)

### Gains attendus

| Docs | V1 (bincode) RAM | V2 (mmap) RSS |
|---|---|---|
| 10k | 2.4 MB | ~0.5 MB (hot pages) |
| 100k | 24 MB | ~2 MB |
| 1M | 240 MB | ~5-10 MB |

Cold start : V1 = lire + deserialize tout → V2 = ouvrir mmaps en ~1ms.

### Risques

- **Complexité merge compaction** : le commit V2 est plus complexe que `bincode::serialize`. Faut gérer le crash safety (rename atomique).
- **Fragmentation** : les deletes créent des trous non récupérés entre deux compactions.
- **Portabilité WASM** : mmap n'existe pas en WASM → garder V1 en fallback.
- **Endianness** : le format on-disk doit être little-endian explicite (pas de problème en pratique, tout est x86/ARM-LE).

### Tests

- Benchmark insert 100k docs + search → comparer latence V1 vs V2
- Test crash recovery : kill pendant commit, vérifier intégrité
- Test migration V1 → V2
- Test concurrent reads (mmap est thread-safe en lecture)

---

## 4. SPLADE — modèle dédié à expansion de termes

### Complexité : très élevée
### Inconnues design : choix du modèle, taille, quality vs perf, training data

### Le problème

BM42 utilise un modèle d'embedding classique (all-MiniLM-L6-v2) et extrait les attention weights comme proxy pour l'importance des tokens. C'est un hack astucieux mais ce n'est pas ce pour quoi BERT a été entraîné — les attention weights ne sont pas des indicateurs d'importance appris.

SPLADE (Sparse Lexical AnD Expansion) est un modèle **spécifiquement entraîné** pour produire des sparse vectors de haute qualité. Il utilise la tête MLM (Masked Language Model) de BERT pour prédire la probabilité de chaque token du vocabulaire, même ceux absents du texte original.

### BM42 vs SPLADE — différences fondamentales

| | BM42 (actuel) | SPLADE |
|---|---|---|
| Base | Embedding model (all-MiniLM-L6-v2, 22MB) | MLM model (splade-v3, 65-130MB) |
| Signal | Attention weights (proxy) | Log-probabilités MLM (appris) |
| Expansion | Non — seuls les tokens présents | Oui — prédit des tokens pertinents absents |
| Exemple | "dog" → {dog: 0.3} | "dog" → {dog: 0.8, canine: 0.4, pet: 0.3, puppy: 0.2} |
| Vocab | Tokens présents (~5-15 dims) | Full vocab activé (~50-200 dims) |
| Training | Pas de fine-tuning spécifique | Entraîné sur MS MARCO + distillation |
| Qualité | Bonne (meilleur que BM25 seul) | Excellente (SOTA sparse retrieval) |

### L'expansion de termes en détail

C'est le super-pouvoir de SPLADE. Pour "the capital of France" :

```
BM42:    {the: 0.02, capital: 0.25, of: 0.01, france: 0.35}
         → 4 dimensions, pas de synonymes

SPLADE:  {france: 0.9, capital: 0.7, paris: 0.6, french: 0.4,
          city: 0.3, europe: 0.2, country: 0.15, ...}
         → ~50-100 dimensions, inclut des termes liés non présents dans le texte
```

Le document "Paris is the capital" et la query "capital of France" matchent via le terme "paris" expansé par SPLADE, alors que BM42 ne les connecte que via "capital".

### Pipeline SPLADE

```
Texte → Tokenizer → BERT+MLM head forward
  → logits: [batch, seq_len, vocab_size]        (~30522 tokens)
  → max pooling across seq_len → [batch, vocab_size]
  → log(1 + ReLU(logits)) → SPLADE activation   (sparsification)
  → seuil (> 0) → SparseVector { indices, values }
```

### Modèles disponibles

| Modèle | Taille | Qualité (MRR@10 MS MARCO) | Notes |
|---|---|---|---|
| naver/splade-v3 | ~130MB | 0.395 | SOTA, distillation cross-encoder |
| naver/splade-cocondenser-ensembledistil | ~130MB | 0.383 | Populaire, bien documenté |
| naver/efficient-splade-V-large | ~130MB | 0.380 | Variante "efficace" (moins de dims actives) |
| prithivida/Splade_PP_en_v1 | ~65MB | ~0.370 | Plus petit, basé sur DistilBERT |

### Implémentation candle

Le modèle SPLADE est un BERT standard + une tête MLM (une couche Linear `hidden_size → vocab_size`). Il faut :

1. **Charger les poids** : le safetensors contient les poids BERT + `cls.predictions.transform.dense.weight/bias` + `cls.predictions.decoder.weight/bias`
2. **Forward** : BERT hidden_states → transform → activation → decoder → logits
3. **SPLADE activation** : `log(1 + ReLU(max_pool(logits)))`

```rust
pub struct SpladeModel {
    bert: Bm42Model,          // ou BertModel standard
    transform_dense: Linear,   // hidden_size → hidden_size
    transform_ln: LayerNorm,
    decoder: Linear,           // hidden_size → vocab_size
    decoder_bias: Tensor,      // vocab_size
}

impl SpladeModel {
    pub fn forward(&self, input_ids: &Tensor, ...) -> Result<Tensor> {
        let (hidden, _attn) = self.bert.forward(input_ids, ...)?;
        // MLM head
        let h = self.transform_dense.forward(&hidden)?;
        let h = h.gelu()?;
        let h = self.transform_ln.forward(&h)?;
        let logits = self.decoder.forward(&h)?
            .broadcast_add(&self.decoder_bias)?;
        // Max pooling across seq_len + SPLADE activation
        let pooled = logits.max(1)?;  // [batch, vocab_size]
        let activated = pooled.relu()?.log1p()?;  // log(1 + ReLU(x))
        Ok(activated)
    }
}
```

### Problèmes à résoudre

#### 1. Taille du modèle

130MB vs 22MB actuel. Ça triple l'empreinte mémoire. Pour WASM c'est problématique (download initial + mémoire).

**Options** :
- Utiliser un modèle DistilBERT-based (~65MB) au prix d'un peu de qualité
- Quantization INT8 du modèle SPLADE (divise par ~2, nécessite support candle)
- Garder BM42 en WASM, SPLADE en natif seulement

#### 2. Vitesse d'inférence

La tête MLM ajoute une grosse multiplication matrice `hidden_size × vocab_size` (768 × 30522 = ~23M multiplications). C'est ~2-3x plus lent que le forward BERT seul.

**Mitigation** : la sparsité naturelle — on peut court-circuiter avec un seuil bas et ne garder que les top-K activations.

#### 3. Plus de dimensions actives

SPLADE produit ~50-200 dimensions actives (vs ~5-15 pour BM42). L'index inverted est plus dense, les posting lists plus longues, le dot product plus coûteux.

**Impact sur sparse_vector extension** : les posting lists par token seront ~10x plus longues. La V1 bincode tient moins bien. C'est un argument fort pour faire la V2 mmap avant SPLADE.

#### 4. Expansion = risque de faux positifs

L'expansion de termes peut introduire du bruit. "dog" → "puppy" est utile, mais parfois le modèle active des termes non pertinents. Le seuillage et le tuning du paramètre de sparsité sont importants.

#### 5. Pas de single forward pass

Contrairement au combo BM42 + dense (même modèle BERT), SPLADE nécessite un modèle séparé entraîné spécifiquement. On ne peut pas faire dense + SPLADE en un seul forward (modèles différents, poids différents).

Sauf si on utilise un modèle **hybride** comme `prithivida/Splade_PP_en_v1` qui produit les deux. Mais la qualité dense de ces modèles est inférieure aux modèles d'embedding dédiés.

### Intégration progressive

**Étape 1** — Implémenter `SpladeEmbedder` qui implémente `SparseEmbedder` :
- Chargement du modèle SPLADE via candle
- Forward + SPLADE activation
- Feature gate `splade` (séparé de `candle-embedder`)

**Étape 2** — Rendre le choix configurable dans `CatalogConfig` :
```yaml
knowledge_bases:
  main:
    sparse_model: "bm42"       # ou "splade"
    sparse_model_id: "naver/splade-v3"  # optionnel, override
```

**Étape 3** — Benchmarker sur un dataset réel :
- Comparer BM42 vs SPLADE en isolation (recall@10, MRR@10)
- Comparer le gain en fusion 3-way (dense + BM25 + sparse)
- Mesurer le surcoût en latence et mémoire

### Changements

| Fichier | Action |
|---|---|
| `src/splade_model.rs` | Nouveau (~200 lignes) — BERT + MLM head + SPLADE activation |
| `src/splade_embedder.rs` | Nouveau (~180 lignes) — SpladeEmbedder implémentant SparseEmbedder |
| `Cargo.toml` | Feature flag `splade` (nouvelle feature, pas de deps supplémentaires) |
| `src/lib.rs` | Conditional modules |
| `src/config.rs` | Ajouter `sparse_model` dans KBConfig |
| `src/catalog.rs` | Factory pattern pour choisir BM42 vs SPLADE |

### Gains attendus

- **Qualité** : +5-15% en recall/MRR sur des benchmarks standard (MS MARCO, BEIR)
- **Expansion de termes** : résout le vocabulary mismatch que ni BM25 ni BM42 ne gèrent
- **3-way fusion** : le sparse signal de meilleure qualité améliore la fusion

### Risques

- **Qualité incertaine dans notre contexte** : les benchmarks SPLADE sont sur MS MARCO (passages web anglais). Pour du code, des docs techniques, ou du français, la qualité pourrait être différente.
- **Taille modèle** : 130MB en WASM est problématique
- **Latence** : ~2-3x plus lent par texte que BM42
- **Maintenance** : deux modèles à charger, configurer, et maintenir
- **Pas de single pass** : annule le gain de l'évolution 1 si on utilise SPLADE au lieu de BM42

### Tests

- Tests unitaires : forward + activation → sparse vector non vide, dimensions ⊂ [0, vocab_size)
- Tests intégration : embed → search → résultats pertinents
- Benchmark latence : BM42 vs SPLADE (ms/texte)
- Benchmark qualité : A/B sur un dataset réel avec métriques recall@K
- Test WASM : vérifier que le feature gate fonctionne (SPLADE désactivé en WASM)

---

## Résumé : ordre de priorité recommandé

| # | Évolution | Effort | Gain immédiat | Prérequis |
|---|---|---|---|---|
| 1 | Single forward pass | ~1 jour | 2x perf, -22MB RAM | Aucun |
| 2 | Sub-word merging | ~1-2 jours | Index plus compact, meilleure sémantique | Aucun (indépendant de 1) |
| 3 | V2 persistance mmap | ~3-5 jours | Scale à 1M+ docs, cold start instant | Recommandé avant SPLADE |
| 4 | SPLADE | ~5-7 jours | +5-15% qualité retrieval | V2 persistance (posting lists plus denses) |

Les évolutions 1 et 2 sont indépendantes et peuvent être faites en parallèle. L'évolution 3 est un prérequis pratique pour 4 (SPLADE produit des sparse vectors plus denses qui stressent la V1).
