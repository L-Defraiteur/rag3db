# Les outils changent de place dans le chat quand la réponse du modèle arrive

- **État** : ouvert (7 novembre 2026), pour everything declarative — lot « le chat tient l'ordre ».
- **Gravité** : réponse fausse à l'écran (l'ordre montré n'est pas l'ordre des faits).
- **Atteignable en service** : oui, tout tour du chat qui appelle des outils (IDE posé à 17 h 22).
- **Touche rag3weaver** : oui (`ui/ide/src/views/Chat.tsx`).

## Ce que c'est

Lucie, sur l'IDE posé à 17 h 22 : « les outils changent de place dans le chat quand on atteint la
réponse du modèle, ils doivent rester dans l'ordre où ils sont arrivés ».

## La cause (lue dans le code)

Une bulle d'agent rend **son texte, puis ses outils** (`Chat.tsx`, l'article du message : `.body`
puis `it.tools.map(…)`). Pendant le tour, les outils arrivent d'abord (`ToolCallStarted`) et se
placent sous un texte vide ; quand les jetons de la réponse arrivent, le texte grandit **au-dessus**
d'eux et les pousse vers le bas. Relu ensuite (`fromTurns`), le même tour devient une bulle par tour
d'assistant (les appels d'abord, la réponse ensuite) : une troisième disposition.

## Ce qu'il faut pour le fermer

Une bulle d'agent tient ses parties **dans l'ordre d'arrivée** (texte, outil, texte…), pendant le
flux comme à la relecture (`fromTurns` fond les tours d'assistant d'une même réponse dans une bulle,
dans l'ordre des tours) ; rien ne se réordonne quand la réponse arrive. Témoin par le chemin de
l'hôte : un tour avec deux outils puis la réponse ; l'ordre rendu est l'ordre d'arrivée avant et
après la réponse, et après un rechargement.
