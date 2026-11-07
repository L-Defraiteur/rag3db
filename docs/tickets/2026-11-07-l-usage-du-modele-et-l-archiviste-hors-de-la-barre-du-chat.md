# L'usage du modèle et l'archiviste ne sont pas dans la barre du chat, et le chat ne prend pas la hauteur

- **État** : ouvert (7 novembre 2026), pour everything declarative — lot « le chat tient l'ordre ».
- **Gravité** : gêne (ce qui compte pour suivre un agent n'est pas visible, ou mal placé).
- **Atteignable en service** : oui (IDE posé à 17 h 22).
- **Touche rag3weaver** : oui (`ui/ide/src/views/Chat.tsx`, l'état du chat de l'hôte).

## Ce que c'est

Lucie : « l'usage modèle devrait être en sidebar avec barre de progression sur le contexte max qui
est config ; les tâches d'archiviste devraient aussi y apparaître succinctement ; le chat doit
prendre si possible toute la hauteur ».

## Ce qui est là

- La consommation est une ligne de texte dans le pied du chat (`#chat-conso`), un compteur par
  ligne avec le modèle répété ; aucune jauge.
- Le maximum existe : le trait `Llm` rend `context_len()` (`src/llm.rs:952`), réglé par
  `models.llm.context_tokens` au manifeste (`src/model_source.rs:166`) ; l'état du chat
  (`GET /ide/chat`) ne le rend pas.
- Les runs de l'archiviste (L1, compaction, archive M4) passent sur le flux, la page du chat ne les
  montre pas.
- La hauteur : la grille du chat est en `h-[calc(100dvh-3rem)]` à l'intérieur d'un cadre paddé ;
  la liste des messages et la page ont chacune leur défilement.

## Ce qu'il faut pour le fermer

1. La consommation quitte le pied pour la barre latérale : une jauge du contexte utilisé sur le
   maximum configuré (`context_len()` du modèle déclaré, rendu par l'état du chat, jamais codé en
   dur), les compteurs (entrée, cache, sortie, raisonnement) en petit, le modèle nommé une fois.
2. Les tâches de l'archiviste en une ligne chacune dans la même barre : état et heure.
3. Le chat prend toute la hauteur disponible : la liste s'étire, le champ reste en bas, un seul
   défilement.
Témoins navigateur : la jauge lit la bonne valeur après un tour (le compteur = l'usage du journal,
le maximum = le manifeste) ; une tâche d'archiviste publiée apparaît dans la barre ; la hauteur du
chat = la fenêtre moins l'en-tête, à 1 080 et 1 440 px.
