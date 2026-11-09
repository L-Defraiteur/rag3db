# L'atelier lit et écrit : la forme proposée

Session archi, 9 novembre 2026. C'est le lot que Lucie a décidé le 8 au soir, et que c7 m'a confié
le 9 au matin. Cette page dit ce qui existe (relevé sur `origin/master` 6036371de), la forme que je
propose, et les questions à trancher avant tout code. everything declarative tient la plume de la
page de conception de l'atelier ; c7 tranche les désaccords de forme.

## À essayer dans ton atelier (sur master `d23af9cd2`)

- **Rien à reposer à la main.** Ton manifeste a été noté à sa pose (9 h 37) et n'a pas changé depuis. Dès que l'hôte tourne sur `d23af9cd2`, c'est-à-dire après la pose par G, il prend seul les outils neufs à la réouverture de l'atelier, et ta racine `/home/lucied` est gardée.
- **Lire puis écrire.** Demande-lui de lire un fichier (`read_file`, 200 lignes à la fois) ou une fonction d'un dépôt attaché (`read_scope`), puis de changer quelques lignes. Il rend le diff, réindexe le dépôt, et refuse d'écrire des lignes qu'il n'a pas relues ce tour-ci ou le précédent.
- **Hors de la racine.** Demande-lui un fichier hors de ta racine : une carte d'accord glisse sous l'appel. Tu accordes ou tu refuses, et la décision s'écrit dans `decisions.jsonl`.

## Ce que Lucie a décidé

1. **Lire** : `read_file {path, offset, length}`, avec 200 lignes par défaut et jamais plus d'un
   coup. `read_scope` rend un scope de l'index, par son nom ou son chemin : son texte et ses bornes.
2. **Écrire** :
   - `replace_text` prend un ancien texte unique dans le fichier ; sinon c'est un refus qui dit le
     nombre d'occurrences.
   - `replace_lines {path, from, to, text}` et `insert_lines {path, at, text}` exigent une empreinte
     de lecture récente : ces lignes lues dans ce tour ou le précédent, sinon « relis d'abord ».
   - Chaque écriture rend le diff unifié, réindexe le scope touché et s'écrit au journal du run.
