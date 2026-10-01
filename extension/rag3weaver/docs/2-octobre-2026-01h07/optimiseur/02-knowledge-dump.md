# Knowledge dump — optimiseur (moteur d'embarquement burn)

**2 octobre 2026.** Tout ce que la session « optimiseur » sait de sa partie, à
jour de cette nuit. Les documents plus anciens restent la référence de détail :
[`docs/optimiseur/6-septembre-2026-16h30/`](../../optimiseur/6-septembre-2026-16h30/01-l-etat-du-moteur.md)
(état du moteur, knowledge dump, chantiers), `7-septembre-2026-21h56/` (banc
HNSW), `2-octobre-2026-00h15/` (les sept PR amont, les forks réécrits).
Le [01](01-rapport-de-session.md) dit ce qui s'est fait cette nuit.

Convention : « mesuré » veut dire mesuré, avec le matériel ; ce qui ne l'est
pas est dit tel.

## 1. Le périmètre

Le moteur d'embarquement en Rust sur burn : les embedders (`src/burn_*_embedder.rs`),
les rerankers, l'OCR (`src/burn_ppocr.rs`), le choix de carte et de précision
(`src/burn_device.rs`), le démon d'embarquement (`src/bin/rag3weaver-embeddings.rs`,
`src/daemon/embeddings.rs`), les graphes générés (`generated/`), les forks de
burn, cubecl et cubek, les bancs (`tests/e2e_banc_*`, `tests/e2e_burn_*`). Le
chemin d'indexation, le catalogue, la base et le cœur C++ sont à d'autres
sessions.

## 2. La pile, et la ligne qui la résume

Chaque modèle imprime au chargement, depuis `burn_device::resolve` :

```
[rag3weaver] burn : <carte demandée> → <carte résolue> · précision <…> · autotune oui/non · fusion oui/non
```

Si elle dit autre chose que « Flex32 (défaut) · autotune oui · fusion oui »,
les chiffres de ce document ne s'appliquent pas.

| couche | version | d'où |
|---|---|---|
| burn | 0.22.0-pre.3 | fork personnel, branche `rag3weaver/pre.3`, révision `21674205` |
| cubecl | 0.11.0-pre.3 | idem, `bdf6b77a` |
| cubek | 0.3.0-pre.3 | idem, `7ba8affd` |
| backend | Vulkan (wgpu + SPIR-V), pilote radv | `Device::vulkan` |
| précision | **Flex32** par défaut : stockage f32, matmul et attention en f16 sur les matrices coopératives, accumulation f32 | `RAG3WEAVER_BURN_FLOAT=f32\|f16\|bf16\|flex32` |
| autotune, fusion | oui (features `burn-autotune`, `burn-fusion`) | cache global `~/.cache/cubecl/default.db`, `cubecl.toml` |

L'amont en est à pre.4 (taguée sur les trois dépôts). Nous n'y sommes pas
montés. Ce qui nous attend à la montée : leur `main` a rendu privés les champs
de `TensorData` (notre `Flex32Adapter`, qui ré-étiquette les octets d'un pack,
sera à reporter — il disparaît si la PR 4 est acceptée), et burn a retiré le
paramètre de runtime de `CubeBackend` (deux de nos correctifs ont dû être
reportés à la main pour les PR).

## 3. Les forks et leur épinglage

Trois forks publics sous le compte personnel de Lucie, forks de tracel-ai,
une seule branche à nous chacun : `rag3weaver/pre.3`, partie du tag pre.3.

| fork | sommet | commits à nous |
|---|---|---|
| burn | `21674205` | 3 (`float_from_data` Flex32 ; flash en Flex32 et voie naïve en lice ; locaux de la fusion) |
| cubecl | `bdf6b77a` | 1 (Vulkan déclare Flex32) |
| cubek | `7ba8affd` | 2 (matmul accumule en f32 ; pas de ligne du masque) |

