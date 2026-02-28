# 11 — Sparse Vectors : reflexion et options

## Contexte

Qdrant propose les sparse vectors via BM42 et SPLADE pour du hybrid search ameliore. On veut comprendre ce que ca implique et si c'est faisable dans notre stack (Rust / C++ / Node / WASM).

## Qu'est-ce qu'un sparse vector ?

Un vecteur dense (classique) : `[0.12, -0.34, 0.56, ...]` — 384 ou 768 dimensions, toutes non-nulles.

Un vecteur sparse : `{token_id: poids, token_id: poids, ...}` — dimension = taille du vocabulaire (~30k), mais 99% des valeurs sont zero. On ne stocke que les (index, value) non-nulles.

Avantage : combine le meilleur du lexical (exact match, interpretable) et du semantique (expansion de termes, contexte).

## Les approches

### BM25 (ce qu'on a)

- TF × IDF classique
- Tantivy le fait nativement (notre fork ld-tantivy)
- Tres rapide, zero modele
- Faiblesse : sur des petits chunks (RAG), TF ≈ 1 toujours → perd de l'information

### BM42 (Qdrant)

Remplace TF par les **poids d'attention** d'un transformer :

```
score(D,Q) = Σ IDF(qi) × Attention(CLS, qi)
```

1. Passer le texte dans un petit transformer (all-MiniLM-L6-v2, ~22MB quantize)
2. Extraire la ligne d'attention du token [CLS] → un poids par token
3. Re-merger les sous-mots WordPiece (somme des poids)
4. Resultat : sparse vector `{mot: poids_attention}`
5. IDF calcule cote DB en temps reel

**Avantage :** leger, rapide, interpretable, meilleur que BM25 sur petits chunks.
**Limite :** Qdrant eux-memes disent que BM42 ne bat pas BM25 seul — c'est pour du hybrid.

### SPLADE

Modele dedie (DistilBERT + MaskedLM head) entraine pour le retrieval sparse :

1. Forward pass dans un DistilBERT
2. Logits du MLM head → ReLU + log(1+x)
3. Max pooling sur la sequence → un score par token du vocabulaire
4. Resultat : sparse vector ~30k dimensions (mais ~100-300 non-zero)

**Avantage :** meilleure qualite que BM42, expansion de termes (le modele ajoute des tokens semantiquement lies).
**Limite :** modele plus lourd (~65-130MB quantize), plus lent.

### BGE-M3

Un seul modele produit dense + sparse + colbert en un seul forward pass.

**Avantage :** un modele pour tout.
**Limite :** ~300MB quantize, trop lourd pour WASM casual.

## Comparaison

| | BM25 | BM42 | SPLADE | BGE-M3 sparse |
|---|---|---|---|---|
| Qualite petits chunks | Basse | Bonne | Excellente | Excellente |
| Qualite grands docs | Haute | Basse | Haute | Haute |
| Modele requis | Aucun | ~22MB | ~65-130MB | ~300MB |
| Vitesse inference | Instantane | Rapide | Lent | Lent |
| Expansion termes | Non | Non | Oui | Oui |
| WASM viable | Oui (natif) | Oui | Limite | Non |

## Notre stack actuelle

### Embeddings denses (deja en place)

- **Candle** (Rust, pure Rust, WASM-compatible) pour inference locale
- **all-MiniLM-L6-v2** (384d, ~22MB) — le modele par defaut pour WASM
- **bge-base-en-v1.5** (768d, ~110MB) — le modele par defaut pour qualite
- Trait `Embedder` generique (candle, TEI server, Ollama, OpenAI)
- Hybrid search existant : vector cosine + BM25 Tantivy, fusion RRF/Weighted/Boost

### BM25 (deja en place)

- Tantivy (ld-tantivy) avec NgramContainsQuery, fuzzy, filtres pre-filter
- 15 tests E2E, 1062 tests Rust

### Sparse vectors : rien pour l'instant

Pas de stockage sparse, pas de generation sparse, pas de search sparse.

## Contrainte : tourner partout (Rust / C++ / Node / WASM)

C'est la contrainte clef. Voici ce qui fonctionne par plateforme :

| Runtime | Natif (Linux/Mac/Win) | Node.js natif | WASM browser |
|---|---|---|---|
| **candle** | Oui | Oui (via napi-rs) | **Oui** (prouve, demos HF) |
| **fastembed-rs** (ort) | Oui | Oui | **Non** (C++ ONNX Runtime) |
| **tract** (ONNX pure Rust) | Oui | Oui | Oui mais perf mediocre sur transformers |
| **ort + ort-candle backend** | Oui | Oui | Experimental |

