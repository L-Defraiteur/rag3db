# Déplier un outil dans le chat déplace le scroll

- **État** : corrigé (7 novembre 2026, hooks-essai, lot « le chat tient l'ordre ») : déplier ou replier un outil ne recolle jamais la liste (seul un événement du flux le demande, et seulement collée en bas) ; témoin navigateur (`scrollTop` identique avant et après)
- **Gravité** : gêne.
- **Atteignable en service** : oui (IDE posé à 17 h 22).
- **Touche rag3weaver** : oui (`ui/ide/src/views/Chat.tsx`).
- **Famille** : produit

## Ce que c'est

Lucie : « le scroll ne doit pas buguer quand j'expand un outil ».

## La cause (lue dans le code)

Déplier un `<details>` change la hauteur du fil ; l'effet de la vue qui recolle en bas
(`scrollTop = scrollHeight` sur chaque changement) et le `scroll-smooth` de la section déplacent
la page sous l'élément qu'on vient d'ouvrir.

## Ce qu'il faut pour le fermer

Déplier ou replier un outil ne déplace pas la page : l'élément déplié reste sous le pointeur
(ancrage sur lui, et aucun recollage en bas qui ne vienne pas d'un nouvel événement du flux).
Témoin navigateur : `scrollTop` identique avant et après l'expansion quand l'élément est visible.
