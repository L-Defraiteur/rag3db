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
