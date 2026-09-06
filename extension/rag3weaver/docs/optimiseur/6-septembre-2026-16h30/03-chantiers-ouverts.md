# Les chantiers ouverts du moteur, dans l'ordre

**6 septembre 2026, session « optimiseur ».** Ce qui reste à faire côté
moteur, avec assez de contexte pour qu'une session neuve prenne un chantier
sans cette conversation. L'état est dans le [01](01-l-etat-du-moteur.md), les
outils dans le [02](02-knowledge-dump.md). Le chemin d'indexation (découpe,
lots, pipeline, base) est à la session architecture, pas ici.

## Ce qui est fini aujourd'hui

| | |
|---|---|
| burn pre.3 par forks épinglés, Flex32, autotune, fusion | 170407802 |
| granite-107m et 278m sur burn, démon paramétré, `budget_conseille()` | cea37ee62 |
| Flex32 pour tous les modèles par un seul chargeur, OCR en f32 | 05bdadc0c |
| chauffe du démon avant d'écouter | 9a05d749a |
| flash attention sur tous les graphes BERT (masque booléen) | 02d709e13 |
| docs 02, 03, 04, banc Vertex, scripts pre.3 | 10d7e7114 |
| le banc de qualité (chantier A, ci-dessous), la ligne de précision honnête | [04](04-le-banc-de-qualite.md) |

## A. Le banc de qualité : granite-107m, granite-278m, BGE-M3 sur des requêtes de code

**Fait le 6 septembre au soir**, résultats et recommandation dans le
[04](04-le-banc-de-qualite.md) : granite-278m devant BGE-M3 sur notre code
(MRR 0,844 contre 0,793), granite-107m à 6–9 points derrière, les MiniLM
hors jeu. Reste à Lucie de trancher ; le banc s'affine avec des requêtes
réelles de `e2e_search` et un corpus anglais (le noyau). Le texte d'origine :

**Pourquoi.** Lucie tranche sur deux chiffres, vitesse et qualité. La vitesse
est mesurée (01 §4) ; la qualité relative des trois sur *nos* requêtes ne
l'est pas. IBM donne quelques points d'écart entre 107m et 278m en recherche
de code, pas un gouffre ; 384 dimensions divisent l'index par deux.

**Quoi.** Un test `#[ignore]` qui embarque un petit corpus de code de rag3db
(quelques dizaines de scopes réels, ceux que `e2e_burn_granite` effleure :
`budget_batches`, `charger_burnpack`, le démon, la découpe…) et une trentaine
de questions en français et en anglais avec leur réponse attendue, puis mesure
par modèle le rang de la bonne réponse (MRR, rappel à 1 et à 5). BGE-M3 en
référence de qualité, les deux granite en candidats, MiniLM anglais en plancher.

**Où.** `tests/e2e_banc_qualite.rs`, sur le modèle de `e2e_burn_granite.rs`
(statics `common::burn::GRANITE_107M`, `GRANITE_278M`, `BGE_M3`, `MINILM`).
Les requêtes réelles de la suite `e2e_search` et des gabarits sont un bon
point de départ pour ne pas inventer des questions.

