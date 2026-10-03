# Une seule carte, partagée avec l'affichage

**3 octobre 2026.** Lecture du code sur `origin/master`, faits du poste lus
dans `/sys/class/drm`. Aucune mesure nouvelle : les durées sont **calculées**
depuis les débits mesurés ici le 2 octobre par la session optimiseur.

> y a des embeddings en cours là ? car mon écran freeze ; on peut pas bosser
> avec granite sur ce PC ?

On peut. Le poste est au régime `plein` — aucune variable posée — et `plein`
est fait pour une carte qu'on a pour soi.

## 1. Ce que le poste est, et ce que le régime en fait

| | |
|---|---|
| cartes | **une** : `card1`, Radeon 8060S (iGPU), deux écrans actifs dessus (eDP, HDMI) |
| `mem_info_vram_total` | **4 096 Mio** — la réserve posée au BIOS, pas une capacité ; la mémoire réelle est le GTT, 112 Gio |
| `RAG3WEAVER_REGIME` | non posée → **`plein`** |

Sous `plein`, `carte_partagee()` rend `false` **par définition** (`regime.rs:147` :
« la question ne se pose pas »). Il en découle : rapport cyclique 100 %, aucune
pause, et le lot suit le conseil du modèle (`lot_budget`, `embedder.rs:925`) —
pour granite, 128 séquences de 512 jetons, jusqu'à 196 608 caractères par
appel, borné par la surface d'attention.

Sous `confort`, la même fonction trouverait la carte partagée — une seule
carte, `least_watched_card` ne rend rien — et donnerait 60 % de rapport
cyclique et des rafales de 2 048 caractères, conseil du modèle ignoré.

## 2. Pourquoi l'écran se fige

Un appel `embed()` est une série de noyaux que rien ne préempte : ni wgpu ni
Vulkan n'exposent de priorité de file (`embedder.rs:799`). Le compositeur
attend la fin de l'appel. **Le gel dure ce que dure une rafale** :

| | jetons par appel | granite-278m (26 000 j/s) | granite-107m (148 000 j/s) |
|---|---:|---:|---:|
| `plein`, chunks de code (~333 jetons × 128) | ~42 000 | **1,6 s** | 0,29 s |
| `plein`, scopes du banc (~500 jetons × 67) | ~33 000 | **1,3 s** | 0,23 s |
| rafale de 8 192 caractères | ~2 700 | 105 ms | 18 ms |
| rafale de 2 048 caractères (`confort`) | ~680 | 26 ms | 5 ms |

À 100 %, les rafales s'enchaînent sans trou : l'écran avance d'une image
toutes les 1,3 à 1,6 s. C'est ce que Lucie a vu — `e2e_banc_etage` charge
granite-278m **dans le processus du test** (`tests/common`, `GRANITE_278M`),
donc par ce chemin exact. Les quatre compilations en parallèle n'y sont pour
rien : c'est la file de la carte, pas le processeur.

**Ce qui se règle** : la longueur d'une rafale (`RAG3WEAVER_EMBED_CHAR_BUDGET`),
le trou entre deux (`RAG3WEAVER_GPU_DUTY`), les deux ensemble
(`RAG3WEAVER_REGIME=confort`), et le modèle. **Ce qui ne se règle pas** : la
priorité sur la carte, et la longueur de séquence — un chunk de 512 jetons ne
se coupe pas en deux appels.

## 3. Le réglage, quand quelqu'un est devant l'écran

**Tout de suite, sans code :**

```sh
RAG3WEAVER_REGIME=confort
```

Rafales de 26 ms en 278m, 17 ms de trou après chacune : l'écran ne se fige
plus. **Coût** : le rapport cyclique seul fait ×1,7 ; les lots de deux à
quatre chunks au lieu de soixante perdent le bénéfice du lot. L'en-tête de
`regime.rs` donne l'ordre de grandeur mesuré le 6 septembre — « quatre fois le
temps ». À confirmer ici par une passe ; je ne l'ai pas mesuré.

**Moins cher, par les deux variables explicites** (elles gagnent sur le
régime) — viser une rafale de ~50 ms, trois images :

| modèle | à poser | rafale | coût estimé |
|---|---|---:|---|
| granite-278m | `RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70` | ~52 ms | ×2 à ×2,5 |
| granite-107m | `RAG3WEAVER_EMBED_CHAR_BUDGET=16384 RAG3WEAVER_GPU_DUTY=80` | ~37 ms | ×1,3 à ×1,5 |

**Où la poser** : dans l'environnement du processus **qui embarque**. Pour un
banc qui charge le modèle lui-même, celui du test. Par le démon, celui du
démon *à son lancement* — un démon déjà là garde son régime ; il faut le
quitter pour qu'il le relise.

## 4. Ce qui manque : la rafale se règle en caractères, elle se vit en millisecondes

Le budget de 2 048 caractères a été calibré le 27 août sur BGE-M3 et une
R9700. Sur ce poste, la même rafale dure 26 ms en 278m et 5 ms en 107m : trop
prudente pour l'un des deux, et rien ne le dit.

**La plus petite modification générique** — proposée, pas codée : sur une carte
partagée, viser une **durée** de rafale (30 à 50 ms) plutôt qu'un nombre de
caractères. `souffler` chronomètre déjà chaque lot : le lot suivant se
dimensionne par règle de trois sur la durée du précédent, borné. Aucune table
par modèle ni par carte — le poste se calibre en trois lots, et le même code
sert l'iGPU d'un portable comme une carte dédiée qu'on partage.

Et un défaut à côté : `plein` répond « non partagée » sans regarder. Sur un
poste à une carte qui porte l'écran, c'est le régime par défaut qui fige —
le cas ordinaire d'un utilisateur. La détection existe
(`least_watched_card`, les connecteurs `enabled`) ; reste à décider si le
défaut d'un poste à une carte doit être `confort`. C'est un choix de produit,
à Lucie.

## 5. L'heuristique du modèle

`card_class` lit 4 096 Mio, le plancher est 4 Gio, et le test est strict :
**ce poste est classé « carte dédiée »**, donc granite-278m. À un mébioctet
près au BIOS, il basculait en « faible ». Deux défauts :

- `mem_info_vram_total` d'un iGPU est une réserve, pas une capacité. Le
  critère de mémoire ne dit rien d'une carte à mémoire partagée.
- **« partagée avec l'affichage » n'y est pas**, alors que c'est ce qui
  compte ici : 107m va 5,7 fois plus vite, donc ses rafales durent 5,7 fois
  moins à lot égal. Au réglage du §3, 107m coûte ×1,3 quand 278m coûte ×2.

**Proposition** : un troisième déclencheur pour 107m au premier index —
*la seule carte du poste porte l'affichage*. Le coût est connu (6 à 7 points
de MRR, banc de l'optimiseur) ; le gain est un poste qui reste utilisable
pendant qu'il indexe. À trancher par Lucie avec les deux autres : 50 000
documents, carte faible ou absente.
