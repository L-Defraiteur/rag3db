# Sur l'iGPU, granite rend des vecteurs faux dès qu'il calcule en lot

- **État** : ouvert (20 novembre 2026, embarquement ; relevé en cherchant un réglage déterministe de cubecl)
- **Pour** : embarquement (la mesure) ; G (l'amont burn/cubecl, avec la fiche de la panique du 19)
- **Famille** : produit
- **Gravité** : haute pour qui pose un service sur l'iGPU. Les vecteurs sont faux, sans erreur ni panique : un texte
  embarqué dans un lot ne rend pas le vecteur qu'il rend seul (cosinus jusqu'à 0,59).
- **Atteignable en service** : oui, tout service granite-278m sur `igpu:0` (aucun en place aujourd'hui : 7878 et 7879
  sont sur card2)
- **Touche rag3weaver** : à départager (burn/cubecl sur l'adaptateur `IntegratedGpu`, Vulkan radv)

## Ce que c'est

Le 20 novembre, à luciepc, un service d'essai granite-278m sur `igpu:0`, port 7995, binaire release de la branche
`warmup-lot` (8d72040db699068c), lib 4ad394f4d2. Mesuré par `scripts/embed_parity.py` sous `poste lourd`, deux jeux :

- **Sans rembourrage** : 64 permutations des mêmes 120 mots d'un jeton, tous à 122 jetons. Un lot n'a donc aucun
  rembourrage.
- **Standard** : `tests/fixtures/embed_parity/texts.json`, 100 textes de 55 à 1 379 caractères.

| Réglage | Jeu | Seul, deux fois | Entre lots : écart max | Cosinus min |
|---|---|---|---|---|
| autotune par défaut | sans rembourrage | 0 (100 % au bit près) | **0,0765** | **0,870** |
| autotune par défaut | standard | 0 | **0,133** | **0,590** |
| `CUBECL_AUTOTUNE_LEVEL=minimal` | sans rembourrage | 0 | 0,0698 | 0,870 |
| `CUBECL_AUTOTUNE_LEVEL=minimal` | standard | 0 | 0,111 | 0,739 |

**Pour comparer**, le même modèle sur `gpu:0` (card2, le 7878, binaire b3764125), le 19 novembre : écart entre lots
5,51e-5, cosinus 0,99999993.

- **Le calcul est déterministe** (même forme : les mêmes bits), mais **faux selon la forme du lot**, même sans
  rembourrage. Ce n'est pas du bruit numérique : un cosinus de 0,87 entre deux calculs du même texte.
- **L'autotune n'en est pas la cause** : le niveau `minimal` ne change presque rien.
- **On ne sait pas encore** lequel est juste, seul ou en lot. À comparer aux vecteurs du même texte sur card2.
- **Probablement la même cause** que la panique du 19 (« Expand 382 != 182 », même adaptateur) : une forme d'un appel
  précédent reprise pour le suivant. Ici sans panique, avec des valeurs fausses.

Les journaux sont à luciepc : `~/.cache/rag3weaver-build/cubecl-mesure/` (`same-*.json`, `parity-*.json`,
`service-*.log`).

## Ce qu'il faudrait

1. **Savoir lequel est juste** : les vecteurs seuls et en lot de l'iGPU, comparés au même texte sur card2.
2. **La fiche amont** chez G, jointe à celle de la panique, avec cette mesure : le défaut est plus grave qu'une
   panique, puisqu'il rend des valeurs fausses en silence.
3. **D'ici là, chez nous** : aucun service sur l'iGPU. La règle « un essai GPU va sur `igpu:0` » ne vaut que pour des
   essais qui ne regardent pas les valeurs. Un garde-fou se discute : refuser l'adaptateur `IntegratedGpu` pour un
   embarqueur, sauf réglage explicite.
