# Sous une vue, cargo écrit le target partagé du dépôt, pas celui de la vue

- **État** : ouvert (21 novembre 2026, 7 h 45), arbre principal.
- **Famille** : produit
- **Gravité** : des builds qui se gênent. Deux vues qui compilent en même temps se disputent le verrou du target
  partagé, et se rebâtissent l'une l'autre en changeant de branche. La règle des sessions (un target par session ou
  par lot) n'est pas celle que l'agent retrouve dans sa vue.
- **Atteignable en service** : oui, dès que deux vues ouvertes par l'hôte lancent `cargo` (le bac du gabarit de code
  pose `CARGO_TARGET_DIR={repo_target}`).
- **Touche rag3weaver** : oui (`src/backend.rs`, `jetons_du_bac` et `build_services` ; `src/daemon_host.rs`, la vue en
  écriture).
- **Pour** : arbre principal ; au mot de c7 du 21 novembre, 7 h 43.
- **Signalé par** : arbre principal, en faisant le lot `run-dans-la-vue` (les commandes sous une vue).

## Ce que c'est

- Depuis `run-dans-la-vue`, sous une vue en écriture, le bac Landlock est monté sur le worktree de la vue. Mais ses
  jetons (`{repo_target}`, `{task_dir}`) restent ceux du backend (`jetons_du_bac(prepared, racine_du_backend)`).
- Le gabarit de code déclare `CARGO_TARGET_DIR={repo_target}` : sous une vue, `cargo` écrit donc dans le target
  partagé du dépôt (`commande::cible_du_depot`), le même pour toutes les vues et pour l'arbre principal.
- `open_view` note pourtant un target propre à la vue : `~/.cache/rag3weaver-build/target-<dossier>/<jj-mm-aaaa>-<lot>`,
  le champ `target` de la vue dans l'état gardé, que `close_view` efface.

## Le correctif attendu

- Sous une vue en écriture, `{repo_target}` vaut le target que `open_view` a noté pour la vue (et `{task_dir}` un
  dossier propre à la vue). L'hôte le passe avec la vue en écriture, et `build_services` construit les jetons du bac
  avec lui.
- Le témoin, rouge d'abord : deux vues ouvertes, `cargo build` dans chacune (un petit crate), sans aucune attente de
  verrou (`Blocking waiting for file lock` absent des deux sorties), et chaque target sous le sien.

## Contournement

Lancer `cargo` avec `--target-dir` pointé sur le target de la vue.