**L'épinglage** : `extension/rag3weaver/Cargo.toml`, section
`[patch.crates-io]`, **51 entrées** (25 burn, 15 cubecl, 11 cubek) de la forme
`{ git = "…", rev = "<8 caractères>" }`, et les lignes correspondantes de
`Cargo.lock` (hash court dans `rev=`, hash complet après `#`). On patche
**chaque atelier en entier** : patcher un seul crate d'un dépôt git tire ses
voisins en double de ceux du registre, et deux `cubecl_common::Device` ne sont
pas le même trait.

**Changer de révision** : remplacer les hashes dans les deux fichiers (complets
d'abord, courts ensuite — le court est un préfixe du complet), jamais un
`cargo update` large ; `cargo metadata --locked` prouve le lock et le
téléchargement sans rien compiler ; vider le cache d'autotune si un noyau a
changé. **Ce qu'une réécriture de branche touche** : ces 51 entrées et le
lock, sur master *et sur chaque branche vivante de rag3db* ; tout commit de
rag3db antérieur à la remontée épingle des révisions qui ne sont plus sur une
branche et ne se construit plus sur un poste qui ne les a pas dans
`~/.cargo/git`.

**Itérer sur un fork sans toucher au `Cargo.toml` partagé** : un fichier
`[patch.crates-io]` par chemin vers un clone local (tous les crates de
l'atelier), passé par `cargo --config <fichier>`. On ne pousse que quand la
mesure est bonne.

**Sauvegardes** (hors git, `.vault/forks/`) : un bundle par fork de la branche
`rag3weaver/pre.3`, un bundle par dépôt des branches de PR, et
`recreer-forks.sh` (recrée les forks et repousse les branches ; mode
`verifier` en lecture seule).

## 4. Flex32, et les sept défauts

Flex32 est un type flottant de cubecl : stocké en f32, calculé en f16 là où le
matériel a des matrices coopératives, accumulé en f32. Sur Vulkan il n'était
utilisable nulle part de bout en bout ; chaque trou rendait des vecteurs
**identiques au bit près à f32**, sans un message — d'où la règle : une
optimisation se vérifie par l'écart absolu maximal, et un écart nul veut dire
que rien n'a changé.

| # | où | le défaut | ce que ça donnait |
|---|---|---|---|
| 1 | cubecl-wgpu, `backend/vulkan.rs` | Vulkan ne déclare pas Flex32 parmi ses types (WGSL si) | burn refuse Flex32 comme précision d'une carte Vulkan ; les noyaux, eux, tournent |
| 2 | burn-cubecl, `ops/tensor.rs` | `float_from_data` ne connaît pas Flex32 | panique au chargement d'une donnée marquée Flex32 |
| 3 | cubek-matmul, `definition/elems.rs` | une sortie Flex32 n'accumule pas en f32 | les candidats à matrices coopératives sont écartés, le matmul reste en f32 |
| 4 | burn-std, conversion de `TensorData` | convertir vers Flex32 rend une donnée marquée F32 | `FloatCastAdapter` ne fait rien pour Flex32 ; chez nous, contourné par `Flex32Adapter` |
| 5 | burn-cubecl, `kernel/attention/` | la flash accélérée exige type global = type de tuile et refuse Flex32 ; burn dégrade en silence vers la voie naïve, jusque dans les candidats d'autotune | la flash ne se lançait jamais ; 3 Gio de scores à 256 × 12 × 512² |
| 6 | cubek-attention, lecteur du masque | le masque matérialisé est lu avec `seq_kv` pour pas de ligne | un masque de remplissage diffusé sur `seq_q` est lu de travers (cosinus 0,04 contre la voie naïve) |
| 7 | burn-cubecl-fusion, pool de locaux | F32 et Flex32 partagent un registre mais sont numérotés à part | dès qu'un noyau fusionné promeut du Flex32 en F32 (`hard_sigmoid`), une entrée est écrasée ; l'OCR rendait une carte vide |

Dans nos forks, la 5 porte aussi un réglage d'autotune (la voie naïve reste en
lice sous 256 Mio de scores, elle est plus rapide sur les séquences courtes) :
laissé hors des PR, à proposer en issue.

État de l'amont, des vérifications et des textes : `docs/optimiseur/2-octobre-2026-00h15/`
et le [01](01-rapport-de-session.md).

**Deux outils de diagnostic qui ont payé** : la sonde par traceur (un point de
synchronisation après chaque instruction d'un graphe généré, puis « partout
sauf une famille ») pour trouver quel noyau fusionné dévie ; et
`CUBECL_DEBUG_LOG=<fichier>` avec `CUBECL_DEBUG_OPTION=debug`, qui dépose la
configuration de chaque noyau (ops, locaux, types) avant sa source — c'est là
qu'on lit les candidats d'autotune et leurs erreurs.

## 5. Les modèles et leurs poids

Tous chargés par `burn_device::charger_burnpack(modèle, octets, nom, précision)`.

| modèle | corps | dim | fenêtre | rôle |
|---|---|---|---|---|
| **granite-278m** | XLM-R, 12 × 768 | 768 | 512 | **défaut** du produit (décision de Lucie, 7 septembre) |
| granite-107m | XLM-R, 6 × 384 | 384 | 512 | régime rapide ; premier index d'un gros dépôt |
| BGE-M3 | XLM-R large, 24 × 1024 | 1024 | 8 192 | dense + creux appris ; documents, second étage |
| MiniLM anglais, MiniLM multilingue | BERT 6 et 12 × 384 | 384 | 512 / 128 | petits modèles, hors jeu en qualité sur le code |
| rerankers ×3 | MiniLM, XLM-R | logit | — | reclassement |
| PP-OCRv6 tiny | convolutions | — | — | OCR ; suit la précision de la carte depuis le 18 septembre |

**Qualité sur notre code** (banc `e2e_banc_qualite`, 67 scopes réels, 45
questions fr/en ; mesuré le 6 septembre sur l'ancien poste) : granite-278m MRR
0,844, BGE-M3 0,793, granite-107m 0,779, MiniLM 0,61 et 0,57.

**Où vivent les poids** : `~/.cache/rag3weaver/<modèle>/{model.bpk,tokenizer.json}`
(`bge-m3`, `granite-278m`, `granite-107m`, `minilm`, `multilingual-minilm`,
rerankers, `ppocrv6-tiny`), ou les variables `RAG3WEAVER_<MODELE>_BPK` /
`_TOKENIZER`. Pas dans le dépôt. Le graphe, lui, est dans `generated/*_onnx.rs`.

**Régénérer un pack** (recette dans `generated/README.md`, rejouée cette nuit
pour granite) : télécharger l'ONNX d'origine et vérifier son empreinte ; un
binaire de trois lignes avec `burn-onnx` **à la version exacte** qui a produit
le graphe (`ModelGen` + `LoadStrategy::Bytes`) ; passer le code généré par
`generated/patch_attention.py` — le résultat doit être le fichier du dépôt, à
l'en-tête près, sinon les poids ne vont pas avec le code. Deux secondes par
modèle une fois le convertisseur construit.

**Prouver un pack** : il n'est pas reproductible à l'octet (même taille,
empreinte différente), ses valeurs le sont. La preuve est l'écart absolu
maximal des vecteurs contre onnxruntime sur l'ONNX d'origine, phrase par
phrase : `tests/fixtures/granite/reference-{278m,107m}.json` (24 phrases, 4 à
512 jetons), `generated/reference_granite.py`, et le test
`e2e_burn_granite::granite_<taille>_contre_la_reference_onnx` (seuil 5·10⁻⁴).
Mesuré le 2 octobre, iGPU Radeon 8060S : 3·10⁻⁷ en f32 pour les deux ;
4,6·10⁻⁵ (278m) et 7,6·10⁻⁵ (107m) en Flex32. BGE-M3 et les autres n'ont pas
de références de ce genre dans le dépôt : à faire si on les régénère.

## 6. Hugging Face

Compte de Lucie : `Lucie666` ; jeton en écriture dans `.vault/hf.env`
(variable `HF_ACCESS_TOKEN`). Toute requête à l'API avec un User-Agent.

| dépôt | état |
|---|---|
| `bge-m3-burnpack`, `all-minilm-l6-v2-burnpack`, `paraphrase-multilingual-minilm-l12-v2-burnpack`, `ms-marco-minilm-l6-v2-burnpack`, `mmarco-mminilmv2-l12-h384-v1-burnpack`, `bge-reranker-v2-m3-burnpack`, `ppocrv6-tiny-burnpack` | publics |
| `granite-embedding-278m-multilingual-burnpack`, `granite-embedding-107m-multilingual-burnpack` | **privés**, publiés le 2 octobre ; le passage en public attend le mot de Lucie |

Les fiches sont en anglais, sur un même gabarit : ce n'est pas notre modèle,
conversion mécanique de format, auteurs et licence d'origine (Apache-2.0 pour
granite, IBM Research), provenance, empreintes, et pour granite la
vérification mesurée. Aucune mention d'outil d'écriture. Les empreintes et
tailles sont dans `generated/README.md`.

## 7. Le démon d'embarquement

`rag3weaver-embeddings` : sert un modèle sur `127.0.0.1:7878`
(`RAG3WEAVER_EMBEDDINGS_ADDR`). `RAG3WEAVER_EMBED_MODEL` = `granite-278m`
(défaut, aussi quand la variable est vide), `granite-107m`, `bge-m3`. Son
`Identite` porte modèle, dimension, précision, conseil de lot ; le client
`DaemonEmbedder` les relaie. Il chauffe seize classes de forme avant
d'annoncer son adresse (`RAG3WEAVER_DEMON_CHAUFFE=0` pour s'en passer).

