# Sous une vue, les commandes tournent dans le bac du backend, pas dans le worktree de la vue

- **État** : ouvert (21 novembre 2026, 4 h 10), arbre principal.
- **Famille** : produit
- **Gravité** : une réponse fausse pour l'agent qui travaille dans sa vue. Depuis le lot `ecrire-la-vue`, l'agent lit,
  cherche et écrit le worktree d'une vue ouverte par `open_view`. Mais `run` (cargo, les tests, git) tourne encore
  dans le bac à sable du backend, sur l'arbre principal. Il compile et teste donc un autre arbre que celui qu'il vient
  d'éditer.
- **Atteignable en service** : oui, dès qu'une session travaille dans une vue ouverte par l'hôte et appelle `run`.
- **Touche rag3weaver** : oui (`src/backend.rs`, `build_services` : `BAC_A_SABLE_SERVICE`, sa racine et ses jetons).
- **Pour** : arbre principal ; au mot de c7 du 21 novembre, 4 h 08.
- **Signalé par** : arbre principal, en faisant le point (1) du scénario 1 de tty (l'écriture dans une vue).

## Ce que c'est

- `build_services` monte le bac à sable des commandes sur la racine du manifeste (`workspace.root`), avec la lecture
  git du principal (`avec_lecture_git`).
- La vue en écriture (`backend::with_writable_view`) ne change que la source de fichiers de l'appel. Le bac, sa racine
  de travail, ses chemins en écriture et son cwd restent ceux du backend.
- Une tâche (`POST /ide/taches`) n'a pas ce défaut : sa surcharge de workspace déplace tout, bac compris.

## Le correctif attendu

- Sous une vue en écriture, monter le bac sur le worktree de la vue : racine, écriture permise dans ce worktree et son
  target (`~/.cache/rag3weaver-build/target-<dossier>/<lot>`), lecture git du dépôt commun (comme pour une tâche, par
  `avec_lecture_git(bac, racine_de_la_vue, principale)`).
- Le témoin, rouge d'abord : sous une vue ouverte, `run {"command": "pwd"}` (ou `git rev-parse --show-toplevel`) rend
  le chemin du worktree de la vue, et pas celui de l'arbre principal.

## Contournement

Ouvrir une tâche sur le worktree : sa surcharge porte le bac.