3. **Périmètre** : « il doit rester capable de tout faire ».
   - La racine est le dossier où l'atelier a été démarré ; par défaut `~/rw-code/`, créé s'il
     manque.
   - Un worktree attaché est détecté seul.
   - Hors de la racine, le modèle a deux voies : déclarer un espace (attach ou open d'un worktree),
     ou demander la permission par la porte d'approbation.
   - Approbations et refus se journalisent, avec le chemin et la raison : c'est la matière du futur
     décideur local.
4. Pas de Landlock dans ce lot.

## Ce qui existe

| sujet | où | ce qu'on y trouve |
|---|---|---|
| outils de l'atelier | `templates/backends/atelier/backend.json` | attach_worktree, list_worktrees, open_branch, run_command, show_progress ; `workspace {root: ".", commands: approval, commands_outside_tasks: true}` ; aucun schéma d'index |
| lecture et écriture du gabarit de code | `ReadFileNode`, `EditFileNode` (`dataflow/code_nodes.rs`) ; `read_file_with`, `write_edit` (`code_tools.rs`) | lecture par fenêtre (offset 1-based, limit 200, plafond 2000) avec `scopes` et `content_hash` ; écriture par `old`/`new` (une seule occurrence) ou par contenu entier, avec `expected_hash` ; réindexation par `code_sync::reingest_file` ; **pas de diff** |
| chemins | `WorkingTree` + `RootPolicy` (`FILE_ACCESS_SERVICE`) | tout reste sous `workspace.root` ; l'extérieur est fermé par défaut, sauf une liste de racines permises |
| chat → outils | `chat_turn_with` → `OutilsDuChat` → `DeclarationTools` → `BackendTools` → `Backend::call` | **aucun outil ne voit la session** : l'invocation ne porte ni le tour ni l'historique ; `OutilsDuChat` filtre déjà les outils (run_commands, write_files) |
| porte des commandes | `Garde::juger_ligne` (`commande.rs`), `RunCommandNode` | « Commande en attente » ; l'accord n'existe que dans le **pas d'un équipier** (boîte : ligne `approval`, réponse `approved`) ; dans le chat de l'atelier, le texte revient tel quel au modèle, et l'utilisateur n'a rien à accorder |
| texte d'un scope | entité `Scope`, champ `content` (`code.rs`) | aucun nœud ni outil ne le rend par nom ou par chemin |
| racine de l'atelier | `atelier_cli` (`bin/rag3weaver-backend.rs`) | `workspace.root` réécrit à **`$HOME`** ; le dossier de lancement n'est noté nulle part |
| journal du tour | `run_chat_turn`, `JournalDesRuns::ajouter` | `tours`, `fichiers`, `propositions` ajoutés au run du chat |

## La forme proposée

**Les outils sont des graphes, et les règles du tour vivent dans la couche du chat.**

- **`read_file`** : `ReadFileNode`, tel quel. La longueur est plafonnée à 200 dans le gabarit de
  l'atelier (la valeur par défaut et le plafond).
- **`read_scope`** : un nœud neuf, `ReadScopeNode(name, path)`. Il lit `Scope` par nom (avec
  `path` pour lever une ambiguïté), et rend le texte, `start_line` et `end_line`. Il dit pourquoi
  quand le dépôt n'a pas d'index.
- **`replace_text`** : `EditFileNode` en mode `old`/`new`. Le refus dit déjà le nombre
  d'occurrences.
- **`replace_lines` et `insert_lines`** : `EditFileNode` reçoit deux opérations neuves,
  `EditOp::Lines{from, to, text}` et `EditOp::Insert{at, text}`.
- **Le diff unifié** : `EditResult` gagne `diff`, calculé entre l'avant et l'après du seul fichier,
  et rendu en tête du texte. La réindexation existe déjà.
- **L'empreinte de lecture** : la couche `OutilsDuChat` voit passer les `read_file` et `read_scope`.
  Elle note, par conversation, chemin → plages lues → numéro du tour. Avant un `replace_lines` ou un
  `insert_lines`, elle vérifie que la plage visée a été lue dans ce tour ou le précédent, sinon elle
  refuse avec « relis d'abord les lignes N-M de <chemin> ». Une écriture déplace les lignes : les
  plages lues du fichier sont alors oubliées, et il faut relire.
- **La racine** : le gabarit de l'atelier garde `workspace.root`. `atelier_cli` y écrit le dossier
  courant, sauf quand on le lance depuis `$HOME` ou `/` ; dans ce cas il prend `~/rw-code`, créé
  s'il manque.
- **Le worktree attaché** : un chemin qui tombe dans un worktree attaché est lu et écrit par le
  backend de ce worktree, ce qui le réindexe. Il n'est pas traité par la racine de l'atelier.
- **Hors de la racine** : refus, avec deux suites dites au modèle.
  - (a) Déclarer l'espace par `attach_worktree` ou `open_branch`.
  - (b) Demander l'accord par un outil `ask_permission {path, mode: read|write, reason}`. La page
    rend une carte d'accord sous l'appel, par le registre `ui`, avec « accorder » et « refuser ».
    La réponse de l'utilisateur ouvre ce chemin à la `RootPolicy` de la conversation, ou le refuse.
  - Chaque décision s'écrit au journal du run (chemin, mode, raison, décision, qui, quand), et la
    boîte n'est pas employée.

## Les questions à trancher

1. **La porte** : une carte d'accord dans le chat (registre `ui`) plutôt que la boîte de l'équipier,
   qui suppose une tâche ? Faut-il un journal à part des décisions, pour le futur décideur, ou le
   journal du run suffit-il ?
2. **L'empreinte** : est-ce bien la couche `OutilsDuChat` qui la tient (en mémoire, par
   conversation, et perdue au redémarrage de l'hôte, ce qui fait relire) ?
3. **La racine** : le dossier courant, sauf `$HOME` et `/`, qui donnent `~/rw-code` ? Et
   l'atelier déjà posé (`root` vaut `$HOME`) : le réécrire à la prochaine commande `atelier`, ou le
   laisser ?
4. **Les noms** : `read_file` comme au gabarit de code, et des noms neufs pour les écritures. Faut-il
   retirer `edit_file`, que l'atelier n'a pas, de la liste de l'atelier ?

## Les témoins (rouges d'abord)

Un e2e par outil, sous le jeu large, avec une garde de capacité :
- lire au-delà de 200 lignes rend 200 et « la suite » ;
- `read_scope` d'une fonction connue rend son texte et ses bornes, et sans index il dit pourquoi ;
- `replace_text` avec deux occurrences est refusé et dit « 2 occurrences » ;
- `replace_lines` sans lecture est refusé, et réussit après lecture, avec le diff et la réindexation
  (`usages` voit la ligne neuve) ;
- `insert_lines` avec une lecture vieille de deux tours est refusé ;
- un chemin hors de la racine est refusé avec les deux suites ; `ask_permission` accordé ouvre le
  chemin, refusé le garde fermé, et les deux se journalisent.

## Ce qui est fait (9 novembre 2026, branche atelier-lire-ecrire)

Les décisions de c7 sur les quatre questions : une carte d'accord dans le chat, et la boîte reste
à l'équipier ; un journal des décisions à part ; l'empreinte gardée en mémoire ; à une pose neuve,
le dossier courant ou `~/rw-code` comme racine, l'atelier déjà posé gardant la sienne ; les noms
`read_file`, `read_scope`, `replace_text`, `replace_lines`, `insert_lines` et `ask_permission`.

En chemin, deux faits ont ajusté la forme :
- **Les outils passent par l'hôte** (`LireFichierNode` et `EcrireFichierNode`, par le service des
  worktrees). L'atelier n'a pas d'index : un chemin pris dans un worktree attaché est servi par le
  backend de code de ce worktree (son `read_file`, `outline`, `usages`, et son `edit_file`, qui
  réindexe) ; un autre chemin, sous la racine de l'atelier, se lit et s'écrit sans index.
