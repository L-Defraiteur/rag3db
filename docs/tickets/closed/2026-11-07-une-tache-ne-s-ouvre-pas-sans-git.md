# Une tâche ne s'ouvre pas sans git

- **État** : corrigé (8 novembre 2026, branche tache-sans-git, hooks-essai), relu par everything
  declarative.
- **Famille** : produit
- **Gravité** : fonction absente. Quand ni le manifeste ni sa `workspace.root` ne sont dans un
  dépôt git, l'équipier ne peut ouvrir aucune tâche.
- **Atteignable en service** : oui, toute racine hors git.
- **Touche rag3weaver** : oui (`src/daemon_host/taches.rs` : ouverture, relecture, fusion, rejet ;
  la boîte de l'équipier).

## Ce qu'il faut pour le fermer

- La racine de la tâche est `workspace.root` résolue, ou à défaut le dossier du manifeste.
- Si cette racine est dans un dépôt git, la tâche est un worktree sur une branche (le cas actuel).
- Sinon, la tâche s'ouvre quand même. Son espace est un dossier de travail à elle, sous le dossier
  des tâches, copié par la même porte que la copie de base (reflink si possible) et compté dans la
  taille dite. Elle n'a pas de branche (`repo: null`, `branch: null`), et la boîte le dit : « sans
  git : les changements se relisent par le diff du dossier ».
- La relecture (le diff ligne à ligne de TaskReview), la fusion et le rejet ont leur chemin sans
  git : un diff de dossiers, une recopie des fichiers changés, un effacement du dossier.
- Une option `git_init: true` ou un outil initialise un dépôt dans cet espace si l'utilisatrice
  le veut, jamais d'office.

Témoin par l'hôte : une racine sans git ; la tâche s'ouvre, l'équipier lit et écrit, et la
proposition arrive.

## Le correctif

- **Sans aucun dépôt** (ni les déclarations ni la racine), la tâche s'ouvre quand même
  (`Tache.sans_depot`). Elle n'a ni worktree ni branche, et ses déclarations sont copiées comme
  pour le cas hors dépôt.
- **Quand la racine du code n'est dans aucun dépôt**, la tâche travaille dans une copie de la
  racine, `<tâche>/espace`, par `code_tree::copy` (reflink quand le disque le sait).
  `WorkspaceDeTache.copie` garde l'empreinte blake3 de chaque fichier au départ, la taille et le
  reflink. Le cas « lue telle quelle, jamais écrite » d'avant disparaît : la tâche écrit dans sa
  copie si le principal le permet (`task_writes`).
- **L'état et la boîte** : `/ide/etat` rend l'espace avec `repo: null` et `branch: null`. La
  boîte dit à l'ouverture : « tâche ouverte : sans git : les changements se relisent par le diff
  du dossier (une copie de … ) ».
- **La relecture** : les changements par différence avec les empreintes (modifiés, neufs,
  effacés), l'avant lu dans la racine, `espace: {git: false, note, octets, reflink}`, et
  `base_a_bouge` si la racine a changé depuis l'ouverture.
- **La fusion** : refusée avant tout si la racine a changé là où la tâche écrit (les fichiers
  sont nommés, rien n'est recopié). Sinon, les fichiers changés sont recopiés et les effacés
  effacés, puis la copie est effacée.
- **Le rejet** efface la copie, sans toucher la racine.
- **`workspace.git_init: true`**, déclaré au manifeste : à l'ouverture d'une tâche, une racine
  sans git devient un dépôt (`git init` et un premier commit, sous l'identité git de
  l'utilisatrice). La tâche est alors une branche `agent/<nom>`. Jamais d'office.

Témoins (par l'hôte, `daemon_host::taches::tests`), rouges sur efc4e2b38, verts sur 1a8a5c5b3 :
- `une_tache_s_ouvre_sans_git_et_se_fusionne_par_copie` : l'équipier écrit par `edit_file`, la
  relecture rend les trois changements, la fusion les recopie, aucun dépôt n'est créé ;
- `sans_git_un_rejet_laisse_la_racine_et_un_conflit_refuse_la_fusion` ;
- `git_init_declare_fait_de_la_racine_un_depot`.
