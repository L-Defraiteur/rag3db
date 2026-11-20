# Un réglage déterministe de cubecl, à chercher (seulement s'il est gratuit)

- **État** : fermé (20 novembre 2026, embarquement, au mot de c7 : pas de réglage gratuit mesurable) ; ouvert (16 novembre 2026, embarquement ; décision de c7 : « pas de correctif sauf s'il est
  gratuit »)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : souhait
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (`src/burn_device.rs`, les embarqueurs burn), si le réglage existe

## Ce que c'est

Voir `2026-11-16-un-meme-texte-ne-rend-pas-les-memes-bits-selon-son-lot.md`. Deux sources possibles de
l'écart entre lots : le remplissage du lot (inévitable tant que les lots ont des longueurs différentes) et
le choix de noyau par l'autotune de cubecl selon la forme. Le journal du service dit « autotune oui · fusion
oui » au démarrage (`burn : DiscreteGpu(0) → … · autotune oui · fusion oui`).

## Ce qu'il faut faire

1. Lire le code de burn et de cubecl dans `~/.cargo/git/checkouts/` (les versions du `Cargo.lock`) : existe-t-il
   un réglage qui fixe l'autotune (un cache d'autotune figé, une variable comme `CUBECL_AUTOTUNE_LEVEL`) ou
   qui rend les réductions déterministes ? Noter ce qui existe, avec le chemin du code, dans ce ticket.
2. Si un réglage existe, le mesurer avec le script du ticket de tolérance
   (`2026-11-16-la-preuve-de-parite-des-services-n-a-pas-de-tolerance-mesuree.md`), sur un **service
   d'essai** lancé à part sur un port libre et sur **l'iGPU (`igpu:0`, card1)**, jamais card0, jamais à la
   place du 7878 ou du 7879 : l'écart entre lots avec et sans le réglage, et le **débit** (textes/s) avec et
   sans.
3. Gratuit = l'écart tombe à 0 entre lots de même longueur de remplissage **et** le débit ne baisse pas de
   plus de 2 %. Alors : le poser par défaut dans `src/burn_device.rs`, avec un témoin. Sinon : fermer en
   le disant, avec les chiffres.

**Ce qui ne se touche pas** : card0 (l'écran de Lucie), card2 pour un essai, les unités en place.

## Comment le prouver

Les chiffres de la mesure dans ce ticket. Si un réglage est posé : un test de lib qui charge l'embarqueur
sur `igpu:0` (garde de capacité : sauté nommé, donc **rouge**, sans iGPU) et compare deux lots de formes
différentes.

## Taille

2 à 3 heures de lecture et de mesure ; 2 heures de plus si un réglage est posé.

## Ce qui le bloque

Le ticket de tolérance d'abord (son script sert à la mesure).

## Fermé (20 novembre 2026)

- **La lecture** (cubecl bdf6b77, burn 21674205) : à l'exécution, `CUBECL_AUTOTUNE_LEVEL` (minimal, balanced,
  extensive, full), `CUBECL_AUTOTUNE_CACHE` (cache persistant) et `CUBECL_AUTOTUNE_SHORT_CIRCUIT`
  (`crates/cubecl-runtime/src/config/base.rs:139-175`). À la compilation, la feature `burn-autotune`, dont l'arrêt
  n'est pas gratuit : c'est elle qui avait fait le facteur 14 du débit.
- **Sur card2** (le 7878, par des `/embed` client) : sur un jeu **sans aucun rembourrage** (64 permutations des mêmes
  120 mots d'un jeton, tous à 122 jetons), seul contre en lot donne **4,9e-5** (cosinus 0,99999994, 0 sur 64 au bit
  près). L'écart ne vient donc pas seulement du rembourrage : c'est un ordre de réductions qui dépend de la taille du
  lot. Il est sain, et couvert par la tolérance de pose prouvée le 19 novembre.
- **Aucun réglage gratuit n'est mesurable** sans un second service sur card2, qu'on ne veut pas à côté de celui qui
  sert. L'iGPU ne peut pas servir à le mesurer : il calcule faux en lot (ticket
  `embarquement/tickets/2026-11-20-sur-l-igpu-granite-rend-des-vecteurs-faux-en-lot.md`). Sur l'iGPU, `minimal` ne
  changeait presque rien (cosinus 0,870 contre 0,870 sans rembourrage).
- **À rouvrir seulement si** une mesure de qualité montre un effet de cet écart.
