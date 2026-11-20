# Un premier commit depuis l'atelier demande cinq accords, dont un en double

- **État** : ouvert (21 novembre 2026, session recherche, scénario 2 de la nuit).
- **Gravité** : confort. Le chemin aboutit, mais lentement et avec du bruit.
- **Atteignable en service** : oui.
- **Touche rag3weaver** : oui (les accords de `run_command`, le prompt de l'atelier).
- **Famille** : produit
- **Pour** : recherche (suivi ; toute réattribution passe par l'orchestration).
- **Signalé par** : recherche.

## Le pas exact

Pour créer un projet et son premier commit : une carte pour `cargo init --lib`, une pour `git init`, une pour
`git add .`, une pour `git commit`, puis deux pour `git config user.*` (voir le ticket de l'identité). Chaque carte
accordée ajoute un message « (carte d'accord) … relance-la telle quelle » et un tour de l'agent. La carte de
`git add .` paraît deux fois, dans deux messages successifs, toutes deux actives. Et l'agent a écrit `Cargo.toml` à la
main pendant que `cargo init` attendait son accord : accordée ensuite, la commande ne pouvait plus réussir.

## Le contournement

Accorder les cartes une à une, la plus récente d'abord.

## Pour le fermer

- Une carte qui accorde une suite de commandes déclarée d'avance (un plan : `git init`, `git add`, `git commit`).
- Une carte déjà accordée ou remplacée ne reste pas active.
- L'agent n'écrit pas ce qu'une commande en attente d'accord va créer.
