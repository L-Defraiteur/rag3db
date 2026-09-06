# Issue 03 — le chemin Vulkan : ce qu'il valait vraiment, et les lignes qui manquaient

**6 septembre 2026, 17h–19h.** Suite de l'[issue 02](02-le-chemin-burn-wgpu-a-optimiser.md).
Lucie : *« allons regarder ça, nous faut éventuellement un fork de burn ou
autre, tout ce qui est envisageable »*, puis : *« dans un monde parfait on
utilise pas ROCm mais Vulkan, comme ça marche avec tous les GPU des users ;
dans un monde moins parfait on détecte que faut ROCm »*. Ce document est ce
qu'on a trouvé en regardant, dans l'ordre, et ce que ça change à l'ordre du
sous-chantier. Beaucoup de choses de l'issue 02 étaient fausses, et pour une
raison qui vaut d'être lue en premier.

## 0. On mesurait burn compilé en -O0

`run_e2e.sh` lance `cargo test` sans `--release`, et `Cargo.toml` n'avait pas
d'`opt-level` pour les dépendances. **Tous les bancs, ceux du 28 août comme
ceux de ce matin, ont tourné avec burn, cubecl et tokenizers en -O0.** Le GPU
ne le voyait pas (le débit chaud des gros lots est le sien), mais la
compilation SPIR-V, l'autotune, les lancements de noyaux, la tokenisation, si.

Corrigé : `[profile.dev.package."*"] opt-level = 3`. Notre crate reste en
debug. Effet mesuré, f32 Vulkan sans rien d'autre : le débit chaud ne bouge
pas, chaque « premier » passage d'une forme coûte **deux fois moins** (8×900 :
9,7 s → 4,8 s ; les 12 formes inédites : 8,1 s → 6,7 s).

## 1. Le « facteur 14 » du 28 août, expliqué

Le commentaire de `burn_device.rs` disait que compiler `burn-rocm` dégradait le
chemin wgpu d'un facteur 14 même SPIR-V épinglé. C'est vrai, et ce n'est pas
ROCm : `burn-rocm` 0.22.0-pre.2 déclare `burn-cubecl` avec `features =
["default"]`, ce qui allume **autotune** (et la feature `fusion`) pour les deux
piles, alors que `burn-wgpu` les laisse éteintes. Notre binaire Vulkan n'avait
donc **ni autotune ni fusion** tant que ROCm n'était pas compilé.

| Vulkan f32, -O0 | sans `burn-rocm` | avec `burn-rocm` (donc autotune) |
|---|---|---|
| chauffe 8×100 | 3,7 s | **67 s** |
| 8×100 chaud | 5 051 j/s | 6 975 |
| 8×900 chaud | 2 009 | 2 182 |
| 12 formes inédites | 3 443 j/s | 594 |

L'autotune essaie des dizaines de candidats par classe de forme (642 dans une
passe complète, 20 s de compilation et de bancs à -O3), ce qui en -O0 devenait
une minute de chauffe. Le débit chaud, lui, gagnait. Le commentaire du 28 août
est réécrit ; le chiffre « ROCm 2,6× plus lent » mesurait un `drain` de 12
documents dominé par cette chauffe.

Depuis burn main, `burn-rocm` déclare `burn-cubecl` sans `default` : le
problème disparaît en montant de version.

## 2. RDNA4 et ROCm : un trou de cubecl pre.2, bouché dans pre.3

`LLVM ERROR: Cannot select: intrinsic %llvm.amdgcn.wmma.f32.16x16x16.f16` :
cubecl-cpp 0.11.0-pre.2 émet pour gfx12 le builtin WMMA de **RDNA3**
(`__builtin_amdgcn_wmma_f32_16x16x16_f16_w32`, fragments de 16 éléments).
RDNA4 veut `_gfx12`, sans `opsel`, fragments de 8. Le PR « Gfx12 support » de
janvier avait déclaré gfx12 « capable de WMMA » sans adapter l'émission.
Reproduit avec le clang de ROCm 7.2.4 : la variante `_gfx12` compile.

