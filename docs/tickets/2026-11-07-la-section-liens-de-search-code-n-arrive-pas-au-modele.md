# La section « Liens » de search_code n'arrive pas au modèle du chat

- **État** : ouvert (7 novembre 2026), relevé par hooks-essai en lisant le chemin des retours
  d'outils du chat.
- **Famille** : produit
- **Gravité** : réponse incomplète. Le crochet après outil de `search_code` (« Liens », le
  graphe `links.mmd`) tourne, mais son texte n'atteint pas le modèle du chat.
- **Atteignable en service** : oui, tout `search_code` du chat de l'IDE.
- **Touche rag3weaver** : oui (`src/backend.rs`, le crochet `after` ; `src/backend_tools.rs`).

## Ce que c'est

Le crochet après outil ajoute sa section au texte quand `result` est une chaîne. Sinon, il la
range dans `response["after"] = {title, text}` (`src/backend.rs:3909-3918`). Le `result` de
`search_code` est un tableau, donc sa section va dans `after`. `BackendTools` ne prend que
`presentation` et ne lit pas `after` : la section « Liens » n'arrive pas dans le tour `tool`.

## Ce qu'il faut pour le fermer

Quand la réponse porte une `presentation` et un `after`, le texte reçu par le modèle se termine
par `### {title}` et la section. Témoin : un `search_code` du chat, et le tour `tool` contient la
section « Liens ».
