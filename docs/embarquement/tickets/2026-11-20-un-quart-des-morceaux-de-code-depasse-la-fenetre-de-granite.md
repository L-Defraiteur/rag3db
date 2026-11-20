# Un quart des morceaux de code dépasse la fenêtre de granite

- **État** : ouvert (20 novembre 2026, embarquement ; point C de la page des bornes)
- **Pour** : la session qui découpe le code (`default_scope_chunking`, `src/code.rs`) ; embarquement pour la mesure
- **Famille** : produit
- **Gravité** : haute pour la recherche par le vecteur. Sous granite-278m, la fin d'un morceau sur quatre n'est pas
  cherchable par le vecteur. Elle est coupée à 512 jetons par le modèle ; la ligne reste entière en base.
- **Atteignable en service** : oui, tout index de code embarqué par granite (le 7878 de luciepc)
- **Touche rag3weaver** : oui (`src/code.rs`, `default_scope_chunking` ; `src/chunker.rs`)

## Ce que c'est

Le code se découpe par lignes : 30 lignes ou 1 500 caractères (`default_scope_chunking`, `src/code.rs:223`). Granite
tronque à 512 jetons (`MAX_SEQ_LEN`). Mesuré le 20 novembre avec le tokenizer de granite, sur les 311 fichiers `.rs` de
`extension/rag3weaver` (master du 20 novembre), découpés comme le fait le crate :

| `max_size` | Morceaux | Au-dessus de 512 jetons |
|---|---|---|
| 1 500 (aujourd'hui) | 8 843 | **2 241 (25,3 %)** ; médiane 448, p95 587, max 742 jetons |
| 1 300 | 9 601 | 546 (5,7 %) |
| 1 200 | 10 190 | 74 (0,7 %) |
| 1 100 | 10 968 | 6 (0,1 %) |
| 1 000 | 11 991 | 1 (0,0 %) |

Ce Rust, commenté en français, compte environ 0,45 jeton par caractère : 1 500 caractères en font jusqu'à 740. Le
compteur du 7878 confirme l'ordre de grandeur : `truncated_texts` vaut **146 463** le 20 novembre à 1 h 31, depuis son
démarrage du 19 à 10 h 47, chauffe comprise (169). C'est surtout le repassage de la bascule du soir. Le service ne
compte pas le total des textes qu'il embarque.

La mesure ne compte pas l'en-tête que l'index ajoute peut-être au texte d'un morceau : la réalité est au moins aussi
haute.

## Ce qu'il faudrait

- **Une taille de morceau à la mesure du modèle**, pas une constante : bge-m3 a 8 192 jetons de fenêtre, granite 512.
  Le plus simple serait `max_size` en caractères déduit de la fenêtre du modèle déclaré (`max_tokens_per_text` de
  l'identité du service, lot `token-cap`), à environ 2,2 caractères par jeton pour le code, avec une marge. Le plus
  juste : un découpage compté en jetons (`text-splitter` sait dimensionner par un tokenizer).
- **D'ici là, pour granite** : 1 100 caractères (0,1 % au-dessus de 512), au prix de 24 % de morceaux en plus.
- **La preuve** : le compteur `truncated_texts` du service, et la ligne du journal d'index qui nomme les morceaux coupés
  (lot `truncation-relay`, en file), sur un index de rag3db avant et après.

Le script de mesure est dans la page des bornes (`docs/embarquement/18-11-2026/02-…`, point C).

## Taille

À voir par la session qui découpe ; la mesure se rejoue en une minute.
