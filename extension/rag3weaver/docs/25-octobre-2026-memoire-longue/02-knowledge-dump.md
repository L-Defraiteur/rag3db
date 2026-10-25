# Knowledge dump — mémoire longue, 25 octobre 2026

Ce qu'une session qui reprend doit savoir, et que le code ne dit pas.

## Le tour de push (luciepc)

- **Le script.** `~/.cache/rag3weaver-build/tour.sh <lot> rejeu|push`, lancé par ssh. Le
  shell de luciepc est fish : on envoie des scripts bash.
- **Le lancement.** `poste` n'est pas dans le PATH du shell distant. On lance par le
  chemin complet, dans une unité :
  `systemd-run --user --unit=<lot>-push-96 --collect [--setenv=TOUR_ARBRE=… --setenv=TOUR_TARGET=…] /home/lucied/.cache/rag3weaver-build/poste push rag3db-96 -- bash …/tour.sh <lot> push`.
  On suit ensuite l'unité par `systemctl --user is-active`.
- **Les fichiers du lot**, à côté du script :
  - `<lot>-tout.diff` : `git diff origin/master HEAD` juste après un fetch ;
  - `<lot>-suites.txt`, et facultatifs `<lot>-rouge.txt` et `<lot>-court.txt` (des suites
    en plus du rejeu court du push) ;
  - `<lot>-commit.txt` : un seul commit par tour, sans ligne d'attribution.
- **Les journaux.** Le tour écrit `<lot>-<mode>.log` lui-même. Ne pas y rediriger aussi la
  sortie du lancement : les deux s'écrasent, et des lignes se perdent.
- **Les arbres et les targets.** Principal : `rag3db-p1r-96` avec `target-96-fiches4`.
  Secondaire : `rag3db-p1-96` avec `target-96-m1`. Un seul script lourd par arbre.
- **Les planchers.** Le push juge `verifier ~/.cache/rag3db-libs/courant` depuis l'arbre.
  Il exige le code 0 **et** la dernière ligne « VERDICT : planchers tenus ».
- **Le rejeu court.** Si le crate a bougé depuis `<lot>-base-jouee.txt`, le push rejoue en
  court : features, lib entière, `e2e_code_sync`, `e2e_copy_liens`, plus `<lot>-court.txt`.

## Pièges vus aujourd'hui

- **Une dépendance de dev dans du code hors test.** `check_features.sh` ne la voit pas,
  puisque `cargo check --tests` compile la lib avec les dev-dependencies. Le bâti des e2e,
  lui, la voit.
- **L'identifiant passé à `verifier`.** Donnez le chemin du lien `courant` ou l'identifiant
  complet. `rag3db-build-58b46a9bf`, court, n'a pas de commit entier et rend 3.
- **zsh.** `$l:r` est un modificateur : écrivez `${l}`.
- **`git worktree remove`** refuse un arbre qui porte le sous-module codeparsers. Lire son
  refus avant d'annoncer l'arbre effacé.

## M3 : où sont les choses

- **`src/fichiers_entite.rs`** :
  - `Miroir::relire`, `lister`, `lire`, `champs_non_portes` ;
  - le bloc machine : `DEBUT_DU_BLOC`, `FIN_DU_BLOC`, `ligne_du_bloc`, `ecrire_le_bloc`,
    `relire_le_bloc`.
- **`src/backend.rs`** :
  - `rebuild_from_files`, `compare_files_to_base`, `verify_files` ;
  - `comparer_les_fichiers_a_l_ouverture`, appelée par `open` ;
  - le champ `ecarts_des_fichiers`, rendu par `index_states` ;
  - `Surcharges.files_root` et `sans_reactions`.
- **`src/backend_nodes.rs`** : l'action `restore` et `SERVICE_RESTAURATION`.
- **La ligne de commande** : `rag3weaver-backend files verify|rebuild`, avec l'op
  `verify_files` de l'hôte.
