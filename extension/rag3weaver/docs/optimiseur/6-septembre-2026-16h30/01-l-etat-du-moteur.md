# L'état du moteur d'embarquement — session « optimiseur »

**6 septembre 2026, 16 h 30 → 23 h.** La session qui tient le moteur burn
(embedders, rerankers, OCR, démon d'embedding, forks de burn/cubecl/cubek),
en face de la session « architecture » qui tient le chemin d'indexation.
Ce document dit ce qui tourne à la fin de la journée et pourquoi ; le
[02](02-knowledge-dump.md) dit comment le mesurer, le [03](03-chantiers-ouverts.md)
ce qui reste.

## Les documents du même genre, avant celui-ci

| | quoi |
|---|---|
| [`docs/issues/6-septembre-2026/03`](../../issues/6-septembre-2026/03-le-chemin-vulkan-ce-qu-il-valait-et-les-lignes-qui-manquaient.md) | le récit complet de la journée, avec tous les tableaux |
| [`docs/issues/6-septembre-2026/04`](../../issues/6-septembre-2026/04-rapport-pour-la-session-principale.md) | le rapport remis à la session architecture, cahier des charges de l'indexation |
| [`docs/issues/6-septembre-2026/02`](../../issues/6-septembre-2026/02-le-chemin-burn-wgpu-a-optimiser.md) | le point de départ du matin — dépassé, gardé pour l'écart |
| [`docs/forks-burn-cubecl-cubek.md`](../../forks-burn-cubecl-cubek.md) | les trois forks, une ligne chacun |

## 1. Ce qui tourne, en une ligne

`[rag3weaver] burn : DiscreteGpu(1) → Device<Vulkan(DiscreteGpu(1))> · précision Flex32 (défaut) · autotune oui · fusion oui`

C'est ce que chaque modèle imprime au chargement (`burn_device::resolve`).
Si cette ligne dit autre chose, rien de ce qui suit ne s'applique.

## 2. La pile

| couche | version | d'où |
|---|---|---|
| burn | 0.22.0-pre.3 | fork `L-Defraiteur/burn`, branche `rag3weaver/pre.3`, rev `ee16daac` |
| cubecl | 0.11.0-pre.3 | fork `L-Defraiteur/cubecl`, `rag3weaver/pre.3`, rev `0565518f` |
| cubek | 0.3.0-pre.3 | fork `L-Defraiteur/cubek`, `rag3weaver/pre.3`, rev `e9821ceb` |
| backend | Vulkan (wgpu + SPIR-V épinglé), radv, RDNA4 gfx1201 | `Device::vulkan(kind)` |
| précision | **Flex32** : stockage f32, matmul f16 sur les matrices coopératives, accumulation f32 | `RAG3WEAVER_BURN_FLOAT=f32` pour l'ancien |
| autotune | oui (feature `burn-autotune`), cache global `~/.cache/cubecl/default.db` | `cubecl.toml` à la racine du crate |
| fusion | oui (feature `burn-fusion`) | |

Les 51 crates burn/cubecl/cubek viennent des forks par `[patch.crates-io]`,
chaque atelier en entier (patcher un seul crate d'un dépôt tire ses voisins
en double de ceux du registre). Un poste sans réseau compile dès que
`~/.cargo/git` a les trois clones.

Ce que les forks corrigent (six PR amont à ouvrir, voir 03 B) :

| crate | fichier | la ligne |
|---|---|---|
| cubecl-wgpu | `backend/vulkan.rs`, `register_types` | Vulkan inscrit `FloatKind::Flex32` parmi ses types |
| burn-cubecl | `ops/tensor.rs`, `float_from_data` | accepte un `TensorData` en Flex32 |
| cubek-matmul | `definition/elems.rs`, `from_globals` | une sortie Flex32 accumule en f32 |
| burn-cubecl | `kernel/attention/base.rs`, `flash_attention` | en Flex32, q, k, v passent en f16 pour la voie accélérée (qui exige type global = type de tuile) ; sans ça la flash ne se lançait **jamais** et la voie naïve tournait sous son nom |
| burn-cubecl | `kernel/attention/tune.rs` | la voie naïve reste en lice tant que ses scores tiennent en 256 Mio (plus rapide sur les séquences courtes) |
| cubek-attention | `global/simple/reader/mask.rs`, `attention.rs` | le masque matérialisé se lit avec `stride(2)`, pas `seq_kv` : un masque `[b,1,1,sk]` étendu était lu de travers (cosinus 0,04) |

Le quatrième trou (`TensorData::convert_dtype(Flex32)` de burn-std, qui
ré-étiquette f32) est contourné chez nous : `Flex32Adapter` dans
`src/burn_device.rs` ré-étiquette les octets au chargement.

## 3. Les modèles

Tous chargés par `burn_device::charger_burnpack(model, octets, nom, précision)`
— store burnpack + adaptateur — et non par le `Model::from_bytes` du code
généré, qui laisserait les poids f32 et ferait calculer tout le graphe en f32.

| modèle | fichier | corps | dim | précision | conseil de lot | pooling |
|---|---|---|---|---|---|---|
| BGE-M3 | `burn_bge_m3_embedder.rs` | XLM-R large, 24 × 1024 | 1024 | Flex32 | (32, 512) | CLS + L2 dans le graphe ; tête creuse |
| granite-107m | `burn_granite_embedder.rs` | XLM-R, 6 × 384 | 384 | Flex32 | (128, 512) | CLS + L2 chez nous |
| granite-278m | idem | 12 × 768 | 768 | Flex32 | (128, 512) | idem |
| MiniLM anglais | `burn_minilm_embedder.rs` | BERT 6 × 384 | 384 | Flex32 | — | mean sous masque |
| MiniLM multilingue | `burn_multilingual_minilm_embedder.rs` | 12 × 384, fenêtre 128 | 384 | Flex32 | — | mean sous masque |
| rerankers ×3 | `burn_reranker.rs`, `burn_xlmr_reranker.rs` | MiniLM / XLM-R | logit | Flex32 | — | — |
| PP-OCRv6 tiny | `burn_ppocr.rs` | conv | — | **f32** (`PRECISION_OCR`) | — | — |

Poids sous `~/.cache/rag3weaver/<modèle>/{model.bpk,tokenizer.json}`
(`granite-107m`, `granite-278m`, `bge-m3`, `minilm`, `multilingual-minilm`,
`ppocrv6-tiny`). Graphes générés dans `generated/*_onnx.rs`, tous passés par
`generated/patch_attention.py` (voir §5).

Le démon (`src/bin/rag3weaver-embeddings.rs`) sert **`granite-278m` (défaut
depuis le 7 septembre, décision de Lucie sur le [04](04-le-banc-de-qualite.md))**,
`granite-107m` (régime rapide) ou `bge-m3` selon `RAG3WEAVER_EMBED_MODEL` ; son
`Identite` porte `modele`, `dim`, `precision`, `lot_conseille`, et le client
`DaemonEmbedder` les relaie (`dim()`, `name()`, `budget_conseille()`). Il
chauffe seize classes de forme avant d'annoncer son adresse
(`RAG3WEAVER_DEMON_CHAUFFE=0` pour s'en passer).

## 4. Ce que ça vaut

Carte TV (gfx1201, `gpu:1`), jetons ≈ mots, lots chauds, Flex32 + autotune + fusion :

| lot | BGE-M3 | granite-278m | granite-107m | MiniLM anglais | ce matin (BGE-M3, f32, sans rien) |
|---|---|---|---|---|---|
| 8 × 100 mots | 15 538 j/s | 39 017 | 91 551 | 110 562 | 5 351 |
| 32 × 100 | 19 191 | 35 444 | 188 872 | 171 102 | 5 755 |
| 64 × 100 | 21 713 | 41 726 | 204 394 | 172 452 | 3 803 |
| 32 × 300 | 15 296 | 45 037 | 108 138 | 136 110 | 3 327 |
| 8 × 900 | 8 028 | 214 694 † | 488 320 † | 576 531 † | — |
| 12 formes inédites (cache froid) | 13 470 | 21 789 | 48 609 | 28 491 | 4 196 |
| **128 × 450 (512 jetons)** | — | **121 975** | **416 153** | — | — |
| **256 × 450** | — | **121 899** | **416 565** | — | — |

† tronqués à 512 jetons. Mesuré à minuit, cache d'autotune désactivé
(`CUBECL_AUTOTUNE_CACHE=false`), sans démon, après la correction de la
flash (03 D). Avant elle, le même banc donnait granite-107m 109 573 à
128 × 450 et 129 960 à 256 (après une panique de 3 Gio), granite-278m 43 833
et 25 533 : **× 3,7 et × 2,8 sur les lots de 512 jetons**, ce que l'indexation
envoie. Sur les lots de 100 jetons la voie naïve reste la plus rapide et le
tuner la garde (elle est en lice sous 256 Mio de scores) ; les écarts avec
la mesure de 23 h (granite-278m 44 227 → 35 444 à 32 × 100, BGE-M3 24 212
→ 15 538 à 8 × 100) sont à confirmer à cache chaud, la mesure de 23 h
tournait avec le cache et l'ancien plan.

BGE-M3 est **à parité avec llama.cpp** sur la même carte (22 000–23 000 j/s sur les lots courts). Vertex
`text-embedding-005`, au quota du compte, plafonne à 8 000 j/s à un appel et
répond 429 dès huit en vol.

Qualité sur notre code (45 questions fr/en, 67 scopes réels, [04](04-le-banc-de-qualite.md)) :
granite-278m MRR 0,844, BGE-M3 0,793, granite-107m 0,779, MiniLM 0,61 et 0,57.

Parité : Flex32 contre f32, cosinus 1,00000 (BGE-M3, 16 textes, après la
correction de la flash) ; flash contre voie naïve de burn, écart absolu max
0,0004 à 0,0008 (`e2e_burn_attention`). Granite : paraphrase fr/en 0,977 contre 0,479 pour un texte étranger ;
une question en français retrouve `budget_batches` à 0,70 contre 0,53 et 0,52.

Ingestion réelle (session architecture, granite-107m, découpe façon
ragforge) : `src/dataflow` 11,8 s ; le cœur C++ de rag3db (1 642 fichiers,
20 132 chunks) **51 s**, dont 23 s de modèle — 5 000 fichiers extrapolés sous
trois minutes. Le matin même : 862 s pour trente fichiers.

## 5. Le graphe généré et ses retouches

`generated/patch_attention.py`, rejouable et idempotent, trois règles :

1. **BGE-M3 (burn-onnx pre.1)** : le bloc QKᵀ/÷√d/+masque/softmax/·V devient
   `burn::tensor::module::attention` (flash, accumulation f32), 23 couches sur
   24 — la dernière est coupée en deux sous-modules par burn-onnx et reste naïve.
2. **Tous les graphes BERT/XLM-R (burn-onnx pre.3)** : burn-onnx émet déjà
   `module::attention` mais passe le masque en biais additif (`attn_bias`), et
   burn-cubecl retombe alors sur l'attention naïve, scores matérialisés (3,2 Go
   à 256 séquences de 512). La règle le convertit en masque booléen (vrai =
   masqué, vue étendue à stride 0) : la flash prend. 66 appels.
3. `.float().cast(DType::F32)` → `.float()` (`--casts-neutres`) : l'ONNX dit
   « float », pas « f32 » ; sous Flex32 la constante du masque vient du pack
   en Flex32 et le cast f32 du masque casse la fusion. La règle vaut pour tout
   graphe chargé par le store + adaptateur (tous, depuis ce soir).

Régénérer un graphe (recette dans `generated/README.md`) puis rejouer le script.

## 6. Les décisions qui ne sont pas à rediscuter sans mesure

- **Vulkan d'abord** (Lucie) : ça tourne sur toute carte. ROCm est une option
  détectée, pas un défaut ; sur pre.3 le f16 y compile (RDNA4 corrigé) mais
  rend des NaN, Flex32 y vaut Vulkan Flex32. Il faut le paquet `rocwmma`.
- **Flex32 par défaut**, sept suites de modèles vertes ; l'OCR reste en f32
  (voir 03, chantier C).
- **Les bancs mesurent en -O3** : `[profile.dev.package."*"] opt-level = 3`.
  Notre crate reste en debug (Lucie a refusé le -O3 sur le crate, à rediscuter).
- **Une optimisation se vérifie au bit près** (écart absolu max contre la
  référence), pas au cosinus arrondi : quatre « optimisations » de la journée
  étaient des no-ops silencieux que seul l'écart nul a trahis.
