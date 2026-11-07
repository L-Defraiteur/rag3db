# Le chat saute au scroll pendant que l'agent répond

- **État** : ouvert (7 novembre 2026), pour everything declarative — lot « le chat tient l'ordre ».
- **Gravité** : gêne (une page qui tremble et vole la lecture).
- **Atteignable en service** : oui, tout tour du chat en flux (IDE posé à 17 h 22).
- **Touche rag3weaver** : oui (`ui/ide/src/views/Chat.tsx`).
- **Famille** : produit

## Ce que c'est

Lucie : « la page commence à buguer, flickering de scroll au fur et à mesure que l'agent progresse
dans sa réponse ».

## La cause (lue dans le code)

À chaque changement des messages (chaque jeton), la vue force `scrollTop = scrollHeight`
(`Chat.tsx`, l'effet sur `items`) sur une section en `scroll-smooth` : une animation de défilement
repart à chaque jeton, et la position est reprise même quand la personne a remonté pour lire.

## Ce qu'il faut pour le fermer

Pendant le flux, la page ne saute pas : collée en bas, elle suit (sans animation) ; remontée, elle
reste où elle est. Témoin navigateur qui mesure `scrollTop` pendant un flux de 50 événements :
aucun saut hors du cas « collée en bas ».