Pièges : un test attaché depuis un binaire plus neuf trouve le démon en place
« périmé » et **le remplace** ; un banc qui ne doit pas y toucher pose
`RAG3WEAVER_SANS_DEMON=1`. Les suites qui veulent BGE-M3 par le démon le
demandent elles-mêmes au lancement. On tue un démon par `pidof`, jamais par un
motif. Cette nuit, le démon du port 7878 servait bge-m3 et n'était pas le
mien ; mes vérifications ont utilisé un démon hermétique sur un port libre.

Le garde-fou « un index, un modèle » est côté catalogue ; depuis septembre un
index peut porter plusieurs modèles (autre session).

## 8. Le poste, l'iGPU et ses débits

`lucie-tablette` : 32 cœurs, 121 Go, **iGPU AMD Radeon 8060S** (RADV
STRIX_HALO), une seule carte. L'ancien poste avait deux cartes RDNA4, dont une
réservée aux mesures : les chiffres de septembre (01 du 6 septembre, §4)
viennent de là et **ne valent pas ici**.

Mesuré sur l'iGPU le 2 octobre (Flex32, autotune, fusion, cache d'autotune
désactivé ; deux autres sessions compilaient, donc à rejouer seul pour un
chiffre ferme) :

| lot | granite-107m | granite-278m |
|---|---|---|
| 32 × 450 mots (512 jetons), deuxième passe | 148 000 jetons/s | 26 000 |
| 256 × 450 mots | 49 000 (une passe, compilation comprise) | — |

