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

## `multilingual_minilm_onnx.rs`

Traduction mécanique, par `burn-onnx`, du graphe ONNX de
**[sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2](https://huggingface.co/sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2)**
(`model_type: bert` : 12 couches, hidden 384, 12 têtes, positions 512, `type_vocab_size` 2,
vocab 250 037 — soit le vocabulaire XLM-R sur un corps BERT). **Multilingue**, 50+ langues,
français compris : c'est le petit embedder dense multilingue, 470 Mo là où BGE-M3 en fait
2,2 Go, 384 d comme `minilm_onnx.rs` dont il est le jumeau multilingue. Le même modèle que
`DefaultModel::MultilingualMiniLM` côté candle, porté sur burn le 24 août 2026.

**Ce n'est pas notre modèle.** Licence Apache-2.0, tout le mérite revient à ses auteurs :
Nils Reimers et l'équipe sentence-transformers, qui l'ont obtenu par distillation
multilingue de `paraphrase-MiniLM-L12-v2` (l'anglais comme professeur, l'élève apprend à
placer la traduction au même endroit — [Reimers & Gurevych, *Making Monolingual Sentence
Embeddings Multilingual using Knowledge Distillation*, EMNLP 2020](https://arxiv.org/abs/2004.09813)),
sur la base du MiniLM de Microsoft et du vocabulaire SentencePiece de XLM-R (Conneau et
al.). Nous n'avons fait que changer le format pour pouvoir le charger depuis Rust.

| | |
|---|---|
| Source | `onnx/model.onnx` de sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2 (470 301 610 octets) |
| Outil | `burn-onnx 0.22.0-pre.1`, `LoadStrategy::Bytes` — même chaîne que les autres |
| Taille | 4 544 lignes (~16 Ko compressé) |
| Poids | **non inclus** — 470 Mo (les 250 037 × 384 de la table d'embeddings en font 384 Mo à eux seuls) |

### Les poids

```
model.bpk        470 075 648 octets
sha256           8f46e22cdc7751e7e66388267d0e9169c41b050d88c372a2e3d49bc8c8069a7a
tokenizer.json     9 081 518 octets
sha256           2c3387be76557bd40970cec13153b3bbf80407865484b209e655e5e4729076b8
```

Publiés avec l'attribution d'origine (Apache-2.0, sentence-transformers) :

**https://huggingface.co/Lucie666/paraphrase-multilingual-minilm-l12-v2-burnpack** — publié
le 24 août 2026.

Téléchargement en HTTPS anonyme, sans compte ni token :

```bash
mkdir -p ~/.cache/rag3weaver/multilingual-minilm
curl -L -o ~/.cache/rag3weaver/multilingual-minilm/model.bpk \
  https://huggingface.co/Lucie666/paraphrase-multilingual-minilm-l12-v2-burnpack/resolve/main/model.bpk
curl -L -o ~/.cache/rag3weaver/multilingual-minilm/tokenizer.json \
  https://huggingface.co/sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2/resolve/main/tokenizer.json
```

Les tests les cherchent dans `~/.cache/rag3weaver/multilingual-minilm/`
(`RAG3WEAVER_MULTILINGUAL_MINILM_BPK` / `RAG3WEAVER_MULTILINGUAL_MINILM_TOKENIZER` pour un
autre chemin). Les trois tests `phase2_vector_multilingual_*` de `tests/e2e_search.rs`
tournent dessus (ils passaient par BGE-M3 en attendant ce portage).

### Interface

```rust
pub fn forward(
    &self,
    input_ids: Tensor<2, Int>,
    attention_mask: Tensor<2, Int>,
    token_type_ids: Tensor<2, Int>,
) -> Tensor<3>   // last_hidden_state [B, S, 384]
```

Comme `minilm_onnx.rs`, le graphe **ne poole pas** : seul `last_hidden_state` sort. Le
mean pooling sous masque est fait dans `src/burn_multilingual_minilm_embedder.rs`, avec
exactement l'arithmétique de `CandleEmbedder`. En amont il n'y a **pas de module de
normalisation** (`modules.json` : Transformer + Pooling, c'est tout) ; on normalise en L2
quand même, comme le chemin candle, pour que cosinus et produit scalaire coïncident dans
l'index vectoriel.

Même piège que les autres : un `forward` par sous-module (`Submodule1..12`) avant celui
de `Model` (ligne ~4522).

Tokenisation : `tokenizer.json` du modèle, **SentencePiece Unigram avec le vocabulaire
XLM-R** (250 002 entrées) sur un corps BERT — d'où `<s>` = 0, `<pad>` = **1**, `</s>` = 2,
`<unk>` = 3 (le `pad_token_id: 0` du `config.json` est trompeur, c'est `<s>` dans ce
vocabulaire ; le masque d'attention cache les positions de padding de toute façon). Séquence
simple `<s> texte </s>`, tous type ids à 0 — le corps prend quand même `token_type_ids`
(`type_vocab_size` 2), on lui passe des zéros. Le fichier **contient ses presets** :
padding `BatchLongest` avec l'id 1, troncature à **128** en `LongestFirst` — c'est le
`max_seq_length` de sentence-transformers pour ce modèle, celui sur lequel il a été
entraîné et évalué. Le wrapper garde ces 128 (sortie identique à l'amont et à l'oracle
candle) ; `with_max_length(n)` (n ≤ 512, les positions du corps) est proposé pour qui veut
des entrées plus longues, documenté comme un écart vis-à-vis de l'amont.

### Régénérer

```rust
// build.rs — identique aux autres, seul l'input change
ModelGen::new()
    .input("~/.cache/rag3weaver/multilingual-minilm/model.onnx")
    .out_dir("model/")
    .load_strategy(LoadStrategy::Bytes)
    .run_from_script();
```

Le modèle tient sous les 2 Go du protobuf ONNX, donc pas d'external data. Le `.bpk` n'est
pas reproductible octet à octet, seules les valeurs le sont — ne pas régénérer sans raison.

### Parité vérifiée

Contre `CandleEmbedder` (`DefaultModel::MultilingualMiniLM`, `candle_transformers` BERT,
CPU), six phrases : la même phrase en français, anglais, allemand et espagnol, une phrase
technique en français, et une ligne de code (`let value = foo->bar;`) :

```
phrase        cosinus       max|Δ|       moy|Δ|
------------------------------------------------
[0]        1.00000024     1.04e-7     2.44e-8   fr : Le chat dort sur le canapé
[1]        1.00000012     9.83e-8     2.47e-8   en : The cat is sleeping on the sofa
[2]        1.00000012     9.31e-8     2.26e-8   de : Die Katze schläft auf dem Sofa
[3]        1.00000024     8.20e-8     2.42e-8   es : El gato duerme en el sofá
[4]        0.99999988     1.42e-7     3.50e-8   fr : compilation incrémentale en Rust
[5]        0.99999994     1.19e-7     3.25e-8   code

worst cosine: 0.99999988   max |Δ|: 1.42e-7   (seuil de l'exemple : 0.9999)
```

Bruit d'accumulation f32 sur 12 couches. Backend au moment du test : Burn + wgpu/Vulkan
sur AMD Radeon AI PRO R9700 via RADV. Chargement du burnpack : ~0,7 s.

Pour rejouer :

```bash
cargo run --release --example multilingual_minilm_reference --features candle-embedder -- /tmp/multilingual_minilm_reference.json
cargo run --release --example burn_multilingual_minilm_vs_candle --no-default-features --features burn-embedder -- \
    /tmp/multilingual_minilm_reference.json
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

---

## `mmarco_mminilm_onnx.rs`

Traduction mécanique, par `burn-onnx`, du graphe ONNX de
**[cross-encoder/mmarco-mMiniLMv2-L12-H384-v1](https://huggingface.co/cross-encoder/mmarco-mMiniLMv2-L12-H384-v1)**
(XLM-RoBERTa : 12 couches, hidden 384, 12 têtes, vocab 250 002 SentencePiece Unigram,
positions 514 dont 512 utilisables, `type_vocab_size` 1). **Multilingue** : entraîné sur
[unicamp-dl/mmarco](https://huggingface.co/datasets/unicamp-dl/mmarco), MS MARCO traduit
automatiquement en 14 langues, français compris. Un **cross-encoder**, comme
`msmarco_minilm_onnx.rs` : il lit la paire `(requête, passage)` en une seule séquence et
rend un logit de pertinence. C'est le reranker produit pour les corpus non anglais
(`SearchOptions.rerank`, doc 29 chantier 3).

**Ce n'est pas notre modèle.** Licence Apache-2.0, tout le mérite revient à ses auteurs :
Nils Reimers et l'équipe sentence-transformers / cross-encoder pour l'entraînement, sur la
base de [nreimers/mMiniLMv2-L12-H384-distilled-from-XLMR-Large](https://huggingface.co/nreimers/mMiniLMv2-L12-H384-distilled-from-XLMR-Large)
(MiniLMv2 de Microsoft, distillé depuis XLM-R Large), et l'équipe mMARCO (Unicamp) pour
les données. Nous n'avons fait que changer le format pour pouvoir le charger depuis Rust.

| | |
|---|---|
| Source | `onnx/model.onnx` de cross-encoder/mmarco-mMiniLMv2-L12-H384-v1 (470 883 696 octets, sha256 `3e9a03ed1e966f7c5288dd4230e3d6a9bf5e3a170a06f1f4241c5bca12c6487c`) |
| Outil | `burn-onnx 0.22.0-pre.1`, `LoadStrategy::Bytes` — même chaîne que les autres |
| Taille | 4 680 lignes (~16 Ko compressé) |
| Poids | **non inclus** — 470 Mo (les 250 002 × 384 de la table d'embeddings en font 384 Mo à eux seuls) |

### Les poids

```
model.bpk        470 617 604 octets
sha256           10e7d173623ea0bc15facd580e02cb0520d68326047b44aec1fb8490887c1c8e
tokenizer.json    17 082 660 octets
sha256           62c24cdc13d4c9952d63718d6c9fa4c287974249e16b7ade6d5a85e7bbb75626
```

Publiés avec l'attribution d'origine (Apache-2.0, sentence-transformers, mMARCO) :

**https://huggingface.co/Lucie666/mmarco-mminilmv2-l12-h384-v1-burnpack** — publié le
24 août 2026.

Téléchargement en HTTPS anonyme, sans compte ni token :

```bash
mkdir -p ~/.cache/rag3weaver/mmarco-mminilm
curl -L -o ~/.cache/rag3weaver/mmarco-mminilm/model.bpk \
  https://huggingface.co/Lucie666/mmarco-mminilmv2-l12-h384-v1-burnpack/resolve/main/model.bpk
curl -L -o ~/.cache/rag3weaver/mmarco-mminilm/tokenizer.json \
  https://huggingface.co/cross-encoder/mmarco-mMiniLMv2-L12-H384-v1/resolve/main/tokenizer.json
```

Les tests les cherchent dans `~/.cache/rag3weaver/mmarco-mminilm/`
(`RAG3WEAVER_MMARCO_BPK` / `RAG3WEAVER_MMARCO_TOKENIZER` pour un autre chemin).

### Interface

```rust
pub fn forward(
    &self,
    input_ids: Tensor<2, Int>,
    attention_mask: Tensor<2, Int>,
) -> Tensor<2>   // logits [B, 1]
```

**Pas de `token_type_ids`** : XLM-R n'a qu'un type de segment (`type_vocab_size` 1),
l'export ONNX ne prend pas l'entrée. Le graphe est complet : encodeur → `<s>` →
`classifier.dense` (384→384) + tanh → `classifier.out_proj` (384→1). Tête RoBERTa, pas
de pooler BERT. `sbert_ce_default_activation_function: Identity` dans le `config.json`,
donc la sortie est le **logit brut** : plus haut = plus pertinent, non borné.
`src/burn_xlmr_reranker.rs` n'applique pas de sigmoïde, seul l'ordre est contractuel.

Même piège que les autres : un `forward` par sous-module (`Submodule1..13`) avant celui
de `Model` (ligne ~4658).

Tokenisation : `tokenizer.json` du modèle, SentencePiece Unigram, `<s>` = 0, `<pad>` =
**1**, `</s>` = 2. Le fichier ne contient **ni troncature ni padding** : le wrapper les
pose (padding `BatchLongest` avec l'id 1, troncature à 512 en `OnlySecond`). Le template
de paire est `<s> requête </s></s> passage </s>`, tous type ids à 0 — émis par le
post-processeur du tokenizer.

### Régénérer

```rust
// build.rs — identique aux autres, seul l'input change
ModelGen::new()
    .input("~/.cache/rag3weaver/mmarco-mminilm/model.onnx")
    .out_dir("model/")
    .load_strategy(LoadStrategy::Bytes)
    .run_from_script();
```

Le code généré importe `burn::nn::LinearLayout` (`LinearLayout::Col` sur `dense` et
`out_proj` de la tête, dont l'export ONNX transpose les poids) : ce type n'existe qu'à
partir de `burn 0.22.0-pre.2`, la version du runtime. Avec `burn 0.21` le fichier ne
compile pas. Le `.bpk` n'est pas reproductible octet à octet, seules les valeurs le sont
— ne pas régénérer sans raison.

### Parité vérifiée

Contre candle (CPU). `candle_transformers::models::xlm_roberta` fournit bien un
`XLMRobertaForSequenceClassification`, mais sa tête applique `GeluPytorchTanh` là où
l'implémentation de référence (`RobertaClassificationHead`) applique `torch.tanh` — les
logits en seraient faux de quelques dixièmes. `examples/xlmr_reranker_reference.rs`
charge donc `XLMRobertaModel` (préfixe `roberta`) et reconstruit la tête à la main depuis
le même `model.safetensors` : `classifier.dense.{weight,bias}` + tanh +
`classifier.out_proj.{weight,bias}` sur l'état caché de `<s>`. Sept paires : le triplet
Berlin de la fiche ms-marco en anglais, le même en français, et une paire croisée
(requête française, réponse anglaise) :

```
paire        burn          candle        |Δ|
------------------------------------------------
[0]      10.498397     10.498405    7.63e-6   en : population
[1]      -9.185736     -9.185722    1.34e-5   en : New York
[2]      -7.658092     -7.658096    4.29e-6   en : mur
[3]       9.889890      9.889894    3.81e-6   fr : population
[4]      -8.640562     -8.640553    8.58e-6   fr : New York
[5]      -6.565522     -6.565524    2.38e-6   fr : mur
[6]      10.194462     10.194460    1.91e-6   fr → en : population

max |Δ|: 1.34e-5   (seuil de l'exemple : 1e-3 ; l'ordre est aussi comparé)
```

Bruit d'accumulation f32 sur 12 couches + tête. Backend au moment du test : Burn +
wgpu/Vulkan sur AMD Radeon AI PRO R9700 via RADV. La réponse en français (9,89) et la
réponse anglaise à la requête française (10,19) sortent au niveau de la paire tout-anglais
(10,50) : le modèle lit bien à travers les langues.

Pour rejouer :

```bash
cargo run --release --example xlmr_reranker_reference --features candle-embedder -- /tmp/xlmr_mmarco.json mmarco
cargo run --release --example burn_xlmr_reranker_vs_candle --no-default-features --features burn-embedder -- \
    /tmp/xlmr_mmarco.json mmarco
```

---

## `bge_reranker_v2_m3_onnx.rs`

Traduction mécanique, par `burn-onnx`, du graphe ONNX de
**[BAAI/bge-reranker-v2-m3](https://huggingface.co/BAAI/bge-reranker-v2-m3)**
(XLM-RoBERTa : 24 couches, hidden 1024, 16 têtes, vocab 250 002, positions 8 194,
`type_vocab_size` 1). **Multilingue**, même famille que `bge_m3_onnx.rs` (il est
fine-tuné depuis BGE-M3). Un **cross-encoder** : la paire `(requête, passage)` en une
séquence, un logit de pertinence. C'est le reranker lourd, à réserver aux pools courts
ou aux GPU qui ont la place.

**Ce n'est pas notre modèle.** Licence Apache-2.0 (fiche du modèle), tout le mérite
revient à BAAI (Chen, Xiao, Zhang, Luo, Lian, Liu — *BGE M3-Embedding*, 2024). L'export
ONNX fp32 vient de [onnx-community/bge-reranker-v2-m3-ONNX](https://huggingface.co/onnx-community/bge-reranker-v2-m3-ONNX)
(Transformers.js). Nous n'avons fait que changer le format pour pouvoir le charger
depuis Rust.

| | |
|---|---|
| Source | `onnx/model.onnx` (656 891 octets, sha256 `faae32b124a9d54afb7e89b5e9896e03c18a9552d56d1d6b273a709a83012486`) + `onnx/model.onnx_data` (données externes, 2 271 088 656 octets, sha256 `f009aa6c6cf21986fd7e0021fa66b20ccce27abc6900a57c7109c8496811bcbe`) de onnx-community/bge-reranker-v2-m3-ONNX |
| Outil | `burn-onnx 0.22.0-pre.1`, `LoadStrategy::Bytes` — même chaîne que les autres |
| Taille | 7 974 lignes (~26 Ko compressé) |
| Poids | **non inclus** — 2,2 Go |

### Les poids

```
model.bpk        2 271 128 324 octets
sha256           3ed858274ab4661332058318c8b961f0ac822af4aed899187557745107fb32e3
tokenizer.json      17 098 273 octets
sha256           69564b696052886ed0ac63fa393e928384e0f8caada38c1f4864a9bfbf379c15
```

Le `tokenizer.json` est **celui de BAAI/bge-reranker-v2-m3**, pas celui de BGE-M3 ni
celui d'onnx-community : même vocabulaire et mêmes ids, mais le normaliseur diffère
(`Strip` à droite en plus du `Precompiled`). Ne pas réutiliser le fichier de `bge-m3/`.

Publiés avec l'attribution d'origine (Apache-2.0, BAAI) :

**https://huggingface.co/Lucie666/bge-reranker-v2-m3-burnpack** — publié le 24 août 2026.

```bash
mkdir -p ~/.cache/rag3weaver/bge-reranker-v2-m3
curl -L -o ~/.cache/rag3weaver/bge-reranker-v2-m3/model.bpk \
  https://huggingface.co/Lucie666/bge-reranker-v2-m3-burnpack/resolve/main/model.bpk
curl -L -o ~/.cache/rag3weaver/bge-reranker-v2-m3/tokenizer.json \
  https://huggingface.co/BAAI/bge-reranker-v2-m3/resolve/main/tokenizer.json
```

Les tests les cherchent dans `~/.cache/rag3weaver/bge-reranker-v2-m3/`
(`RAG3WEAVER_BGE_RERANKER_BPK` / `RAG3WEAVER_BGE_RERANKER_TOKENIZER` pour un autre chemin).

### Interface

Identique à `mmarco_mminilm_onnx.rs` : `forward(input_ids, attention_mask) -> Tensor<2>`
(logits `[B, 1]`), **pas de `token_type_ids`**, tête `classifier.dense` (1024→1024) +
tanh → `classifier.out_proj` (1024→1), logit brut. `Submodule1..25` avant le `forward`
de `Model` (ligne ~7621). Mêmes ids spéciaux (`<s>` 0, `<pad>` 1, `</s>` 2), même
template de paire, même absence de presets de troncature/padding dans le tokenizer.

Le modèle accepte 8 192 positions ; `src/burn_xlmr_reranker.rs` tronque tout de même à
**512** par paire, comme les autres rerankers : l'attention est quadratique en la
longueur, et un passage à reranker est un chunk, pas un document. Un appelant qui veut
plus long paie le carré.

### Régénérer

Même `build.rs`, avec `model.onnx` et `model.onnx_data` côte à côte dans le dossier
d'entrée (données externes : burn-onnx les résout par le chemin relatif du fichier
ONNX). `LinearLayout::Col` sur la tête, donc `burn ≥ 0.22.0-pre.2`. Non reproductible
octet à octet — ne pas régénérer sans raison.

### Parité vérifiée

Même oracle que mmarco (`examples/xlmr_reranker_reference.rs … bge`, tête reconstruite à
la main, voir plus haut), depuis `model.safetensors` de BAAI/bge-reranker-v2-m3 (2,2 Go
via hf-hub). Sept paires, les mêmes :

```
paire        burn          candle        |Δ|
------------------------------------------------
[0]       6.798565      6.798559    5.72e-6   en : population
[1]     -11.028526    -11.028533    6.68e-6   en : New York
[2]      -9.748907     -9.748901    5.72e-6   en : mur
[3]       6.165921      6.165920    4.77e-7   fr : population
[4]     -11.032015    -11.032012    2.86e-6   fr : New York
[5]     -10.509789    -10.509794    5.72e-6   fr : mur
[6]       5.617373      5.617373    9.54e-7   fr → en : population

max |Δ|: 6.68e-6   (seuil de l'exemple : 1e-3 ; l'ordre est aussi comparé)
```

Bruit d'accumulation f32 sur 24 couches + tête. Backend au moment du test : Burn +
wgpu/Vulkan sur AMD Radeon AI PRO R9700 via RADV ; les 2,2 Go se chargent en 2,9 s
depuis le cache disque.

Pour rejouer :

```bash
cargo run --release --example xlmr_reranker_reference --features candle-embedder -- /tmp/xlmr_bge.json bge
cargo run --release --example burn_xlmr_reranker_vs_candle --no-default-features --features burn-embedder -- \
    /tmp/xlmr_bge.json bge
```

---

## `ppocrv6_tiny_det_onnx.rs` et `ppocrv6_tiny_rec_onnx.rs`

Traduction mécanique, par `burn-onnx`, des deux graphes ONNX de **PP-OCRv6 tiny**
(PaddleOCR 3.7, juin 2026) : le détecteur **PP-OCRv6_tiny_det** (DBNet — backbone
PPLCNetV4, cou RepLKFPN, 428 k paramètres) et le reconnaisseur **PP-OCRv6_tiny_rec**
(PPLCNetV4, projection directe, tête CTC sur 6 904 caractères + blank + espace,
1,1 M paramètres, 49 langues dont le latin accentué et le chinois). C'est l'OCR
embarqué de rag3weaver (`src/burn_ppocr.rs`, feature `burn-ocr`) : 6 Mo de poids en
tout, pas de tokenizer.

**Ce n'est pas notre modèle.** Licence Apache-2.0, tout le mérite revient à l'équipe
PaddleOCR (Baidu — *PP-OCRv6: From 1.5M to 34.5M Parameters, Surpassing Billion-Scale
VLMs on OCR Tasks*, 2026). Les ONNX sont **ceux publiés par PaddlePaddle** sur HF, pas
un export tiers.

| | det | rec |
|---|---|---|
| Source | [PaddlePaddle/PP-OCRv6_tiny_det_onnx](https://huggingface.co/PaddlePaddle/PP-OCRv6_tiny_det_onnx) `inference.onnx` (1 780 590 octets, sha256 `193bab7a04fca699a6c82e6abb5b81bdb28177f0abd4062552b04908dafb19f8`) | [PaddlePaddle/PP-OCRv6_tiny_rec_onnx](https://huggingface.co/PaddlePaddle/PP-OCRv6_tiny_rec_onnx) `inference.onnx` (4 462 639 octets, sha256 `9ef676d6ed3c88256a2d92c640c44f25b0c40947e111b14b8be8f594091563e6`) |
| Opset / IR | 14 / 10 | 11 / 6 |
| Entrée | `x [B, 3, H, W]` f32, H et W dynamiques | `x [B, 3, 48, W]` f32, W dynamique |
| Sortie | `[B, 1, H, W]` (sigmoïde, carte DB) | `[B, W/8, 6906]` (softmax, CTC) |
| Outil | `burn-onnx 0.22.0-pre.1`, `LoadStrategy::Bytes` | idem |
| Taille | 1 393 lignes | 1 548 lignes |
| Poids | **non inclus** — 1,7 Mo | **non inclus** — 4,4 Mo |

### Le patch `auto_pad`

Le détecteur contient deux `Conv` et un `MaxPool` (noyau 2×2, stride 1) déclarés avec
`auto_pad = SAME_UPPER`. `burn-onnx 0.22.0-pre.1` refuse cette forme dès que l'entrée
est dynamique :

```
auto_pad SAME_UPPER/SAME_LOWER requires static input shape, but input has dynamic
dimensions. Use explicit pads instead
```

On a donc réécrit, avant génération, ces trois nœuds en `pads = [0, 0, 1, 1]`
(padding total `k − 1 = 1`, placé en bas/droite comme `SAME_UPPER` l'exige) — même
arithmétique, aucun poids touché, et **H, W restent dynamiques** (l'alternative,
fixer 1×3×640×640, marche aussi mais forçait un resize à taille fixe). Le
reconnaisseur n'a rien eu besoin. Le script (`onnx` seul, pas d'onnxruntime) :

```python
for n in m.graph.node:
    if n.op_type in ("MaxPool", "AveragePool", "Conv"):
        ap = [a for a in n.attribute if a.name == "auto_pad"]
        if ap and ap[0].s in (b"SAME_UPPER", b"SAME_LOWER"):
            ks = [a for a in n.attribute if a.name == "kernel_shape"][0].ints
            tot = [k - 1 for k in ks]      # strides == 1 vérifié
            pads = [t // 2 for t in tot] + [t - t // 2 for t in tot]   # SAME_UPPER
            n.attribute.remove(ap[0]); n.attribute.append(helper.make_attribute("pads", pads))
```

Les ONNX patchés sont publiés avec les `.bpk` (`det_pads.onnx`, sha256
`f74ec758df06b1f77cde82a44bc840cbb39f8b7cf1573f373ea052a1e8d93ae6` ; `rec_pads.onnx` est
l'original octet pour octet, rien n'y a été touché).

### Les poids

```
det.bpk        1 737 476 octets
sha256         73a139fa82b9fc8f7c03b66ab3c3dc9e959e8c1f4d95b2da09b4e50529e76b04
rec.bpk        4 443 368 octets
sha256         53bfcb22a068cc6991f2b8b3ba0782a1aac3c54c16895ae7138eb4e755169436
dict.txt          27 156 octets (6 904 lignes, UTF-8, une entrée par ligne)
sha256         c5cbe34ef40c29c4df07ed012bf96569cb69a2d2a01a07027e9f13cb832bd9cd
```

`dict.txt` est la liste `PostProcess.character_dict` du `inference.yml` publié avec
PP-OCRv6_tiny_rec, dans l'ordre : l'index CTC `i ∈ 1..=6904` est la ligne `i`, `0` est
le blank, `6905` l'espace (`use_space_char`). Les trois fichiers vont dans
`~/.cache/rag3weaver/ppocrv6-tiny/` (`RAG3WEAVER_PPOCR_DIR` pour un autre dossier) ;
`BurnPpOcr::from_cache_dir` les lit. Publication HF : à venir (dépôt Lucie666, avec
l'attribution Apache-2.0 PaddleOCR, les ONNX patchés et cette recette).

### Interface

```rust
// det — Submodule1..2 avant le forward de Model (ligne ~1386)
pub fn forward(&self, x: Tensor<4>) -> Tensor<4>   // [B,3,H,W] -> [B,1,H,W]
// rec — Submodule1..3 avant le forward de Model (ligne ~1540)
pub fn forward(&self, x: Tensor<4>) -> Tensor<3>   // [B,3,48,W] -> [B,W/8,6906]
```

H, W, B sont dynamiques à l'exécution (vérifié sur 320×320, 224×416, 2464×736 pour
det ; W = 160/320/640 et B = 1/2 pour rec). Aucun `Shape`/`Reshape` dans le tiny rec :
le graphe le plus simple de la famille (le small rec en a, et passe aussi).

### Pré et post-traitement (ce que `src/burn_ppocr.rs` reproduit)

D'après les `inference.yml` des modèles, `ppocr/data/imaug/operators.py`,
`tools/infer/predict_rec.py`, `ppocr/postprocess/{db,rec}_postprocess.py` et les
défauts PaddleX (`text_detection/predictor.py`) :

* **det** — `DetResizeForTest` : `limit_type min`, `limit_side_len 736` (le plus petit
  côté est ramené à ≥ 736 ; v5 mobile utilisait `max 960`), plafond `max_side_limit
  4000`, puis H et W arrondis au **multiple de 32** (jamais sous 32) ; resize
  bilinéaire ; `NormalizeImage` mean `[0.485, 0.456, 0.406]` std `[0.229, 0.224, 0.225]`
  sur `x/255`, **appliqué aux canaux BGR dans cet ordre** (PaddleOCR décode en BGR et
  n'échange pas — on swappe donc notre RGB) ; CHW. Post-DB : `thresh 0.2`, contours →
  boîte minimale, rejet côté < 3, score = moyenne de la carte dans la boîte
  (`box_score_fast`) ≥ `box_thresh 0.4`, unclip `d = aire × 1.4 / périmètre`, rejet
  côté < 5, `max_candidates 3000`, retour aux pixels d'origine (arrondi, borné).
  **Écart assumé** : boîte englobante axée au lieu de `minAreaRect` + `pyclipper`
  (texte incliné : détecté, recadré droit, mal lu — dette).
* **rec** — crop du quadrilatère (perspective chez PaddleOCR, simple crop ici puisque
  la boîte est axée), rotation de 90° si `h/w ≥ 1.5` ; `rec_image_shape 3,48,320` :
  hauteur 48, largeur `ceil(48·w/h)` plafonnée à `48 × max(320/48, max w/h du lot)`,
  bilinéaire, `(x/255 − 0.5)/0.5` en BGR, padding zéro à droite ; lots de
  `rec_batch_num 6` triés par ratio. CTC glouton : argmax, répétitions consécutives
  fusionnées puis blank (0) retiré, confiance = moyenne des probabilités gardées ; une
  ligne sans caractère est écartée.

### Régénérer

```rust
// build.rs — un ModelGen par graphe, sur les ONNX patchés
ModelGen::new().input("det_pads.onnx").out_dir("model/").load_strategy(LoadStrategy::Bytes).run_from_script();
ModelGen::new().input("rec_pads.onnx").out_dir("model/").load_strategy(LoadStrategy::Bytes).run_from_script();
```

`burn-onnx = "0.22.0-pre.1"` en build-dep, `burn = "0.22.0-pre.2"` (features `std`,
`ndarray` suffit pour générer). L'en-tête du fichier généré porte le chemin machine de
l'ONNX : on le remplace par la provenance HF. Même réserve que les autres : le `.bpk`
n'est pas reproductible octet à octet.

### Parité vérifiée

Oracle : onnxruntime 1.29 (CPU, Python jetable — jamais une dépendance produit) sur les
ONNX patchés, avec **nos** tenseurs d'entrée (pour ne comparer que les réseaux) ; en
face `examples/burn_ppocr_vs_onnxruntime.rs`. Fixture `tests/fixtures/ocr/hello.png`
(400×120 → det 2464×736 ; deux crops → rec `[2, 3, 48, 320]`) :

```
                      max|Δ|     moyenne|Δ|   n > 1e-3
det carte 2464×736   1.81e-3     1.77e-6      87 / 1 813 504   (seuil 5e-3)
rec probas [2,40,6906]  1.44e-5  9.2e-11       0                (seuil 1e-3)
```

Les 87 pixels du det sont sur les bords des glyphes (sigmoïde en zone raide, image
agrandie ×6) ; **burn/ndarray donne le même écart face à ORT (1.89e-3) et wgpu ≈
ndarray à 9.7e-5** : bruit d'accumulation f32 propre à l'ordre des opérations, pas un
défaut wgpu. Boîtes identiques, texte identique des deux côtés :
`"Hello rag3weaver"` (0.987) et `"OCR 2026"` (0.984). Avec le pré-traitement refait
par PIL côté oracle (resize ×6 : jusqu'à un niveau de gris d'écart), la carte bouge de
0.1 sur les bords et les probas rec de 1.9e-3 — même texte. Backend au moment du test :
Burn + wgpu/Vulkan sur AMD Radeon AI PRO R9700 via RADV.
## `qwen2_5_0_5b_onnx.rs` — **retiré le 28 août 2026**

Ce fichier n'existe plus. Avec lui sont partis `src/burn_llm.rs`, la feature
`burn-llm`, la dépendance `hf-chat-template`, le rôle `BurnRole::Llm` et les
suites `e2e_burn_llm`, `e2e_burn_agent`, `e2e_burn_code_agent` — 16 782 lignes.

**Pourquoi**, pour que personne ne refasse le chemin :

Notre moteur fait l'**embedding**, le **rerank**, l'**OCR** — bientôt le TTS et
le STT. Ce sont des passages courts, à modèle fixe, où un graphe burn local a
un sens : pas de serveur à tenir, pas de protocole, et le résultat entre dans
le catalogue sans quitter le processus.

L'inférence d'un LLM n'a rien de commun avec ça. Elle veut un cache KV, du
batching, de la quantification, un ordonnanceur — des années de travail que
`llama.cpp` a déjà faites, et mieux. Nous n'avions pas de raison de refaire ce
chemin, et une bonne raison de ne pas le refaire : **le modèle qu'on peut
porter soi-même n'est pas celui qui produit de beaux artefacts.** Un 0,5 B
répondait déjà « The `signals` input port is used to pass signals to the `on`
method », ce qui ne veut rien dire.

Le LLM vient donc de `llama.cpp` (API compatible OpenAI, feature `openai-llm`)
ou d'un fournisseur distant. C'est ce serveur-là qui choisit sa carte, pas
nous — d'où la disparition de `RAG3WEAVER_BURN_DEVICE_LLM`.

**Le défaut qui a précipité la décision**, gardé ici parce qu'il vaut pour tout
export ONNX de décodeur qu'on tenterait plus tard : la traduction `burn-onnx`
paniquait dans l'expansion GQA au-delà d'environ 2 000 jetons —

```
attendu :  [1, 2,    7, seq, 64]  →  [1, 14, seq, 64]
obtenu  :  [1, 2,  seq, seq, 64]  →  panique Reshape
```

Le facteur de répétition (7 = 14 têtes Q / 2 têtes KV) prenait la longueur de
séquence à sa place. Le vrai Qwen2.5-0.5B fait 32 768 jetons ; c'était notre
traduction qui était fausse, pas le modèle. Ce genre de bug se corrige à la
source (l'export, ou `burn-onnx`), jamais dans le fichier généré.

Pour le ressusciter : `git show <commit>^:extension/rag3weaver/generated/qwen2_5_0_5b_onnx.rs`.
La recette d'export, les empreintes des poids et l'inventaire des exports ONNX
essayés (Qwen3-0.6B refusé pour `GroupQueryAttention`, Qwen2.5-1.5B vérifié)
sont dans l'historique de ce fichier.