Corrigé dans cubecl 0.11.0-pre.3 (25 août, refactor « Pliron » du 11 août :
`AmdWmma::Rdna4`), donc burn 0.22.0-pre.3. La branche principale a en plus un
backend AMDGPU par LLVM (`cubecl-llvm`, 31 août) qui vise `gfx1201` en dur.
**Pas de fork pour ça : monter de version.** (Expérience pre.3 : voir §8.)

## 3. Le cache d'autotune marche ; ses clés sont arrondies

Contrairement à l'issue 02 : le cache persistant est actif par défaut
(`cubecl-environment`, SQLite, `target/environment/default.db` quand on lance
depuis le workspace, `~/.cache/cubecl/default.db` sinon — deux bases selon le
cwd, d'où l'apparente stagnation). Les clés de matmul sont **ancrées à la
puissance de 2** (`m`, `n`, `k`), celles de reduce à 25 classes au plus : une
forme de lot inédite ne coûte un autotune que si elle change de classe. Vérifié
: d'un processus au suivant, le « premier » 8×100 tombe à 113 ms, égal au
chaud.

Deux réserves. Le checksum d'une entrée ne voit que les **noms** des candidats :
patcher un noyau (cubek-matmul, §6) ne l'invalide pas, il faut vider la base.
Et le coût résiduel d'une forme neuve sans autotune (compilation de noyaux,
allocation) reste de l'ordre de 1 s par forme à -O3 — la stabilisation des
formes (issue 02, A) garde son sens.

Réglage recommandé, `cubecl.toml` à la racine du crate : `[environment] path =
"global"`, pour une seule base quel que soit le cwd. `CUBECL_AUTOTUNE_CACHE`
est un booléen (`false` désactive la persistance), pas un chemin.

## 4. La flash attention est là, mais pas sans autotune

`burn::tensor::module::attention` existe en pre.2 (cubek-attention, tuiles
f16, accumulateur f32 strict). `burn::nn::attention` ne l'utilise pas, et le
graphe généré par burn-onnx non plus. Deux faits :

- **Sans la feature `autotune`, la stratégie par défaut est `Fallback`** — la
  voie naïve, matmul + softmax. Le patch du graphe rendait des vecteurs
  identiques au bit près et le même débit.
- Avec autotune, les trois candidats `blackbox_accelerated_{2,4,8}_planes`
  gagnent partout ; le repli est écarté d'office dès que `seq_q > head_dim`.

`generated/patch_attention.py` remplace les blocs QKᵀ/÷√d/+masque/softmax/·V
par l'appel fusionné, masque booléen étendu à stride 0. **23 couches sur 24**
de BGE-M3 : la dernière est coupée en deux sous-modules par burn-onnx (les
scores arrivent en argument), elle reste naïve. Les autres graphes (MiniLM,
rerankers) n'ont pas ce motif exact ; leur attention est un chantier à part. Gain seul, f32 : +8 à +11 %. L'attention
n'était pas le goulot.

## 5. Le vrai goulot : des matmuls f32 sans tensor cores

Profil d'une passe (R2, GPU seul, 46 s de noyaux) : matmul 53 %, réductions
31 % (LayerNorm déroulée, softmax), élémentaire 12 %. Et sur Vulkan, **toutes
les clés de matmul f32 se règlent sur des noyaux `unit`**, sans matrices
coopératives : radv expose f16×f16→f32 en 16×16×16 (vu dans le journal cubecl
: « Supported CMMA sizes »), mais pas f32, et tf32 n'existe pas ici.

D'où **Flex32** : un dtype de burn/cubecl qui veut dire « stockage f32, matmul
en f16 sur les tensor cores, accumulation f32 ». La précision mixte de l'issue
02 (D), sans un cast dans le graphe, et sans les débordements du f16 pur
(cosinus 0,708). Un seul `device.configure(FloatDType::Flex32)`. Sauf que.

## 6. Flex32 était inutilisable de bout en bout : quatre trous, une dizaine de lignes

Chaque porte fermée rendait des vecteurs **identiques au bit près** à f32, sans
un message. Dans l'ordre où on les a trouvées :

