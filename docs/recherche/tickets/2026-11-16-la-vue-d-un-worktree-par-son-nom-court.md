# La vue d'un worktree ne se demande pas encore par son nom court

- **État** : ouvert (16 novembre 2026, session recherche, pour la passation à Codex).
- **Gravité** : confort. Aujourd'hui, une vue se nomme par l'id de sa couche ou par son nom complet.
- **Atteignable en service** : oui, dès qu'une couche de travail par worktree existe (2a′-3).
- **Touche rag3weaver** : oui (`src/backend.rs`, `parse_view` et `load_call_view_stack`).
- **Famille** : produit
- **Pour** : recherche (ou Codex, par la passation du 16 novembre).
- **Signalé par** : arbre principal, page `docs/arbre-principal/16-11-2026/01-2a-prime-3-couches-par-branche-et-par-worktree.md`, section « Le nom court d'une vue ».
- **Bloqué par** : 2a′-3 d'arbre principal (les couches de travail par worktree). Rien à faire avant.

## Ce que c'est

`view` accepte aujourd'hui `{"layer": <id ou nom>, "rank"?: n}` (2b-C, `docs/recherche/16-11-2026/01-la-vue-des-outils.md`).
La page de 2a′-3 prévoit un sucre : `view: {"worktree": "<chemin>"}`, ou le nom d'une tâche de l'atelier.
Il se résout en `worktree:<chemin réel>` dans `call_tool_on`, au même endroit que `layer`. La vue lue
reste dite par son id, sa couche et son rang.

## Ce qu'il faut pour le fermer

- `parse_view` accepte `worktree` (un chemin, canonicalisé) **ou** `layer`, jamais les deux : les deux à
  la fois, c'est un refus qui les nomme. `task` (le nom d'une tâche) suit si la page de 2a′-3 le confirme.
- Le nom complet `worktree:<chemin réel>` passe par `suivies::layer_id_by_name`, comme un nom de couche.
- Le schéma annoncé dans `tools/list` (le bloc `view` posé au chargement des outils, `backend.rs`) dit la
  nouvelle forme.
- Les témoins, rouges d'abord, dans `backend.rs` (module `tests`) :
  - un worktree réel du dépôt de test, nommé par son chemin, rend la même pile que par son id ;
  - un chemin qui n'est pas un worktree suivi est refusé avant toute lecture ;
  - `worktree` et `layer` ensemble sont refusés.