- **Le refus d'une base pleine.** Il compte les lignes au-delà du `seed` de chaque entité.
  Une base qui n'a que son seed passe pour vide.

## `cited_by` : ce que le repérage a trouvé

- **Rien n'est livré.** Aucun gabarit ne déclare `cited_by`, et il n'y a ni graphe ni
  `plans/cited_by.json`.
- **Ce qui existe** :
  - la fixture `ir/tests/cited_by.json` (Memory, CITE, ANCHORED_TO) ;
  - le témoin `tests/e2e_plan_declare.rs` (`plan_essai`, sur un manifeste à lui) ;
  - le Plan réduit de `src/plan_declare_tests.rs`.
- **La forme du graphe** est à la page 03 :
  `params (Rhai) → PlanNode(plan=@plans/cited_by.json) → RenderResultsNode(format=$format)`.
  - Le Rhai pose `value_dir` et `value_sym` quand `under` est vrai, et retire `since` quand
    il vaut 0 : le liage ne retire pas un entier.
  - Le rendu prend `group=false`, comme outline, pour garder l'ordre des dates.
- **La Fiche** n'a que name, about, state, author, via, revision et les dates. Les
  `returns` de la fixture ne s'y appliquent donc pas.

## Le soir : ce qui a changé dans le tour

- **La lib s'épingle au départ.** `LIB_EPINGLEE` vaut le nom résolu de `courant`, et `RAG3DB_LIB` comme le contrôle des planchers jugent cette lib-là. La bascule de 17 h 43 a eu lieu au milieu d'un rejeu.
- **`RAG3WEAVER_MOTEUR_ANCIEN=1` reste posé.** Un commit du moteur plus récent que la lib du magasin (6b63e4922) faisait refuser toutes les suites par le garde de fraîcheur de `run_e2e.sh`.
- **La lib entière se joue sous `…,postgres,openai-llm`.** Sans cette feature, 80 tests ne jouaient pas, dont celui qui a rougi master après `cited_by`.
- **`cargo clean -p rag3weaver -p rag3db`** : les liaisons se rebâtissent à chaque passe, depuis le changement d'ABI.
- **Le diff du lot se tire à trois points** (`origin/master...HEAD`), puis on lit son `--stat`.

## Les fichiers, depuis bea6c6a98

- **`format: N`** suit `uuid` dans chaque en-tête, et `format:` ouvre le YAML de chaque bloc machine.
- **Le manifeste du backend** déclare `files_format: {version, migrations: [{from, to, script}], lists: {"genres": "RefType.name"}}`. Un script de migration reçoit `input = #{entity, fields}` et rend les champs.
- **`manifeste.yaml`** à la racine des fichiers est écrit à chaque ouverture, sauf si le dossier parle d'un autre schéma, ou s'il a des fichiers sans manifeste. Dans ces deux cas, l'ouverture le dit dans `index_state.files.manifest` avec un avertissement, et `rebuild_from_files` refuse.
- **Les relations** d'un manifeste prennent une `description` d'une ligne, celle que le manifeste du dossier recopie.

## La migration de la fiche de Lucie : la recette

- **L'instantané** : `~/.cache/rag3weaver-notes-A/instantanes.sh <étiquette> <base.rag3db>` (le script de l'arbre principal). Il refuse si un processus tient un fichier de la base, copie en reflink sous `~/.local/share/rag3weaver/instantanes/<étiquette>/` et écrit `RETOUR.sh` à côté.
- **Sans arrêter son hôte** : unregister de memoire-longue par l'hôte, l'instantané, le manifeste mis au gabarit livré, puis la réouverture par l'hôte. Prévenir c7 avant l'unregister.
- **L'avant** (25 oct.) :
  - la Memory 1fb42f63 sur le rapport cyclique de 70 % : fact, inferred, person, current, revision 1, created_at 1791682063181 ;
  - le Subject « régime GPU ».