**Comment on saura.** Un tableau modèle × (MRR, R@1, R@5, jetons/s, Mo
d'index pour 20 000 chunks). Si 107m tient à quelques points de 278m, c'est
lui ; sinon 278m ; BGE-M3 reste le second étage. Le démon change de modèle
par `RAG3WEAVER_EMBED_MODEL`, le catalogue garde le nom et la dimension dans
`_catalog_meta` (session architecture) et refuse un mélange.

## B. Les quatre PR amont, depuis les forks

**Pourquoi.** Trois entrées `[patch]` et un adaptateur disparaissent le jour
où c'est fusionné ; et ce sont des corrections d'une ligne, faciles à
accepter.

| dépôt | branche de départ | commit | PR |
|---|---|---|---|
| tracel-ai/cubecl | `rag3weaver/pre.3` du fork | `crates/cubecl-wgpu/src/backend/vulkan.rs` : `Flex32` dans `register_types` | « wgpu/vulkan: register Flex32 like WGSL does » |
| tracel-ai/burn | idem | `crates/burn-cubecl/src/ops/tensor.rs` : `DType::Flex32` dans `float_from_data` | « burn-cubecl: accept Flex32 in float_from_data » |
| tracel-ai/cubek | idem | `crates/cubek-matmul/src/definition/elems.rs` : Flex32 accumule en f32 | « matmul: Flex32 output accumulates in f32 » |
| tracel-ai/burn | à écrire | burn-std `convert_dtype(Flex32)` doit étiqueter Flex32 (`convert_inplace_with` pose `Target::dtype()` = f32) | « TensorData::convert_dtype keeps Flex32 » |

Les commits existent sur les branches `rag3weaver/pre.3` (et `pre.2`) des
forks, en français ; à rebaser sur `main` de chaque dépôt avec un message en
anglais et le motif « Flex32 was unusable end to end: … » (la preuve : des
vecteurs identiques au bit près à f32). Vérifier avant que `main` n'a pas
déjà bougé (burn main avait encore les trous 2, 3 et 4 le 6 septembre).

## C. L'OCR en Flex32 : la dérive des convolutions

**Ce qu'on sait.** En Flex32 le détecteur PP-OCRv6 rend une carte vide
(max 0,0000), alors que chaque opération prise seule est exacte (convolutions
larges, depthwise, 1×1, à stride, interpolate, sigmoid, hardswish, maxpool,
normalisation, concat : cosinus 1,000000), et que dans le vrai graphe le
signal dérive étage après étage (tronc : |max| 16 contre 20, 4,7 contre 15,
5,4 contre 8,8, 2,0 contre 17) jusqu'à la tête. Ni un seuil, ni un cast.

**Les outils.** `tests/e2e_burn_ocr.rs` : `carte_de_detection_selon_la_precision`,
`une_convolution_seule_selon_la_precision`, `les_operations_du_detecteur_selon_la_precision`,
`les_convolutions_du_detecteur_selon_la_precision`, `le_detecteur_etage_par_etage`
(le graphe généré expose `stades()`). L'interrupteur : `PRECISION_OCR` dans
`src/burn_ppocr.rs` (mettre `float_dtype_voulu()` et relancer f32 puis flex32).

**Par où.** Les sondes exactes étaient bit-identiques, donc les convolutions
sondées ne prenaient sans doute pas le chemin f16 (trop petites) ; le vrai
graphe le prend (184 × 616). Sonder une convolution à la taille réelle du
premier étage avec les vrais poids, comparer f32 / Flex32, puis descendre
dans cubek-convolution (implicit GEMM, `adjust_dtypes`, accumulateur). Si
c'est un bug amont, c'est une cinquième PR. Le gain attendu est modeste
(l'OCR n'est pas sur le chemin d'indexation du code) : chantier de fond, pas
d'urgence.

## D. Le conseil de lot à 256, et l'autotune qui essaie la voie naïve

**Ce qu'on sait.** L'attention est fusionnée partout ; ce qui retient le
conseil à 128 × 512, c'est que l'autotune de l'attention garde le candidat
naïf dans son plan (priorité minimale, mais essayé), et qu'à 256 séquences de
512 il demande 3,2 Go d'un tenseur, au-dessus de la taille maximale d'un
tampon wgpu : le serveur cubecl panique et se rattrape (pre.3).

**Quoi.** Soit retirer le candidat naïf du plan quand `seq × têtes × L² × 4`
dépasse la limite du tampon (dans burn-cubecl `kernel/attention/tune.rs`, une
priorité qui devient « exclu » — une PR amont de plus, ou une ligne dans le
fork), soit laisser 128 : la carte est saturée à 128 déjà, le gain à 256 est
sur la traîne des longueurs, pas sur le débit. À mesurer avant de choisir.

## E. La dépendance au poste

- `rocwmma` (paquet de la distribution) pour que ROCm f16/Flex32 compile sur
  toutes les formes ; ROCm reste une option détectée (`/opt/rocm`), à brancher
  dans `regime.rs::carte_locale` si on veut la choisir automatiquement.
- Le -O3 de notre crate en profil dev : Lucie a refusé l'édit ; les mesures
  restent avec notre crate en debug. À rediscuter (un profil `banc` dédié ?).
- Publier les burnpacks granite sur Hugging Face comme BGE-M3
  (`Lucie666/bge-m3-burnpack`), et leur sha256 dans `generated/README.md`
  (déjà notés).

## F. Ce qui n'est pas à moi mais que je surveille

- Les lots par modèle en jetons, arrondis au multiple de 64, tailles fixes
  (session architecture, en cours) : c'est ce qui absorbe le « premier » de
  chaque forme.
- Le choix final du modèle d'indexation : Lucie, sur le banc A.