Pour comparaison, sur l'ancienne carte : 416 000 et 122 000 à 128 × 512.
**Non mesuré sur ce poste** : BGE-M3, les rerankers, l'OCR en débit, le banc de
qualité, l'ingestion de bout en bout avec granite-278m. La suite granite (11
tests), la suite OCR et la parité BGE-M3 n'ont pas été rejouées ici en entier ;
la suite granite, si : 11 sur 11.

Bancs lourds : un seul à la fois, les annoncer à l'orchestration, `-j8` au
plus quand d'autres sessions compilent, un `target` à part pour ce qui n'est
pas rag3weaver.

## 9. Contribuer à tracel-ai : leurs règles

Lu sur leurs branches principales le 2 octobre.

- **burn** : gabarit de PR — cocher « `cargo run-checks` exécuté » et « le
  livre est à jour » ; sections issues liées, changements, tests. `run-checks`
  = `cargo xtask validate` : format, typos, audit, lint de tout l'atelier,
  vérification no-std, tests de backend (Flex par défaut ;
  `--backend vulkan` pour un GPU). Guide : s'il n'y a pas d'issue, **en ouvrir
  une d'abord**. `rustfmt` à 100 colonnes. Tests de backend dans
  `crates/burn-backend-tests`, lancés depuis ce dossier (`cargo test-vulkan`),
  le dtype flottant choisi par fichier de test (f32, f16 ; **aucun fichier
  Flex32**).
