# Doc 02 — Spike burn / Vulkan / AMD : résultats

Date : 23 août 2026
Suite du [doc 01](01-reprise-etat-et-plan.md), §2 « Changement d'environnement CUDA → AMD ».

**Conclusion courte : la direction burn + Vulkan est validée. BGE-M3 tourne sur RDNA4 et
reproduit l'implémentation candle — dense (cosinus 1.00000000) *et* sparse (mêmes
token_ids, poids à 6e-06 près).** Les poids convertis sont publiés sur
[Lucie666/bge-m3-burnpack](https://huggingface.co/Lucie666/bge-m3-burnpack).

---

## 0. Le problème de départ

La machine est passée de CUDA à AMD : deux **Radeon AI PRO R9700** (Navi 48, gfx1201,
RDNA4), ROCm 7.2.4, aucun CUDA. Or `candle-core 0.8.4` n'expose que
`accelerate · cuda · cudnn · metal · mkl` — **pas de backend ROCm**. La feature `cuda`
de rag3weaver (`Cargo.toml:55`) est donc morte sur cette machine.

Contrainte produit posée par Lucie : **pas de Python, pas de Docker, pas de sidecar.**
Ce qui fait la valeur du projet c'est la portabilité, l'embarquabilité et la
packageabilité — un utilisateur qui doit installer un environnement Python a perdu
l'intérêt du moteur.

Cible retenue : **burn + wgpu/Vulkan**, une seule implémentation qui couvre AMD natif,
NVIDIA natif, Apple, et **le navigateur via WebGPU** — ce qu'aucun sidecar ne peut donner.

---

## 1. Spike 1 — burn tourne-t-il sur RDNA4 ?

C'était le vrai risque : `cubecl-hip` documente RDNA 2 et RDNA 3, **pas RDNA 4**, et son
accélération matmul est « rdna3 par défaut, rocWMMA requis pour les autres
architectures ». Il existe des rapports de panic à l'init sur certaines cartes AMD
(6700xt) alors que PyTorch fonctionne sur la même machine.

**Le contournement : passer par Vulkan (RADV) plutôt que HIP** — agnostique de
l'architecture. Vulkan 1.4.357 avec `radeon_icd.json` est présent sur la machine.

Projet : `scratchpad/burn-amd-spike` (burn 0.21, features `wgpu,vulkan`).
Testé aux shapes BGE-M3 réelles (batch 8 × seq 512 × hidden 1024 × 16 têtes) :

```
[correction] max|A@I - A|        = 0e0        ← matmul exact au bit près
[correction] max|sum(softmax)-1| = 1.19e-7    ← exactement 1 ULP f32

  proj QKV  [B*S,H] @ [H,H]              1.60 ms
  attn QK^T [B,h,S,d] @ [B,h,d,S]        1.42 ms
  softmax   [B,h,S,S] dim=-1             2.29 ms
  BM42 chain softmax→mean(h)→CLS         2.21 ms   ← reproduit bm42_embedder.rs:158-177
  attn AV   [B,h,S,S] @ [B,h,S,d]        1.24 ms
  sparse head [B*S,H] @ [H,1] + relu     0.38 ms   ← reproduit bge_m3_embedder.rs:79
```

**Aucun panic, aucune primitive manquante.** Le scénario 6700xt ne se produit pas sur
gfx1201. Le `WARNING: radv is not a conformant Vulkan implementation` est cosmétique
(certification Khronos chez Mesa).

**Bonus : les deux cartes sont adressables séparément** via
`WgpuDevice::DiscreteGpu(0)` et `(1)`. De quoi paralléliser l'embedding par shard.

### Ce que ce spike ne montre pas

Le benchmark synchronise le GPU à **chaque** itération (`.sum().into_data()`), donc il
mesure une latence d'aller-retour hôte, **pas un débit**. Symptôme : f16 et f32 donnent
des temps identiques à 1 % près, et `max|sum(softmax)-1|` vaut exactement 1 ULP f32 dans
les deux cas — alors qu'en vrai f16 on attendrait ~1e-3. Ces chiffres sont un plancher.
Il reste aussi à confirmer que le compilateur SPIR-V est actif plutôt que WGSL (c'est là
que se jouent les matrix cores).

---

## 2. Spike 2 — BGE-M3 sans écrire le modèle

### Le constat qui a tout changé

`bge_m3_embedder.rs` n'utilise **pas** de code maison : il s'appuie sur
`candle_transformers::models::xlm_roberta::XLMRobertaModel`, une implémentation **upstream
de 545 lignes**. Le porter à la main, c'était réimplémenter ~400 lignes qu'on n'a jamais
écrites, avec des pièges silencieux (le décalage de position_ids RoBERTa :
`cumsum(mask) * mask + padding_idx`).

**Alternative trouvée : `burn-onnx`**, qui convertit un ONNX en code Burn Rust natif,
backend-agnostique. Et BAAI/bge-m3 publie `onnx/model.onnx`.

### Vérifications préalables

| Point | Résultat |
|---|---|
| Ops nécessaires supportés | ✅ tous, dont **`CumSum`** (le piège position_ids), `Erf`, `Gelu`, `LayerNormalization` |
| Opsets | ✅ 1 → 24 ; l'export BAAI est un PyTorch 2.1.2 |
| Sorties du graphe ONNX | ✅ `token_embeddings [B,S,1024]` **et** `sentence_embedding [B,1024]` |
| Pooling | ✅ `1_Pooling/config.json` → `pooling_mode_cls_token: true`, identique à `extract_dense` |

Le risque redouté — un export qui ne sortirait que le dense poolé, rendant la tête sparse
impossible — **n'existe pas** : `token_embeddings` est bien le `last_hidden_state`.

### Le bug de la 0.21, et sa résolution

`burn-onnx 0.21.0` **échoue** sur les poids externes :

```
PANIC => onnx-ir-0.21.0/src/proto_conversion.rs:264
invalid tensor '0.auto_model.embeddings.word_embeddings.weight'
  (dims [250002, 1024] => 256002048 elems) with payload 0 elems;
  original error: VariantNotFound("... uses external data but no base_path provided.")
```

Cause : `proto_conversion.rs:511` utilise `TensorDataRef::try_from(tensor)` — une
conversion **sans contexte**, donc sans `base_path`, sur le chemin zero-copy mmap.
L'infrastructure existe (`external_data.rs`, `resolve_path`), elle n'est pas câblée là.

Ce n'est pas contournable : le protobuf ONNX plafonne à 2 Go, donc un modèle de cette
taille *doit* utiliser l'external data.

**`burn-onnx 0.22.0-pre.1` corrige le problème.**

### Résultat du codegen

```
model.rs   8 974 lignes, 359 Ko    (25 structs : Submodule1 = embeddings + masque,
                                     Submodule2..25 = les 24 couches, Model qui chaîne)
model.bpk  2,2 Go                   (poids au format burnpack)
```

API générée — exactement ce qu'il faut :

```rust
pub fn forward(
    &self,
    input_ids: Tensor<2, Int>,
    attention_mask: Tensor<2, Int>,
) -> (Tensor<3>, Tensor<2>)
//     token_embeddings [B,S,1024]   sentence_embedding [B,1024]
```

Note API : **burn 0.22 supprime le générique de backend** sur `Tensor`. On écrit
`Tensor<2, Int>` et non `Tensor<B, 2, Int>`, et le device se construit par
`Device::wgpu(DeviceKind::DiscreteGpu(0))`. C'est un changement d'API notable par rapport
à la 0.21.

---

## 3. Exécution réelle sur Vulkan

Projet : `scratchpad/bge-onnx-22`. Tokenizer HF (`tokenizer.json`, padding `<pad>`=1),
3 phrases françaises, forward sur `DiscreteGpu(0)`.

```
chargement des poids            1.08 s
forward #1 (kernels à froid)    0.62 s
forward chaud (meilleur sur 5)  63.2 ms      ← batch 3 × 19 tokens
```

Le premier run affichait 10,96 s : c'était la compilation des kernels SPIR-V, pas un
problème de perf. 63 ms pour 24 couches, aller-retour hôte compris.

### Vérifications auto-portantes

```
[1] ||sentence_embedding||                = 1.000000  (×3)
[2] cos(normalize(CLS des tokens), sent)  = 0.999999 / 1.000000 / 1.000001
[3] cos(paraphrase chat/félin)            = 0.6494
    cos(chat, compilation Rust)           = 0.3750
    cos(félin, compilation Rust)          = 0.2976
```

- **[1]** le dense sort bien L2-normalisé
- **[2]** le CLS des `token_embeddings` normalisé redonne exactement le
  `sentence_embedding` → les deux sorties sont cohérentes entre elles, et la tête sparse
  sera nourrie par le même tenseur que le dense
- **[3]** écarte le scénario dangereux : des nombres plausibles mais un modèle faux

---

## 4. Parité avec l'implémentation candle — **CONFIRMÉE**

Les vérifications du §3 sont auto-portantes : elles montrent que le modèle est *sain*,
pas qu'il est *identique* à celui qui a produit les index existants. Il fallait un oracle.

### Ce qui a été ajouté au code

`BgeM3Embedder::from_local_dir()` et `from_local_dir_on(dir, device)`
(`src/bge_m3_embedder.rs`) : chargement depuis un répertoire local, **sans hf-hub**.
Volontairement **non gaté** derrière la feature `bge-m3` — le constructeur a besoin de
candle, pas de hf-hub. C'est aussi le premier pas concret vers la sortie de hf-hub :
les poids peuvent désormais être livrés à côté du binaire au lieu d'être téléchargés au
runtime.

`examples/bge_m3_reference.rs` : produit les vecteurs de référence en JSON, sur **CPU
forcé** pour la reproductibilité.

```bash
cargo run --release --example bge_m3_reference --features bge-m3 -- \
    ~/.cache/bge-m3-weights /tmp/candle_reference.json
```

### Résultat

Même jeu de 3 phrases, candle CPU vs burn-onnx Vulkan RDNA4 :

```
phrase        cosinus       max|Δ|       moy|Δ|
------------------------------------------------
[0]        1.00000000     3.10e-07     6.77e-08
[1]        1.00000000     3.50e-07     4.33e-08
[2]        1.00000000     2.40e-07     5.26e-08

similarités inter-phrases :
  (0,1)  candle=0.649415   burn=0.649415   Δ=9.32e-08
  (0,2)  candle=0.375003   burn=0.375003   Δ=1.11e-07
  (1,2)  candle=0.297606   burn=0.297607   Δ=5.93e-08
```

Cosinus **1.00000000** sur les trois. L'écart max de 3,5e-07 porte sur des composantes
de l'ordre de 0,03 — quelques ULP f32, dus à un ordre d'accumulation différent sur
24 couches. Ce n'est pas une divergence.

Corollaire : le 0,6494 de la paraphrase, qui semblait bas pour BGE-M3, est **exactement**
ce que produit l'implémentation actuelle. C'est la paire de phrases qui est lexicalement
disjointe, pas le modèle qui dérive.

**Les index existants restent valides après bascule.**

Note : le `pytorch_model.bin` (2,2 Go) a servi d'oracle. Il reste utile pour rejouer la
comparaison après toute montée de version de burn.

### Le sparse aussi — **CONFIRMÉ**

Tête sparse implémentée côté burn : `Linear(1024 -> 1)` sur les `token_embeddings`,
puis ReLU, puis scatter par token_id en gardant le max, avec exclusion des ids spéciaux
XLM-R (`<s>`=0, `<pad>`=1, `</s>`=2, `<unk>`=3) — soit exactement `extract_sparse`.

```
[0] nnz 7=7    indices identiques   max|Δ| 5.50e-07   max rel 5.57e-06
[1] nnz 9=9    indices identiques   max|Δ| 2.40e-07   max rel 6.00e-06
[2] nnz 14=14  indices identiques   max|Δ| 2.90e-07   max rel 6.73e-06
```

Mêmes token_ids sélectionnés, mêmes poids. Le scatter par max et l'exclusion des tokens
spéciaux se comportent à l'identique.

#### Un obstacle de packaging découvert au passage

`sparse_linear.pt` stocke ses poids en **f16** (`HalfStorage` : 2048 octets = 1024 × 2,
plus 2 octets de biais). Or `burn_store::PytorchStore` **n'expose aucun hook de cast
dtype** — il applique toujours `PyTorchToBurnAdapter` (qui gère bien la transposition
`Linear` [out,in] → [in,out]), mais pas la conversion de précision. Résultat :
`DTypeMismatch` au chargement dans un `Linear` f32.

Contourné dans le spike en lisant les valeurs f16 directement depuis le zip et en
construisant le `Linear` à la main. Pour la production, trois options :

1. **Embarquer les valeurs dans le crate** — c'est 1025 f32, soit **4 Ko**. À cette
   taille, un `include_bytes!` ou un tableau const supprime toute la question du
   chargement. *Recommandé.*
2. Convertir une fois `sparse_linear.pt` en safetensors ou burnpack f32 et le publier à
   côté de `model.bpk` (4 Ko de plus sur le repo HF).
3. Demander en amont un cast dtype sur `PytorchStore`.

Note : les poids d'origine étant en f16, la précision réelle de la tête sparse est celle
du f16 — la convertir en f32 ne perd rien, c'est ce que fait déjà candle
(`VarBuilder::from_pth(..., DTYPE=F32, ...)`).

### Réserves assumées

1. **Pre-release** — burn 0.22.0-pre.2 + burn-onnx 0.22.0-pre.1. Ça marche, mais c'est du
   non-stabilisé. La 0.21 stable ne peut pas faire le job (bug external data).
2. **Code généré illisible** (`add197_out1`, `mul2_out1`). Contrepartie : reproductible
   depuis l'ONNX, c'est un artefact de build plutôt que du source.
3. **`model.bpk` = 2,2 Go** — même question de distribution des poids qu'avec hf-hub.
4. **Débit non mesuré** aux tailles de batch réelles d'ingestion (le 63 ms est sur
   batch 3 × 19 tokens, non extrapolable).

