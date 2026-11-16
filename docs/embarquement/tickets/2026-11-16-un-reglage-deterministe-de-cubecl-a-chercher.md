# Un réglage déterministe de cubecl, à chercher (seulement s'il est gratuit)

- **État** : ouvert (16 novembre 2026, embarquement ; décision de c7 : « pas de correctif sauf s'il est
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
