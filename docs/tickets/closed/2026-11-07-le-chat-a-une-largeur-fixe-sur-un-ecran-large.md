# Le chat a une largeur fixe sur un écran large, et son pied écrase l'aide

- **État** : corrigé (7 novembre 2026, hooks-essai, lot « le chat tient l'ordre ») : le chat sort du cadre borné (route du chat seule), barres aux bords, colonne fluide, largeur de lecture sur les bulles, l'aide sur sa ligne ; témoin navigateur à 2 560 px
- **Gravité** : gêne (l'espace perdu, l'aide illisible).
- **Atteignable en service** : oui, tout écran plus large que le cadre (IDE posé à 17 h 22).
- **Touche rag3weaver** : oui (`ui/ide/src/main.tsx`, `ui/ide/src/views/Chat.tsx`).
- **Famille** : produit

## Ce que c'est

Lucie, avec une capture (`/tmp/captures-lucie/2026-11-07-ide-chat-largeur-fixe.png`) : « usage
inefficient de l'espace, notamment pour le texte qui se retrouve compacté horizontalement et écrase
la zone de chat, "Entrée pour …" ; mon écran est large et le chat semble à largeur fixe au lieu
d'avoir ses sidebars sur les bords et le contenu dynamiquement adapté ». Sur la capture : un vide à
gauche, la colonne des conversations qui ne porte qu'une phrase en bas, l'aide « Entrée pour
envoyer · Maj + Entrée pour une nouvelle ligne » cassée mot par mot dans une colonne d'environ
60 px, coincée par la ligne « consommé : llm.gemini-3.8-flash 47687 input_tokens, … ».

## La cause (lue dans le code)

- Le cadre borne **toute vue** : `main#vue` est en `max-w-5xl mx-auto` (`main.tsx:71`) ; le chat
  y loge sa grille (`260px | 1fr`) et ne peut pas atteindre les bords.
- Le pied du chat met sur une seule rangée flexible l'aide (`flex-1`), la consommation (sans borne,
  une ligne par compteur avec le modèle répété) et les boutons : la consommation prend la place,
  l'aide se réduit à presque rien.

## Ce qu'il faut pour le fermer

Les barres latérales collées aux bords ; la colonne du chat fluide (l'écran moins les barres), avec
une largeur de lecture maximale pour le texte des bulles seulement, pas pour le cadre ; l'aide sur
une ligne sous le champ ; la consommation sur sa ligne à elle, lisible (un total par compteur, le
modèle une fois) ; le bouton Envoyer aligné. Témoin navigateur à 2 560 px de large : la colonne du
chat occupe l'espace, l'aide tient sur une ligne, aucun élément du pied sous 200 px de large.