### Corollaire pour BM42

La contrainte « pas de Python à l'exécution » n'interdit pas d'utiliser PyTorch comme
**outil d'export au build**. Un export ONNX unique du BERT modifié avec
`output_attentions=True`, passé dans burn-onnx, et l'artefact livré reste 100 % Rust.
Le port à la main de BM42 devient optionnel lui aussi.

### Suite proposée

1. ~~Ajouter `BgeM3Embedder::from_local_dir()`~~ ✅ fait
2. ~~Comparer candle vs burn → établir la parité~~ ✅ fait, cosinus 1.00000000
3. ~~Brancher la tête sparse + comparer aux nnz de référence~~ ✅ fait, indices identiques
4. Mesurer le débit aux tailles de batch d'ingestion réelles
5. Exporter BM42 en ONNX depuis PyTorch (build-time) plutôt que le porter à la main
6. Décider où vit le modèle généré : artefact de build régénéré depuis l'ONNX, ou code
   commité dans le repo

---

## Annexe — artefacts

| Chemin | Contenu |
|---|---|
| `scratchpad/burn-amd-spike/` | Spike 1 — primitives BM42/BGE-M3, 2 binaires (principal + `f16`) |
| `scratchpad/bge-onnx-22/` | Spike 2 — codegen ONNX + runner Vulkan |
| `~/.cache/bge-m3-weights/` | 4,3 Go : `model.onnx` + `model.onnx_data` (route ONNX), `pytorch_model.bin` (oracle candle), `config.json`, `tokenizer.json`, `sparse_linear.pt` |

Le `pytorch_model.bin` (2,2 Go) ne sert qu'à l'oracle : supprimable une fois la parité établie.