- **`workspace.writes_outside_tasks`** (vrai dans l'atelier seul) : hors d'une tâche, le chat
  ne gardait aucun outil d'écriture.

**Qui écrit un worktree attaché**, puisque tout arbre inscrit porte `task_writes` (décision de
c7) :
- un worktree **né de l'atelier**, créé par `open_branch` et marqué à côté de son manifeste
  (`ouvert-par-l-atelier`), s'écrit sans question, au nom de l'utilisatrice ;
- tout autre arbre passe par `ask_permission`. La marque n'est pas dans le manifeste : l'écrire
  sur un dépôt ouvert le ferait relire, ce qui couperait son index.

| outil | ce qu'il fait |
|---|---|
| `read_file` | 200 lignes au plus, la suite dite ; lu par son dépôt dans un worktree attaché |
| `read_scope` | par `usages` (où) et `outline` (ses bornes) ; un nom inconnu, ambigu (les définitions listées) ou un fichier sans index sont refusés en disant pourquoi |
| `replace_text` | un ancien texte unique, sinon « N occurrences » ; le diff unifié |
| `replace_lines`, `insert_lines` | leurs bornes vérifiées ; l'empreinte de lecture (dans `OutilsDuChat`) exige d'avoir lu ces lignes dans ce tour ou le précédent ; une écriture oublie les lectures du fichier |
| `ask_permission` | une carte d'accord sous l'appel (registre `ui`, kind `accord`) ; `/ide/accords` décide ; un oui ouvre ce chemin pour ce mode (`write` vaut aussi lecture), et chaque décision s'ajoute à `<racine des runs>/decisions.jsonl` (chemin, mode, raison, décision, qui, quand) |

Témoins, tous vus rouges d'abord : sept par le chemin de l'hôte (dans `daemon_host::atelier_fichiers`)
et un au navigateur (la carte d'accord), plus la règle de la racine (`code_tree`) et
l'empreinte (`empreintes`).

**Lot suivant** : les commandes en attente de `run_command` sur la même carte, accordées par la
`Garde` de l'atelier, avec le même journal.

