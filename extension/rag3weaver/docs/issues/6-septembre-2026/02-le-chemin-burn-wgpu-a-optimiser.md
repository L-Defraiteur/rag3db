# Issue 02 — le moteur d'embarquement : ce qu'il vaut, et ce qu'une autre session peut y faire

> **Dépassée le soir même par l'[issue 03](03-le-chemin-vulkan-ce-qu-il-valait-et-les-lignes-qui-manquaient.md)** :
> les bancs ci-dessous mesuraient burn compilé en -O0, sans autotune ni fusion, et
> plusieurs conclusions (cache d'autotune, coût des formes, f16 ROCm, ROCm par
> défaut) sont réécrites là-bas. Les chiffres restent vrais pour ce qu'ils mesuraient.

**6 septembre 2026, 16h30–17h15.** Suite de l'[issue 01](01-quatorze-minutes-pour-trente-fichiers.md) :
une fois le chemin d'indexation réparé, il reste le modèle lui-même. Lucie :
*« les gens ont pas souvent un setup d'embedder, faut que le mode par défaut
soit parfait, donc il faut optimiser cette voie-là au max »*. Ce document est
le banc, ses chiffres, et le **sous-chantier** qu'il ouvre — pensé pour être
pris par une autre session, sans rien de cette conversation.

## Le banc

`tests/e2e_banc_bge_m3.rs` : BGE-M3 seul, sans base ni découpe, sur la carte
que personne ne regarde (`gpu:1` = `0000:07:00.0`, la carte TV). Jetons ≈ mots.

```sh
RAG3WEAVER_SANS_DEMON=1 ./run_e2e.sh --test e2e_banc_bge_m3                                   # Vulkan f32
RAG3WEAVER_BURN_DEVICE_EMBEDDER=rocm:1 RAG3WEAVER_SANS_DEMON=1 ./run_e2e.sh --features burn-rocm --test e2e_banc_bge_m3
RAG3WEAVER_BURN_FLOAT=f16 RAG3WEAVER_SANS_DEMON=1 ./run_e2e.sh --test e2e_banc_bge_m3        # f16 (voir parité)
```

| lot | Vulkan f32 | ROCm f32 | Vulkan f16 |
|---|---|---|---|
| 8 × 100 mots | 5 051 j/s | 7 109 | 11 426 |
| 32 × 100 | 5 688 | 8 588 | 14 248 |
| 64 × 100 | 3 815 | 7 772 | 16 591 |
| 32 × 300 | 3 358 | 7 594 | 11 677 |
| 8 × 900 | 2 009 | 4 601 | 5 373 |
| 12 formes de lot inédites | 3 443 j/s | 3 134 | **969** |
| chauffe (première forme) | 3,7 s | 21,9 s | 2,5 s |

Carte : Radeon AI PRO R9700, gfx1201, 32 Go, ~48 TFLOPS fp32. Le f32 Vulkan
en tire 13 %.

## Ce qu'on sait

1. **ROCm (HIP) vaut 1,5 à 2,3 fois Vulkan/radv en f32**, sans rien changer
   au modèle. La feature `burn-rocm` existe, ROCm est installé
   (`/opt/rocm`, gfx1201 supporté nativement). Chauffe de 22 s : le démon
   l'amortit.
2. **Le f16 Vulkan va 2,5 à 3 fois plus vite que le f32 — et rend de faux
   vecteurs.** Cosinus minimal **0,708** contre le f32 sur seize textes
   (`le_f16_rend_les_memes_vecteurs`). Tout le graphe en f16, LayerNorm et
   softmax compris : ça déborde. Il faut de la précision **mixte**.
3. **Le f16 ROCm ne compile pas** : `LLVM ERROR: Cannot select: intrinsic
   %llvm.amdgcn.wmma.f32.16x16x16.f16`. cubecl-hip émet une intrinsèque WMMA
   que la chaîne installée ne sélectionne pas pour gfx12. C'est cubecl-hip
   (ou la version de LLVM/HIP), pas nous.