**Candle est le seul runtime qui fonctionne partout y compris WASM en production.**

Et on l'utilise deja ! Le `CandleEmbedder` dans rag3weaver utilise exactement les memes modeles que BM42.

## Options d'implementation

### Option A : BM42-style via candle (recommandee)

**Effort : moyen (~200-300 lignes Rust)**

On a deja candle + all-MiniLM-L6-v2 dans rag3weaver. Il faut :

1. Modifier le forward pass pour extraire les **poids d'attention du [CLS]** (candle donne acces aux tenseurs d'attention)
2. Re-merger les sous-mots WordPiece (somme des poids)
3. Produire un sparse vector `Vec<(u32, f32)>` — (token_index, attention_weight)
4. Stocker dans Tantivy comme index inverse (token_id → doc_id + weight)
5. Au query time : meme extraction d'attention sur la query, dot product avec IDF

**Avantages :**
- Meme modele que le dense (pas de download supplementaire)
- ~22MB quantize, viable en WASM
- Candle WASM est prouve (demos HF en production)
- IDF calculable en temps reel par Tantivy (on a deja l'infra)

**Limites :**
- Pas d'expansion de termes (contrairement a SPLADE)
- Qualite inferieure a SPLADE sur certains benchmarks
- BM42 a ete critique (Qdrant a retropdale un peu)

### Option B : SPLADE via candle (qualite maximale)

**Effort : eleve (~500+ lignes Rust)**

HuggingFace TEI implemente deja SPLADE pooling en candle (code de reference disponible). Il faut :

1. Charger un modele DistilSPLADE ONNX → convertir en safetensors pour candle
2. Forward pass MaskedLM head
3. ReLU + log(1+x) sur les logits
4. Max pooling → sparse vector ~30k dimensions
5. Stocker + rechercher dans Tantivy

**Avantages :**
- Meilleure qualite, expansion de termes
- Code de reference dans TEI

**Limites :**
- Modele ~65-130MB quantize (acceptable natif, limite en WASM)
- Plus lent a l'inference
- Modele separe du dense embedding

### Option C : Support generique sparse dans Tantivy (stockage seul)

**Effort : faible (~100 lignes Rust)**

Ne pas generer les sparse vectors dans notre stack — juste les stocker et les chercher. L'utilisateur les genere cote serveur (fastembed-rs, TEI, etc.) et nous les passe.

1. API bridge : `add_sparse_vector(doc_id, Vec<(u32, f32)>)`
2. Index inverse dans Tantivy : chaque token_id → liste de (doc_id, weight)
3. Search : dot product sparse query × sparse doc via index inverse

**Avantages :**
- Decouple generation et stockage
- L'utilisateur choisit son modele
- Rapide a implementer

**Limites :**
- Pas de generation embarquee (pas de valeur en WASM standalone)
- L'utilisateur doit gerer le pipeline d'embedding sparse lui-meme

### Option D : Les trois (incrementale)

1. **V1 :** Option C — support stockage/search sparse dans Tantivy (utile immediatement)
2. **V2 :** Option A — BM42 via candle (generation embarquee, WASM-compatible)
3. **V3 :** Option B — SPLADE optionnel (qualite maximale, natif seulement)

## Etat des lieux : nos deux stacks d'embedding

### Stack serveur (Rust) — CandleEmbedder

- **Runtime :** candle (pure Rust, HuggingFace)
- **Modeles :** all-MiniLM-L6-v2 (384d, ~23MB) / bge-base-en-v1.5 (768d, ~110MB)
- **Fichier :** `rag3weaver/src/candle_embedder.rs`
- **Trait :** `Embedder` generique — supporte candle, TEI server, Ollama, OpenAI, CallbackEmbedder
- **WASM :** candle compile en WASM (prouve par HuggingFace demos)

### Stack browser (JS) — Transformers.js

- **Runtime :** `@xenova/transformers@2.17.2` + `onnxruntime-web@1.19.0`
- **Modeles :** Xenova/all-MiniLM-L6-v2 (384d), bge-small/base-en-v1.5, gte-small, multilingual-e5-small
- **Fichier :** `ragforge-core-exp-kuzu/kuzu-wasm-exp/dist/browser-rag.js`
- **Backends :** WebGPU (fp16, rapide) ou WASM CPU (fp32, fallback)
- **Aussi :** GLiNER.js pour NER zero-shot (~183MB int8, WASM CPU)

### Tests Playwright WASM

3 fichiers de tests E2E browser dans `packages/rag3db/tools/wasm/test/browser/` :

| Test | Ce qu'il valide |
|---|---|
| `rag3weaver.spec.js` | API Weaver WASM, MockEmbedder (zero vectors), opaque handles, async drain/search |
| `idbfs.spec.js` | Persistance IDBFS, Tantivy FTS (contains/fuzzy/phrase/regex), vector HNSW 4D, reload |
| `threading.spec.js` | std::thread, rayon par_iter, futures::ThreadPool — validation pthreads emscripten |

Les tests WASM utilisent un `MockEmbedder` (zero vectors, dim 4) — le provider d'embedding est pluggable.

### Implications pour les sparse vectors

**Fait important :** Les deux stacks utilisent le meme modele (all-MiniLM-L6-v2) mais via des runtimes differents :

| | Serveur (Rust) | Browser (JS) |
|---|---|---|
| Runtime | candle | Transformers.js (ONNX) |
| Modele | all-MiniLM-L6-v2 | Xenova/all-MiniLM-L6-v2 |
| Acces attention | Oui (tenseurs candle) | Oui (ONNX output nodes) |
| Forward pass | Modifiable (Rust) | Modifiable (JS pipeline) |

Pour BM42, il faudrait extraire les poids d'attention du [CLS] dans **les deux runtimes** :
- **candle (Rust)** : modifier le forward pass pour retourner les attention weights en plus du mean pooling
- **Transformers.js (JS)** : configurer le pipeline pour exposer les attention outputs (ONNX supporte `output_attentions`)

Un seul forward pass → dense embedding + sparse BM42 vector, dans les deux stacks. Le modele (~23MB) est le meme, zero download supplementaire.

## Questions ouvertes

1. **Stockage sparse dans Tantivy** : utiliser l'index inverse existant (avec poids flottants) ou creer une structure dediee ?
2. **Fusion hybrid 3-way** : dense + BM25 + sparse — comment fusionner les 3 scores ? RRF s'etend bien a 3+ listes.
3. **IDF temps reel** : Tantivy calcule deja l'IDF pour BM25. Peut-on le reutiliser pour BM42 ?
4. **Quantisation attention** : les poids d'attention sont des floats — peut-on les quantiser en u8 pour economiser de la memoire ?

## Fichiers cles pour l'implementation

| Composant | Fichier |
|---|---|
| Candle embedder (Rust) | `rag3weaver/src/candle_embedder.rs` |
| Trait Embedder | `rag3weaver/src/embedder.rs` |
| Hybrid search + fusion | `rag3weaver/src/search.rs`, `rag3weaver/src/fusion.rs` |
| Tantivy index (stockage) | `ld-tantivy/tantivy_fts/rust/src/handle.rs` |
| Tantivy bridge | `ld-tantivy/tantivy_fts/rust/src/bridge.rs` |
| Extension C++ | `extension/tantivy_fts/src/` |
| Browser RAG (Transformers.js) | `ragforge-core-exp-kuzu/kuzu-wasm-exp/dist/browser-rag.js` |
| Tests Playwright WASM | `tools/wasm/test/browser/{rag3weaver,idbfs,threading}.spec.js` |
| Playwright config | `tools/wasm/playwright.config.js` |

## Sources

- [BM42: New Baseline for Hybrid Search (Qdrant)](https://qdrant.tech/articles/bm42/)
- [Modern Sparse Neural Retrieval (Qdrant)](https://qdrant.tech/articles/modern-sparse-neural-retrieval/)
- [Sparse Vectors and Inverted Indexes (Qdrant)](https://qdrant.tech/course/essentials/day-3/sparse-vectors/)
- [fastembed-rs (Qdrant)](https://github.com/Anush008/fastembed-rs)
- [candle (HuggingFace)](https://github.com/huggingface/candle)
- [TEI SPLADE pooling (HuggingFace)](https://github.com/huggingface/text-embeddings-inference)
- [ort Rust ONNX Runtime](https://github.com/pykeio/ort)
- [tract ONNX pure Rust](https://github.com/sonos/tract)