- **cubek** : gabarit — « valider la PR avec burn » : une branche burn
  pointant sur le hash de la PR, une PR burn liée. Alias `cargo test-vulkan`
  (`--release --features cubecl/vulkan,heavy`). `cubek.toml` a une politique
  de test `correct` qui **accepte un noyau qui ne compile pas** : un test
  reproducteur se joue en `strict`. Son outil de test range des données
  logiques entières dans une disposition à strides explicites (strides 0
  compris).
- **cubecl** : même validation en deux étages (cubek, puis burn) ;
  `cargo xtask validate`. La suite SPIR-V : `cargo test -p cubecl-wgpu
  --features spirv --lib` (797 tests sur l'iGPU).
- **Titres** : `fix(<zone>): …`, le numéro de PR est ajouté à la fusion.

## 10. L'identité, et la règle de Lucie

Règle ferme : aucun commit ni aucune PR de ses projets ne porte son adresse
professionnelle ni ne passe par son compte GitHub professionnel.

- L'identité git **globale** du poste est l'adresse professionnelle ; le dépôt
  rag3db signe, en local, de l'adresse personnelle. **Tout clone neuf hérite de
  la globale** : poser `user.name` et `user.email` en local dans chaque clone
  avant le premier commit, et contrôler par `git log --format='%ae %ce'`
  avant tout push.
- `gh` a les deux comptes, le professionnel actif. On ne le change pas : le
  compte personnel se passe par variable pour la seule commande qui en a
  besoin. Le push par ssh authentifie le compte personnel.
- Un commit poussé sur un fork est visible dans tout le réseau du dépôt
  d'origine dès le push, et y reste accessible par son hash après une
  réécriture ou une suppression.
- Les fichiers qui portent des hashes ou des adresses à ne pas publier vivent
  dans `.vault/` (ignoré par git) : la demande au support, le script de
  contrôle, les bundles.

## 11. Travailler à plusieurs sessions

- La session d'orchestration distribue le travail, séquence cargo et la
  carte ; on lui annonce toute compilation lourde et tout banc.
- L'arbre principal est partagé, **l'index aussi** : on commite par
  `git commit -- <chemins>`, on relit `git show --stat`. Pour un commit
  pendant qu'une autre session édite l'arbre : un worktree détaché de
  `origin/master`, commit par chemins, `git push origin HEAD:master` sans
  force, rebase si master a bougé, retrait du worktree.
- Commits en français, sans trailer.
- Le journal des chantiers (`docs/journal-des-chantiers.md`) se lit en
  arrivant.

## 12. Ce qui n'est pas fait

- Les vérifications des PR 7, 3, 2, 5, et `cargo run-checks` côté burn.
- Aucune des sept PR n'est ouverte ; la forme de l'envoi n'est pas tranchée.
- La purge des anciens commits par le support GitHub.
- La montée en pre.4.
- Les bancs de débit et de qualité sur ce poste, hors granite.
- Les chantiers de fond de septembre : `rocwmma` et ROCm comme option
  détectée ; le -O3 de notre crate en profil dev (refusé, à rediscuter).
