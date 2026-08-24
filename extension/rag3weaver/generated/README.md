# `generated/` — code produit par machine, pas par nous

Ce dossier contient du code **généré**, conservé dans git pour éviter d'imposer une étape
de conversion de 2,2 Go à quiconque compile le projet. Rien ici n'est écrit à la main et
rien ne doit y être édité : toute modification serait perdue à la prochaine régénération.

Ce dossier n'est **pas** dans l'arbre de modules du crate. Il ne compile pas tant que
`burn` n'est pas ajouté en dépendance (optionnelle) et que le fichier n'est pas déclaré
dans `lib.rs`.

---

## `bge_m3_onnx.rs`

Traduction mécanique, par `burn-onnx`, du graphe ONNX de **[BAAI/bge-m3](https://huggingface.co/BAAI/bge-m3)**
(XLM-RoBERTa-large : 24 couches, hidden 1024, 16 têtes, vocab 250 002).

**Ce n'est pas notre modèle.** Licence MIT, tout le mérite revient à ses auteurs — Jianlv
Chen, Shitao Xiao, Peitian Zhang, Kun Luo, Defu Lian et Zheng Liu (BAAI). Nous n'avons
fait que changer le format pour pouvoir le charger depuis Rust.

| | |
|---|---|
| Source | `onnx/model.onnx` + `onnx/model.onnx_data` de BAAI/bge-m3 |
| Outil | `burn-onnx 0.22.0-pre.1`, `LoadStrategy::Bytes` |
| Taille | 8 974 lignes (~29 Ko une fois compressé par git) |
| Poids | **non inclus** — 2,2 Go, publiés séparément |

### Les poids

Ils ne sont pas dans ce dépôt et n'y seront jamais. Ils vivent ici :

**https://huggingface.co/Lucie666/bge-m3-burnpack**

```
model.bpk   2 266 989 828 octets
sha256      3edce43cf80ce99a19922e430d5d2ef4e47864fff2114654968c1a3726fbac9d
```

Téléchargement en HTTPS anonyme, sans compte ni token :

```
https://huggingface.co/Lucie666/bge-m3-burnpack/resolve/main/model.bpk
```

Chargement :

```rust
let bytes = /* les octets du .bpk, lus ou téléchargés */;
let model = Model::from_bytes(Bytes::from_bytes_vec(bytes), &device);
```

### Interface

```rust
pub fn forward(
    &self,
    input_ids: Tensor<2, Int>,
    attention_mask: Tensor<2, Int>,
) -> (Tensor<3>, Tensor<2>)
```

- `Tensor<3>` — `token_embeddings [B, S, 1024]`, les hidden states par token. C'est
  l'entrée de la tête sparse BGE-M3 (`sparse_linear.pt` → ReLU → scatter par token_id).
- `Tensor<2>` — `sentence_embedding [B, 1024]`, le dense déjà CLS-poolé et L2-normalisé.
  Correspond à `pooling_mode_cls_token: true` en amont, donc identique à ce que produit
  `BgeM3Embedder::extract_dense`.

La tokenisation n'est pas incluse : utiliser le `tokenizer.json` de BAAI, avec `<pad>` = 1.

### Régénérer

```rust
// build.rs
use burn_onnx::{LoadStrategy, ModelGen};

fn main() {
    ModelGen::new()
        .input("onnx/model.onnx")        // + model.onnx_data à côté
        .out_dir("model/")
        .load_strategy(LoadStrategy::Bytes)
        .run_from_script();
}
```

Deux pièges, tous deux vérifiés :

1. **`burn-onnx 0.21` ne peut pas faire le job.** Il panique sur les tenseurs en external
   data (`base_path` non propagé sur le chemin zero-copy mmap de `onnx-ir`). Et ce n'est
   pas contournable : le protobuf ONNX plafonne à 2 Go, donc ce modèle *doit* utiliser
   l'external data. Il faut `>= 0.22.0-pre.1`.
2. **Le `.bpk` n'est pas reproductible octet à octet.** Deux builds identiques donnent des
   fichiers de même taille mais d'octets différents. Les *valeurs* des tenseurs, elles, ne
   bougent pas — la sortie reste numériquement identique. Conséquence : si tu régénères,
   le checksum publié devient caduc alors que rien n'a changé fonctionnellement. Ne
   régénère pas sans raison, et republie le checksum si tu le fais.

### Parité vérifiée

Contre `candle_transformers::models::xlm_roberta::XLMRobertaModel` (l'implémentation de
référence, sur CPU), même jeu de phrases :

```
phrase        cosinus       max|Δ|       moy|Δ|
------------------------------------------------
[0]        1.00000000     3.10e-07     6.77e-08
[1]        1.00000000     3.50e-07     4.33e-08
[2]        1.00000000     2.40e-07     5.26e-08
```

Le résidu est du bruit d'accumulation f32 sur 24 couches, dû à un ordre d'opérations
différent. Backend au moment du test : Burn + wgpu/Vulkan sur AMD Radeon AI PRO R9700
(Navi 48, RDNA4, gfx1201) via RADV.

Pour rejouer la comparaison :

```bash
cargo run --release --example bge_m3_reference --features bge-m3 -- \
    <dir-des-poids-pytorch> /tmp/candle_reference.json
```

---

## `minilm_onnx.rs`

Traduction mécanique, par `burn-onnx`, du graphe ONNX de
**[sentence-transformers/all-MiniLM-L6-v2](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2)**
(BERT : 6 couches, hidden 384, vocab 30 522, positions 512). Anglais uniquement.

**Ce n'est pas notre modèle.** Licence Apache-2.0, tout le mérite revient à ses auteurs
(Nils Reimers et l'équipe sentence-transformers, sur la base du MiniLM de Microsoft).
Nous n'avons fait que changer le format pour pouvoir le charger depuis Rust.

| | |
|---|---|
| Source | `onnx/model.onnx` de sentence-transformers/all-MiniLM-L6-v2 (export PyTorch 2.5, 90 405 214 octets) |
| Outil | `burn-onnx 0.22.0-pre.1`, `LoadStrategy::Bytes` — même chaîne que BGE-M3, pour que code généré et runtime `burn 0.22.0-pre.2` s'accordent |
| Taille | 2 420 lignes (~9 Ko compressé) |
| Poids | **non inclus** — 90 Mo |

### Les poids

```
model.bpk   90 290 432 octets
sha256      6089c3066b983985c4e0933eb3b88ba7ae62573206de5ee8b78d7317de9cdcd4
```

Publiés, comme BGE-M3, avec l'attribution d'origine (Apache-2.0, sentence-transformers) :

**https://huggingface.co/Lucie666/all-minilm-l6-v2-burnpack**

Téléchargement en HTTPS anonyme, sans compte ni token :

```bash
mkdir -p ~/.cache/rag3weaver/minilm
curl -L -o ~/.cache/rag3weaver/minilm/model.bpk \
  https://huggingface.co/Lucie666/all-minilm-l6-v2-burnpack/resolve/main/model.bpk
curl -L -o ~/.cache/rag3weaver/minilm/tokenizer.json \
  https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer.json
```

Les tests les cherchent dans `~/.cache/rag3weaver/minilm/`
(`RAG3WEAVER_MINILM_BPK` / `RAG3WEAVER_MINILM_TOKENIZER` pour un autre chemin).
Ils se régénèrent aussi en deux minutes depuis l'ONNX (voir « Régénérer ») — mais le
`.bpk` obtenu n'aura pas le même checksum, voir la réserve plus bas.

Pourquoi ce modèle : c'est le **défaut navigateur** décidé le 24 août — 90 Mo contre
2,2 Go pour BGE-M3 — avec `LoadStrategy::Bytes`, donc c'est JS qui fournit les octets
(IDBFS), jamais un poids embarqué dans le WASM.

### Interface

```rust
pub fn forward(
    &self,
    input_ids: Tensor<2, Int>,
    attention_mask: Tensor<2, Int>,
    token_type_ids: Tensor<2, Int>,
) -> Tensor<3>   // last_hidden_state [B, S, 384]
```

Contrairement à BGE-M3, le graphe **ne poole pas** : seul `last_hidden_state` sort.
Le mean pooling sous masque puis la normalisation L2 sont faits dans
`src/burn_minilm_embedder.rs`, avec exactement l'arithmétique de `CandleEmbedder`.

Piège : le fichier généré contient un `forward` par sous-module (`Submodule1..6`) avant
celui de `Model` (ligne ~2404). Le premier qu'on trouve en lisant rend un tuple avec un
`Tensor<4>` — ce n'est pas le bon.

Tokenisation : `tokenizer.json` de sentence-transformers, `[PAD]` = 0, troncature à 512.

### Régénérer

```rust
// build.rs — identique à BGE-M3, seul l'input change
ModelGen::new()
    .input("~/.cache/rag3weaver/minilm/model.onnx")
    .out_dir("model/")
    .load_strategy(LoadStrategy::Bytes)
    .run_from_script();
```

Le modèle tient sous les 2 Go du protobuf ONNX, donc pas d'external data : ici
`burn-onnx 0.21` aurait suffi. On garde `0.22.0-pre.1` par cohérence avec le runtime.
Même réserve que pour BGE-M3 : le `.bpk` n'est pas reproductible octet à octet, seules
les valeurs le sont — ne pas régénérer sans raison.

### Parité vérifiée

Contre `CandleEmbedder` (`DefaultModel::MiniLM`, `candle_transformers` BERT, CPU),
quatre phrases dont une de code (`let value = foo->bar;`) :

```
phrase        cosinus       max|Δ|       moy|Δ|
------------------------------------------------
[0]        1.00000012     2.09e-07     4.37e-08
[1]        1.00000036     1.56e-07     3.87e-08
[2]        1.00000012     1.88e-07     3.79e-08
[3]        1.00000024     1.49e-07     3.80e-08
```

Bruit d'accumulation f32 sur 6 couches. Backend au moment du test : Burn + wgpu/Vulkan
sur AMD Radeon AI PRO R9700 via RADV.

Pour rejouer :

```bash
cargo run --release --example minilm_reference --features candle-embedder -- /tmp/minilm_reference.json
cargo run --release --example burn_minilm_vs_candle --no-default-features --features burn-embedder -- \
    ~/.cache/rag3weaver/minilm/model.bpk ~/.cache/rag3weaver/minilm/tokenizer.json /tmp/minilm_reference.json
```

---

## `msmarco_minilm_onnx.rs`

Traduction mécanique, par `burn-onnx`, du graphe ONNX de
**[cross-encoder/ms-marco-MiniLM-L-6-v2](https://huggingface.co/cross-encoder/ms-marco-MiniLM-L-6-v2)**
(BERT : 6 couches, hidden 384, vocab 30 522, positions 512, `type_vocab_size` 2).
Anglais uniquement. Entraîné sur MS MARCO Passage Ranking : un **cross-encoder**, qui
lit la paire `(requête, passage)` en une seule séquence et rend un logit de pertinence.
C'est le reranker produit (`SearchOptions.rerank`, doc 29 chantier 3).

**Ce n'est pas notre modèle.** Licence Apache-2.0, tout le mérite revient à ses auteurs
(Nils Reimers et l'équipe sentence-transformers / cross-encoder, sur la base du MiniLM de
Microsoft). Nous n'avons fait que changer le format pour pouvoir le charger depuis Rust.

| | |
|---|---|
| Source | `onnx/model.onnx` de cross-encoder/ms-marco-MiniLM-L-6-v2 (91 011 230 octets, sha256 `5d3e70fd0c9ff14b9b5169a51e957b7a9c74897afd0a35ce4bd318150c1d4d4a`) |
| Outil | `burn-onnx 0.22.0-pre.1`, `LoadStrategy::Bytes` — même chaîne que les deux autres |
| Taille | 2 440 lignes (~9 Ko compressé) |
| Poids | **non inclus** — 90 Mo |

### Les poids

```
model.bpk        90 883 844 octets
sha256           f2c416115ca43604b18a4e7da3c0651ea0cdb10994f1e6a60f19185304d9acd6
tokenizer.json   711 396 octets
sha256           d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66
```

Publiés, comme les deux autres, avec l'attribution d'origine (Apache-2.0,
sentence-transformers) :

**https://huggingface.co/Lucie666/ms-marco-minilm-l6-v2-burnpack** — publié le 24 août 2026 (sha256 vérifié après téléchargement) ; en
attendant, le `.bpk` se régénère en deux minutes depuis l'ONNX (voir « Régénérer »).

Téléchargement en HTTPS anonyme, sans compte ni token :

```bash
mkdir -p ~/.cache/rag3weaver/msmarco-minilm
curl -L -o ~/.cache/rag3weaver/msmarco-minilm/model.bpk \
  https://huggingface.co/Lucie666/ms-marco-minilm-l6-v2-burnpack/resolve/main/model.bpk
curl -L -o ~/.cache/rag3weaver/msmarco-minilm/tokenizer.json \
  https://huggingface.co/cross-encoder/ms-marco-MiniLM-L-6-v2/resolve/main/tokenizer.json
```

Les tests les cherchent dans `~/.cache/rag3weaver/msmarco-minilm/`
(`RAG3WEAVER_MSMARCO_BPK` / `RAG3WEAVER_MSMARCO_TOKENIZER` pour un autre chemin).

### Interface

```rust
pub fn forward(
    &self,
    input_ids: Tensor<2, Int>,
    attention_mask: Tensor<2, Int>,
    token_type_ids: Tensor<2, Int>,
) -> Tensor<2>   // logits [B, 1]
```

Contrairement à `minilm_onnx.rs`, le graphe est **complet** : encodeur → `[CLS]` →
pooler (dense 384→384 + tanh) → classifier (384→1). Le `config.json` déclare
`sbert_ce_default_activation_function: Identity`, donc la sortie est le **logit brut** :
plus haut = plus pertinent, non borné (−11 … +11 sur MS MARCO). Une sigmoïde en ferait
une probabilité ; `src/burn_reranker.rs` ne l'applique pas, seul l'ordre est contractuel.

Même piège que MiniLM : le fichier généré contient un `forward` par sous-module
(`Submodule1..6`) avant celui de `Model` (ligne ~2424).

Tokenisation : `tokenizer.json` du modèle, `[PAD]` = 0, **paires** via
`EncodeInput::Dual` (`[CLS] requête [SEP] passage [SEP]`, `token_type_ids` 0 puis 1 —
émis par le tokenizer, pas codés en dur). Troncature à 512 en `OnlySecond` : c'est le
passage qu'on coupe, la requête survit entière (sentence-transformers utilise
`LongestFirst` ; identique tant que la requête tient, et une requête de 500 tokens n'est
pas un cas de reranking).

### Régénérer

```rust
// build.rs — identique aux deux autres, seul l'input change
ModelGen::new()
    .input("~/.cache/rag3weaver/msmarco-minilm/model.onnx")
    .out_dir("model/")
    .load_strategy(LoadStrategy::Bytes)
    .run_from_script();
```

Le code généré importe `burn::nn::LinearLayout` (`LinearLayout::Col` sur le pooler et
le classifier, dont l'export ONNX transpose les poids) : ce type n'existe qu'à partir de
`burn 0.22.0-pre.2`, la version du runtime. Avec `burn 0.21` le fichier ne compile pas.
Même réserve que pour les deux autres : le `.bpk` n'est pas reproductible octet à octet,
seules les valeurs le sont — ne pas régénérer sans raison.

### Parité vérifiée

Contre candle (CPU) : `candle_transformers::models::bert::BertModel` s'arrête à
l'encodeur, donc `examples/reranker_reference.rs` charge lui-même `bert.pooler.dense` et
`classifier` depuis le même `model.safetensors` et les applique sur `[CLS]`. Cinq paires,
dont les trois de la fiche du modèle (« how many people live in berlin ») :

```
paire        burn          candle        |Δ|
------------------------------------------------
[0]       8.648815      8.648816    9.54e-7
[1]     -11.352621    -11.352626    4.77e-6
[2]      -9.009550     -9.009547    2.86e-6
[3]       4.504333      4.504328    5.25e-6
[4]     -11.164450    -11.164444    5.72e-6

max |Δ|: 5.72e-6   (seuil de l'exemple : 1e-3 ; l'ordre est aussi comparé)
```

Bruit d'accumulation f32 sur 6 couches + pooler + classifier. Backend au moment du
test : Burn + wgpu/Vulkan sur AMD Radeon AI PRO R9700 via RADV.

Pour rejouer :

```bash
cargo run --release --example reranker_reference --features candle-embedder -- /tmp/reranker_reference.json
cargo run --release --example burn_reranker_vs_candle --no-default-features --features burn-embedder -- \
    /tmp/reranker_reference.json
```