4. **Chaque forme de lot jamais vue coûte des secondes** (autotune + noyaux),
   et en f16 c'est pire : 2,4 s par forme. Or l'indexation, avec ses lots
   triés par longueur, produit des formes presque toutes différentes. cubecl
   a un cache d'autotune persistant (`CUBECL_AUTOTUNE_CACHE`, environnements
   nommés) — pas trouvé sur ce disque, à vérifier s'il est actif.
5. Le coût croît plus que linéairement avec la longueur (2 000 j/s à 900
   mots contre 5 700 à 100) : l'attention est quadratique, et sans noyau
   fusionné.

## Le sous-chantier, pour une autre session

Par ordre de rendement, et chacun avec sa mesure — **toujours le banc, sur
`gpu:1`/`rocm:1`, jamais la carte du bureau** (le régime `confort` choisit
la carte au moins d'écrans actifs ; `RAG3WEAVER_BURN_DEVICE_EMBEDDER` force).

| | quoi | comment on saura | où |
|---|---|---|---|
| A | **Des formes qui se répètent.** Arrondir la longueur de séquence au multiple de 64 (`pad_to_multiple_of` du tokenizer) et les lots à des tailles fixes (8, 16, 32) après le tri par longueur | `12 formes inédites` tombe au niveau du chaud ; `e2e_mesure_ingestion_code` sous 100 s | `burn_bge_m3_embedder.rs` (padding), `embedder::budget_batches` (tailles) |
| B | **Le cache d'autotune sur disque**, s'il n'est pas actif : le configurer (`CUBECL_AUTOTUNE_CACHE`, `[environment] path`) pour que la première forme ne se paie qu'une fois par poste | deuxième lancement du banc : « premier » ≈ « chaud » | config cubecl, `burn_device.rs` |
| C | **ROCm par défaut quand il est là.** `burn-rocm` dans le jeu de features du démon, `rocm:N` choisi par le régime quand `/opt/rocm` existe, repli Vulkan sinon | le banc en f32 : ≥ 7 000 j/s sur 32 × 100 | `Cargo.toml`, `regime.rs`, `burn_device.rs` |
| D | **La précision mixte en f16** : poids et matmuls en f16, LayerNorm, softmax et pooling en f32. `HalfPrecisionAdapter` de burn-store convertit par type de module et exclut `LayerNorm` — mais le graphe généré par burn-import n'a que des `SubmoduleN` : il faut soit régénérer avec des types nommés, soit cibler par chemin de tenseur | `le_f16_rend_les_memes_vecteurs` : cosinus > 0,999, et le banc à ≥ 11 000 j/s | `generated/bge_m3_onnx.rs`, `burn_bge_m3_embedder.rs`, éventuellement un fork de burn-import |
| E | **cubecl-hip f16 sur gfx12** : reproduire l'erreur LLVM avec un matmul f16 minimal, ouvrir l'issue chez cubecl (ou forker : l'intrinsèque WMMA gfx12 s'écrit `v_wmma_f32_16x16x16_f16` avec une signature différente de gfx11) | le banc ROCm en f16 tourne | cubecl-hip, LLVM/HIP installé |
| F | L'attention fusionnée (flash) dans le graphe : dépend de ce que burn 0.22 offre | 900 mots ≥ 5 000 j/s en f32 | burn |

Ce qui est **déjà en place** pour ce chantier : `RAG3WEAVER_BURN_FLOAT`
(`f16` / `bf16` / `f32`) qui règle la carte et convertit les poids au
chargement (`FloatCastAdapter`) ; les sorties toujours rendues en f32 ; le
banc et le test de parité ; le tri par longueur des lots (issue 01).

## Ce que ça change pour le défaut

Aujourd'hui le défaut est Vulkan f32, le seul chemin qui rend des vecteurs
justes partout. Le chemin court vers « parfait par défaut » est A + B + C :
sans toucher au modèle, ROCm quand il existe et des formes stables, on peut
attendre 3 à 4 fois le débit actuel. D et E sont le monument, et ils valent
chacun un facteur 2 de plus.
