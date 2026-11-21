# Sous une vue, les commandes tournent dans le bac du backend, pas dans le worktree de la vue

- **État** : corrigé (21 novembre 2026, 7 h 30) — branche rag3db `arbre-principal/run-dans-la-vue-2`, le bac monté sur le worktree de la vue ; témoin `run_command_under_an_opened_view_runs_in_the_views_worktree` (`src/daemon_host/invocation/tests/run_dans_la_vue.rs`). Ouvert à 4 h 10, arbre principal.
- **Famille** : produit
- **Gravité** : une réponse fausse pour l'agent qui travaille dans sa vue. Depuis le lot `ecrire-la-vue`, l'agent lit,
  cherche et écrit le worktree d'une vue ouverte par `open_view`. Mais `run` (cargo, les tests, git) tourne encore
  dans le bac à sable du backend, borné à l'arbre principal : une commande qui lit ou écrit les fichiers de la vue
  y est refusée (voir la correction ci-dessous).
- **Atteignable en service** : oui, dès qu'une session travaille dans une vue ouverte par l'hôte et appelle `run`.
- **Touche rag3weaver** : oui (`src/backend.rs`, `build_services` : `BAC_A_SABLE_SERVICE`, sa racine et ses jetons).
- **Pour** : arbre principal ; au mot de c7 du 21 novembre, 4 h 08.
- **Signalé par** : arbre principal, en faisant le point (1) du scénario 1 de tty (l'écriture dans une vue).

## Ce que c'est

- **Correction du 21 novembre, 7 h 20** : la première description disait que `run` tournait sur l'arbre principal. C'était
  faux pour son répertoire. Le répertoire d'une commande est la racine de la source de fichiers de l'appel
  (`racine_du_travail` dans `run_nodes.rs`), et celle-ci suit la vue depuis `ecrire-la-vue` : sur master, `pwd` sous la
  vue répondait déjà le worktree de la vue. Le premier témoin, sur `pwd`, est passé vert dès son commit rouge, et n'a
  pas été déposé.
- **Le vrai défaut** : le bac à sable Landlock (actif par défaut, `SandboxMode::Landlock`), que `build_services` montait
  sur la racine du backend. Une commande lancée dans la vue qui lisait ou écrivait ses fichiers (`cat`, `cargo`) se
  heurtait donc au bac : `cat src/lib.rs` sous la vue échouait.
- Une tâche (`POST /ide/taches`) n'avait pas ce défaut : sa surcharge de workspace déplace tout, bac compris.

## Le correctif

- Fait : sous une vue en écriture, `build_services` monte le bac sur le worktree de la vue (son domaine Landlock), avec la
  lecture git du dépôt commun depuis l'arbre du backend (`avec_lecture_git(bac, racine_de_la_vue, principale)`, comme
  pour une tâche). Les jetons (`{repo_target}`, `{task_dir}`) restent ceux du backend : le target de cargo est le
  target partagé du dépôt, pas celui que `open_view` note pour la vue.
- Le témoin, vu rouge sur master : sous une vue ouverte, `run_command {"command": "cat src/lib.rs"}`, un fichier changé
  dans la vue seule, réussit et le lit.

## Contournement

Ouvrir une tâche sur le worktree : sa surcharge porte le bac.