| | où | quoi | correction |
|---|---|---|---|
| 1 | cubecl-wgpu `backend/vulkan.rs` `register_types` | Vulkan n'inscrit pas `Flex32` parmi ses types (WGSL le fait, cubecl-spirv l'émet) → `configure` refusé, `UnsupportedDType` | 1 ligne |
| 2 | burn-std `convert_dtype(Flex32)` | convertit vers `f32` et ré-étiquette **f32** (`self.dtype = Target::dtype()`) → `FloatCastAdapter::to(Flex32)` ne fait rien | contourné : `Flex32Adapter` dans `burn_device.rs`, ré-étiquette les octets |
| 3 | burn-cubecl `float_from_data` | refuse un `TensorData` en Flex32 (`unimplemented!`) | 1 ligne |
| 4 | cubek-matmul `MatmulElems::from_globals` | sortie Flex32 → accumulateur Flex32 ; aucune configuration CMMA n'a Flex32 en sortie → `Unavailable` → candidats accélérés démotés | 1 ligne |

Et un faux trou : burn-cubecl `kernel/cast` fait de `cast(F32)` sur du Flex32
(et l'inverse) un no-op qui garde l'étiquette d'entrée. On l'a d'abord
« corrigé » en ré-étiquetant ; c'était une erreur. Le graphe généré contient
des `.cast(DType::F32)` explicites (le masque), qui veulent dire « float » :
ré-étiquetés en f32 au milieu d'un graphe Flex32, ils passaient inaperçus sans
fusion (les noyaux binaires prennent le dtype de gauche) et faisaient paniquer
la fusion (`DTypeMismatch`, IR stricte). Le no-op est le bon comportement ;
c'est au chargement qu'on pose Flex32, pas en cours de graphe.

Et une subtilité qui a coûté une heure : dans BGE-M3 (et Granite) le cast se
retire, mais dans MiniLM et les rerankers **il faut le garder**. La règle
dépend de la façon dont l'embarqueur *charge* le graphe, pas du graphe :
BGE-M3 et Granite passent par `BurnpackStore` + `Flex32Adapter`, la constante
que le masque rencontre arrive en Flex32 ; MiniLM et les rerankers passent par
`Model::from_bytes`, tout y reste f32 (ils ne profitent d'ailleurs pas de
Flex32), et le cast est ce qui garde le masque cohérent. `patch_attention.py`
retire les casts là où il a fusionné l'attention, ou sur `--casts-neutres`. Même logique dans notre code : le masque du mean-pooling des
MiniLM (`attention_mask.float()`, donc Flex32) prend maintenant le dtype de la
sortie du graphe (f32), sinon la fusion refuse le produit.

Les 1, 3, 4 sont trois forks (L-Defraiteur/cubecl, /burn, /cubek), une
branche `rag3weaver/pre.2` partant du tag exact du lock avec un commit d'une
ligne chacune, prises par `[patch.crates-io]` en git ; la branche
`rag3weaver/pre.3` porte le même commit sur la version suivante. burn main a
encore les trous 2, 3 et 4 : ce sont quatre PR amont d'une ligne. Voir
`docs/forks-burn-cubecl-cubek.md`.

## 7. Ce que ça donne, Vulkan, carte TV (gfx1201), dépendances en -O3

| lot | f32 (état du matin) | **défaut livré** : f32 + autotune | + Flex32 | + Flex32 + fusion (opt-in) | ROCm f32 (issue 02) |
|---|---|---|---|---|---|
| 8 × 100 mots | 5 351 j/s | **8 456** | 4 866 † | 15 063 | 7 109 |
| 32 × 100 | 5 755 | **10 026** | 10 883 | 13 118 | 8 588 |
| 64 × 100 | 3 803 | **7 680** | 11 529 | 12 561 | 7 772 |
| 32 × 300 | 3 327 | **7 718** | 8 085 | 8 138 | 7 594 |
| 8 × 900 | 2 009 | **5 261** | 3 802 | 3 664 | 4 601 |
| 2 × 900 | 2 416 | **5 174** | 4 399 | 4 684 | — |
| 12 formes inédites | 4 196 | 1 432 § | 2 184 | 2 324 | 3 134 |
| chauffe (première forme) | 5,0 s | 21 s § | 15,0 s | 4,6 s ‡ | 21,9 s |

† bruité par le test de parité qui tourne en même temps dans le même
processus. ‡ cache d'autotune déjà chaud pour les clés de matmul. § cache
neuf (`cubecl.toml` venait de déplacer la base) : c'est le coût « une fois par
poste ». Les colonnes Flex32 datent d'un cache réglé plus tôt dans la journée ;
à 8×900, le défaut f32 réglé à neuf les dépasse — les gagnants d'un autotune
varient d'un réglage à l'autre, et il faudra retuner Flex32 à cache neuf pour
comparer à armes égales.

Parité contre f32, 16 textes : cosinus minimal **0,999999** en Flex32,
**0,999997** avec la fusion ; écart absolu maximal 2,3e-4. Les 20 clés de
matmul Flex32 se règlent sur `matmul_simple_cyclic_cmma` (tensor cores), les
10 clés d'attention sur `blackbox_accelerated`.

Autrement dit : **le défaut livré, f32 avec autotune, fait ×1,6 à ×2,6
partout et dépasse ROCm f32**, sans rien qui dépende d'AMD ; Flex32 et la
fusion promettent ×1,5 de plus sur les lots courts, l'une sur demande, l'autre
cassée en pre.2 hors BGE-M3. Le prix : l'autotune coûte 15 s par classe de
forme neuve, une fois par poste (cache), et une forme inédite coûte deux fois
plus qu'avant. La stabilisation des formes n'est plus une optimisation, c'est
la contrepartie de l'autotune.

Ce qui est **par défaut** à la fin de la journée (voir §8 pour le passage à
pre.3) : burn 0.22.0-pre.3 par les forks, **Flex32** sans variable
(`RAG3WEAVER_BURN_FLOAT=f32` pour l'ancien comportement), autotune et fusion
(features `burn-autotune` et `burn-fusion`, dans `burn-embedder`),
dépendances en -O3, le graphe patché (attention fusionnée, casts « float »
neutres), `cubecl.toml`.

## 8. Fusion, pre.3, ROCm

**Fusion** (`burn/fusion`) : jamais active avant aujourd'hui — `burn-dispatch`
prend `burn_wgpu::Vulkan`, qui n'est `Fusion<CubeBackend>` que si la feature
est posée sur burn-wgpu, et rien ne la posait. Elle fusionne les LayerNorm
déroulées (`ReduceBroadcasted`, accumulation f32 forcée) et les chaînes
élémentaires : c'est elle qui fait 11 500 → 15 000 sur les lots courts, et
5 700 → 10 000 en f32. Mais **la passe e2e complète la refuse** : les deux
rerankers paniquent (`FusedMatmulAutotuneKey` m=16, n=1, k=512 — la tête de
classification — n'a aucun candidat viable, « Can't execute the autotune
plan »), et l'OCR rend du texte fragmenté sur des boîtes de quatre pixels. Une
feature par binaire, pas par modèle : elle reste opt-in (`burn-fusion`),
utilisable pour le démon BGE-M3 seul, et à réessayer sur pre.3 (« fix!: address
fusion and autodiff edge cases », « reject a padded reference layout »). Trois
pièges rencontrés en route : `burn/fusion` tire `burn-remote` et un tokio ≥
1.51 (`cargo update -p tokio --precise 1.53.1`) ; l'IR de fusion refuse un
mélange f32/Flex32 que les noyaux nus toléraient ; et la sortie d'un graphe
Flex32 sous fusion reste étiquetée Flex32, d'où `convert::<f32>()` sur les
données avant `to_vec` dans l'embarqueur.

**pre.3, mesuré** (script `vers_pre3` : versions, branches `rag3weaver/pre.3`
des forks, `Flex32Adapter` porté sur `bridge::map_data`, et `#![allow(deprecated)]`
le temps de l'essai — `TensorData::to_vec` y cède la place à `try_to_vec`,
absent de pre.2). Un seul binaire, compilé avec `burn-rocm`, dépendances en -O3,
autotune, sans fusion :

| lot | pre.2 défaut (Vulkan f32) | pre.3 Vulkan f32 | pre.3 **Vulkan Flex32** | pre.3 Vulkan Flex32 + fusion | pre.3 ROCm f32 | pre.3 ROCm f16 | pre.3 ROCm Flex32 |
|---|---|---|---|---|---|---|---|
| 8 × 100 mots | 8 456 | 7 391 | **21 636** | 15 455 † | 7 204 | 8 312 ‖ | 13 756 |
| 32 × 100 | 10 026 | 6 997 | 17 616 | **17 716** | 8 563 | 10 672 ‖ | 10 678 |
| 64 × 100 | 7 680 | 7 124 | 16 950 | **21 482** | 7 748 | 10 560 ‖ | 14 975 |
| 32 × 300 | 7 718 | 4 195 | 8 477 | **9 831** | 4 341 | 10 504 ‖ | 7 077 |
| 8 × 900 | **5 261** | 2 210 | 3 954 | 4 282 | 2 592 | 6 008 ‖ | 4 232 |
| 2 × 900 | 5 174 | 2 733 | 5 047 | **5 353** | 2 760 | 6 511 ‖ | 4 442 |
| 12 formes inédites | 1 432 | 2 771 | 10 748 | **11 016** | 3 078 | échec | 3 804 |
| chauffe | 21 s | 13 s | 5,9 s | 4,9 s | 24 s | 112 s | 21 s |

‖ des vecteurs entièrement NaN : le débit d'un calcul faux ne compte pas.

Ce que ça dit :

- **Le f16 ROCm compile et tourne sur la gfx1201 avec pre.3** : le trou WMMA
  RDNA4 est bien bouché, pas de fork. Mais le f16 pur rend des vecteurs
  **entièrement NaN** (LayerNorm et softmax en f16, comme le cosinus 0,708 de
  pre.2 le laissait voir), et sur les formes inédites cubecl émet des noyaux
  qui incluent `rocwmma/rocwmma.hpp`, absent du poste (`fatal error: file not
  found`) — le paquet `rocwmma` existe dans le dépôt de la distribution. La
  bonne précision sur ROCm est Flex32, comme sur Vulkan.
- **Vulkan Flex32 sur pre.3 fait ×2 de plus que sur pre.2** sur les lots courts
  (21 636 contre 10 883 sur 8×100), et **les formes inédites coûtent quatre fois
  moins** (2,6 s contre 12 s pour les douze) — cache de noyaux à deux niveaux et
  gestion mémoire refaits entre les deux versions.
- Le **f32 sur pre.3 est moins bon** sur les longs lots (8×900 : 2 210 contre
  5 261). Pas creusé : le défaut à monter n'est pas f32, c'est Flex32.
- ROCm Flex32 sur pre.3 (cosinus 0,99999) vaut Vulkan Flex32 sur les lots
  courts et le dépasse un peu sur les longs (4 232 contre 3 954 à 8×900) —
  pas de quoi en faire un défaut : **pre.3 + Vulkan Flex32** est la cible,
  ROCm Flex32 une option détectée.

**Sur l'ingestion réelle** (`e2e_mesure_ingestion_code`, les 30 fichiers de
`src/dataflow`, par le démon) : **75 s**, contre 149 s à la fin de l'issue 01
et 862 s au réveil. L'objectif « sous 100 s » de l'issue 02 est passé, sans
avoir encore touché aux formes de lot (A).

**L'arbre est passé sur pre.3 le soir même.** Les huit `to_vec` sont devenus
`try_to_vec`, `Flex32Adapter` passe par `bridge::map_data`, les branches
`rag3weaver/pre.3` des forks portent les trois lignes. En Flex32, les six
suites de modèles passent (MiniLM, MiniLM multilingue, reranker, XLM-R, OCR,
BGE-M3), à trois tests de déterminisme près qui exigeaient l'égalité **au bit
près** entre deux appels : avec l'autotune, le premier appel d'un processus
est rendu par un candidat en cours de banc, et un noyau accéléré ne fixe pas
l'ordre des additions (8,648816 contre 8,648815). Réécrits : chauffe, puis
égalité à 1e-3. Et **la fusion ne casse plus le reranker sur pre.3** (5/5).

## 8 bis. Trois points de comparaison : Vertex, llama.cpp, MiniLM

Lucie, le soir : *« toujours affreux si on envisage d'indexer le kernel Linux
de 90k docs, faudrait des jours »*. À 75 s pour 1,15 Mo de source, un noyau
(~1,3 Go) c'est de l'ordre de 24 h sur une carte. Avant de chercher plus loin
dans le moteur, trois repères sur **les mêmes lots** que le banc :

| lot | **local, défaut final** (BGE-M3 Flex32, Vulkan) | llama.cpp (BGE-M3 f16, Vulkan, même carte) | Vertex `text-embedding-005`, un appel | MiniLM anglais (22 M, 384 dim) | MiniLM multilingue (118 M, 384 dim) |
|---|---|---|---|---|---|
| 8 × 100 mots | 24 212 j/s | 22 889 (pp816) | 593 (2,2 s de latence) | **53 706** | 18 154 |
| 32 × 100 | 21 225 | 23 404 (pp3264) | 3 601 | **87 856** | 27 735 |
| 64 × 100 | 21 882 | 22 044 (pp6528) | 6 258 | **91 043** | 29 085 |
| 32 × 300 | 9 891 | 20 423 (pp9664) † | 7 195 | **54 757** | tronqué ‡ |
| 8 × 900 | 4 294 | 13 183 (pp7216) † | 8 222 | tronqué ‡ | tronqué ‡ |
| 2 × 900 | 5 368 | 29 401 (pp1804) † | 5 912 | tronqué ‡ | tronqué ‡ |

‡ les MiniLM tronquent (512 jetons pour l'anglais, 128 pour le multilingue) :
au-delà, ils rendent des chiffres qui comptent des jetons jamais lus, ignorés ici.

† `llama-bench -p N` traite **une** séquence de N jetons, pas un lot de
séquences : sur les lots longs les deux colonnes ne mesurent pas la même
attention (une séquence de 9 664 jetons n'existe pas chez nous, notre 32×300
en a 32 de 300). Sur les lots courts, où l'attention ne pèse rien, c'est
comparable, et **on est à parité avec llama.cpp** — le comparateur
« spécialisé » de ragforge (TEI) ne tourne pas sur RDNA4, llama.cpp en tient
lieu. Le moteur n'est plus le problème.

**Vertex** compte 1,5 fois plus de jetons que nous sur les mêmes textes (1 280
contre 816 sur 8×100 : notre compte « un jeton par mot » sous-estime), donc
ses jetons/s sont à lire ×1 et les nôtres ×1,5. Un appel à la fois plafonne
à 8 000 jetons/s Vertex, latence de 1,4 à 2,2 s ; à 8 appels en vol, le quota
`online_prediction_requests_per_base_model` (`textembedding-gecko`) répond
**429** dès la deuxième seconde, et le quota est par minute : relancé à un
appel en vol juste après, tout est encore refusé. Il n'y a pas de débit
soutenu à mesurer sans demander une augmentation. Avec le quota tel qu'il est, le local va trois à quatre fois plus vite
que Vertex ; la question n'est donc pas le moteur mais **la quantité de texte
qu'on lui donne** (§10 : embarquer moins, modèle à la taille du corpus,
pipeline, deux cartes, incrémental).

**MiniLM anglais** fait ×2,2 à ×4 sur BGE-M3 sur les lots courts (à 22 M de
paramètres contre 560 M on attendrait plus : à cette taille, ce sont les
lancements de noyaux et la tokenisation qui bornent, pas la carte — le banc
mesure 15 ms par lot de 8 textes). Le multilingue (XLM-R base, 12 couches,
vocabulaire de 250 k) ne fait que ×1,2. Pour du code en anglais, MiniLM
anglais est le candidat du premier étage ; il reste à mesurer ce qu'il vaut
*en qualité* sur nos requêtes de code, ce que ce banc ne dit pas.

**Les candidats multilingues de la classe bge-base**, mesurés par
`llama-bench` (Vulkan1, f16, une seule séquence par banc — ce qui sous-sature
la carte et fait osciller les 12×768 entre deux plateaux, 38 000 et 60 000 ;
les fenêtres de 512 des BERT/XLM-R n'acceptent pas plus) :

| modèle | corps | dim | fenêtre | pp256 | pp512 | pp3264 |
|---|---|---|---|---|---|---|
| BGE-M3 (référence) | 24 × 1024 | 1024 | 8192 | 19 267 | 26 977 | 25 122 |
| multilingual-e5-base | 12 × 768 | 768 | 512 | 26 198 | 38 000–60 323 | — |
| granite-embedding-278m-multilingual | 12 × 768 | 768 | 512 | 37 466 | 38 209–59 175 | — |
| bge-base-en-v1.5 (ragforge, anglais) | 12 × 768 | 768 | 512 | 28 149 | 38 070–58 044 | — |
| EmbeddingGemma-300M (bf16) | Gemma | 768 | 2048 | 21 230 | 46 000 | 44 751 |
| multilingual-e5-small | 12 × 384 | 384 | 512 | 29 475 | 61 505 | — |
| granite-embedding-107m-multilingual | 12 × 384 | 384 | 512 | 56 264 | 102 350–146 592 | — |
| paraphrase-multilingual-MiniLM-L12 | 12 × 384 | 384 | 511 | 29 557 | 88 169 | — |
| all-MiniLM-L6-v2 (anglais) | 6 × 384 | 384 | 512 | 83 680 | 116 274 | — |

gte-multilingual-base : pas de GGUF, architecture inconnue de llama.cpp.
Lecture : e5-base et granite-278m coûtent **exactement** ce que coûtait
bge-base-en (même corps, seul le vocabulaire change, et un vocabulaire ne se
calcule pas), soit 1,5 à 2,3× BGE-M3 à séquence unique, et davantage en lots
pleins où le rapport de calcul (×4 à ×5) s'exprime. EmbeddingGemma est le
seul de la classe à accepter 2 048 jetons, et le seul entraîné explicitement
sur du code ; son graphe n'est pas BERT.

Banc : `e2e_banc_bge_m3` (`jetons_par_seconde_minilm`,
`jetons_par_seconde_multilingual_minilm`, à lancer **un par un**, `--exact` :
lancés ensemble ils se partagent la carte), `e2e_banc_vertex_embedding`
(`GOOGLE_APPLICATION_CREDENTIALS`, `GOOGLE_CLOUD_PROJECT`, feature
`openai-llm`), et `llama-bench` sur le GGUF f16 de gpustack.

## 9. Instrumentation

On a mesuré à l'aveugle pendant deux semaines. Ce qui est en place maintenant :

- `env_logger` dans le banc (`RUST_LOG=cubecl_wgpu=debug,cubecl_runtime=info`)
  : « Supported CMMA sizes », « Tuning … », « Load autotune cache ». Sans
  collecteur, ces lignes n'existaient nulle part.
- `CUBECL_DEBUG_LOG=<fichier> CUBECL_DEBUG_OPTION=debug` : chaque compilation
  de noyau, chaque candidat d'autotune et son temps, chaque gagnant. Lourd
  (×5 sur le chaud), à réserver au diagnostic.
- `CUBECL_AUTOTUNE_CACHE=false` pour forcer un retune sans toucher à la base.

## 10. Le sous-chantier, réordonné

| | quoi | état |
|---|---|---|
| 0 | dépendances en -O3 dans les bancs | fait |
| A | formes stables (multiple de 64, tailles de lot fixes) | à faire — la contrepartie de l'autotune |
| B | cache d'autotune | actif ; `cubecl.toml` `[environment] path = "global"` à poser |
| C | ROCm par défaut | **non** : Vulkan d'abord. ROCm Flex32 sur pre.3 vaut Vulkan Flex32 ; à brancher comme option détectée (`/opt/rocm`, paquet `rocwmma`), pas comme défaut |
| D | précision mixte | **fait et par défaut** : Flex32, quatre trous bouchés (forks), six suites de modèles passées |
| E | f16 ROCm sur gfx12 | **fait** : pre.3 monté, ça compile et tourne ; le f16 pur rend des NaN, Flex32 est la bonne précision là aussi |
| F | attention fusionnée | fait sur 23/24 couches ; dépend de `burn-autotune` |
| G | autotune dans le jeu de features | **fait** : `burn-autotune` dans `burn-embedder` |
| H | fusion | **fait et par défaut** sur pre.3 (cassait rerankers et OCR en pre.2) |
| I | Flex32 par défaut | **fait** (`float_dtype_voulu` rend Flex32 sans variable) |
| K | `try_to_vec` | fait, huit sites |
| J | les trois PR amont (cubecl-wgpu, burn-cubecl, cubek-matmul) | à ouvrir depuis les forks ; les entrées `[patch]` disparaissent avec |
