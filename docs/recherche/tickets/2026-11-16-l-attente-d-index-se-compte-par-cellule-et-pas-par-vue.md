# L'attente d'index se compte par cellule, et pas encore par vue

- **État** : ouvert (16 novembre 2026, session recherche, pour la passation à Codex).
- **Gravité** : réponse approximative. Sous une vue d'une autre couche que celle du backend, « index pas
  prêt » parle du retard de la cellule, pas de celui de la couche lue.
- **Atteignable en service** : oui, dès que des couches de travail par worktree s'indexent (2a′-3).
- **Touche rag3weaver** : oui (`src/catalog/retard.rs`, `src/backend.rs`, `wait_for_a_lagging_index`).
- **Famille** : produit
- **Pour** : recherche (ou Codex, par la passation du 16 novembre).
- **Signalé par** : recherche, au lot « index pas prêt » (master 8b7686025e, plan `docs/recherche/16-11-2026/02-plan-index-pas-pret.md`).
- **Bloqué par** : 2a′-3 d'arbre principal. Sa page dit que le reste d'une vue se compte en fichiers,
  comme l'écart entre sa pointe (HEAD et fichiers non commités) et sa dernière génération publiée.

## Ce que c'est

`Lag` (`catalog/retard.rs`) tient, **par cellule**, les fichiers changés et pas encore réindexés, et le
débit observé. `Backend::wait_for_a_lagging_index` lit l'estimation de la cellule de l'appel, quelle que
soit la `view` demandée. Sous 2b-C, une seule couche s'indexe (`backend`), et c'est donc exact. Avec 2a′-3,
chaque worktree aura sa couche de travail et sa montre : le retard doit se lire pour la couche de la vue.

## Ce qu'il faut pour le fermer

- `Lag` tient ses entrées par `(cellule, couche)`. La montre de 2a′-3 note avec la couche du worktree qui a
  changé. `estimate` et `wait_caught_up` prennent la couche.
- `wait_for_a_lagging_index` lit la couche de `CALL_VIEW` s'il y en a une, sinon celle du backend.
- Le contrat d'archi (`Lag::estimate`, `Lag::wait_caught_up`) garde sa forme à une couche près. Le lui
  dire avant le lot.
- Les témoins, rouges d'abord : deux couches, un fichier en retard dans l'une. Sous la vue de l'autre, la
  lecture ne l'attend pas ; sous la sienne, elle l'attend, ou répond « index pas prêt ».
