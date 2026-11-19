# granite-278m panique à sa chauffe sur l'iGPU de luciepc

- **État** : ouvert (19 novembre 2026, embarquement ; relevé en préparant la mesure de la voie courte)
- **Pour** : embarquement ; optimiseur (G) pour l'iGPU et burn
- **Famille** : produit
- **Gravité** : haute pour qui l'emploie. Un service granite-278m posé sur `igpu:0` meurt avant d'écouter. La règle
  des essais GPU (« sur `igpu:0`, card1 ») ne peut donc pas servir pour granite.
- **Atteignable en service** : oui, pour tout service granite-278m dont `RAG3WEAVER_BURN_DEVICE_EMBEDDER=igpu:0`
- **Touche rag3weaver** : à départager (burn, cubecl, la fusion ; ou le graphe de granite)

## Ce que c'est

Le 19 novembre à 14 h 4x, deux services d'essai sont lancés sur `igpu:0` à luciepc, port 7990, avec
`RAG3WEAVER_EMBED_MODEL=granite-278m` et `RAG3WEAVER_REGIME=plein`. Le premier est le binaire de master posé par G
(`91bd3ebd959ebedd`), le second celui de la branche `short-lane`. Les deux paniquent pendant la chauffe, avant d'écouter :

```
[rag3weaver] burn : IntegratedGpu(0) → Device<Vulkan(IntegratedGpu(0))> · précision Flex32 (défaut) · autotune oui · fusion oui
  chargé en 1.434110579s
thread 'main' panicked at …/burn-tensor/src/tensor/api/base.rs:3050:9:
=== Tensor Operation Error ===
  Operation: 'Expand'
  Reason:
    1. The provided tensor can't be broadcasted to the target shape. Incompatible size at dimension '0' => '382 != 182',
       which can't be broadcasted. Tensor shape Shape { dims: [1, 1, 1, 382] }, Target shape Shape { dims: [1, 1, 182, 182] }.
```

Un masque d'attention de 382 positions est élargi vers une entrée de 182 positions. La chauffe passe ses formes dans
l'ordre 40, 90, 190, 380 « mots » ; 182 et 382 sont les jetons de 90 et de 190 mots. Le masque d'une forme est donc
appliqué à une autre. Le même binaire, sur `gpu:0` (card2), chauffe ses 16 formes sans faute ; c'est le 7878 depuis le 19
au matin. Les journaux sont à luciepc : `~/.cache/rag3weaver-build/short-lane-mesure/service-avant.log` et
`service-apres.log`.

## Ce qu'il faudrait

1. Départager : l'iGPU seul (le même binaire sur `gpu:0` passe), la fusion (un bâti sans `burn-fusion` sur `igpu:0`),
   ou l'autotune (un bâti sans `burn-autotune`).
2. Le témoin : la chauffe de granite-278m sur `igpu:0` passe ses 16 formes, ou le service refuse l'iGPU par un refus
   nommé avant de charger.

## Taille

À départager d'abord (1 à 2 h).
