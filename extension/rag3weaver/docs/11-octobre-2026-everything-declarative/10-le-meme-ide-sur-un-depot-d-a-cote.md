# Le même IDE sur un dépôt d'à côté : l'agent y édite du code dans une tâche

19 octobre 2026, everything declarative, demande de c7 : la marche 3 de la vision. Il s'agit de
faire tourner le même IDE sur un dépôt qui n'est pas rag3weaver. `code-tree add` inscrit un dépôt
quelconque, l'explorateur s'ouvre dessus, puis l'agent l'édite dans une tâche. Ici il édite du
code, pas des gabarits, avec la relecture et la fusion de la page 09.

## Ce qui manquait

- `code-tree add` posait l'arbre en `read_only` et retirait `edit_file`. Personne ne pouvait
  écrire le dépôt, ni l'agent ni l'humain, et une tâche n'y aurait rien changé.
- Une tâche branchait le dépôt **du backend** seulement. Le code d'un arbre inscrit est dans un
  autre dépôt, celui de l'utilisateur. Ses déclarations, sous `~/.config/rag3weaver/<nom>`,
  n'étaient dans aucun dépôt.
- Un trou de T1 : une tâche gardait la racine du workspace du principal. Sur un backend dont le
  code est dans son propre dépôt, une édition de l'agent dans la tâche aurait écrit l'arbre
  principal.

## Décisions (acceptées par c7, avec ses deux précisions)

1. **`task_writes`**, une clé du workspace, dit que l'agent n'écrit pas l'arbre principal. Il
   écrit dans une tâche, sur sa branche, relue puis fusionnée.
   - **Elle ne vise que l'agent** (c7). Lucie édite le principal dans l'IDE (B1 de G), et cela
     continue de marcher.
   - Le refus est dans `Backend::call`, le chemin des appels de l'agent, y compris l'agent dans
     l'hôte. L'IDE appelle `call_humain` : l'hôte reconnaît sa session et pose un drapeau le temps
     de l'appel. Le refus dit quoi faire : « ouvrez une tâche (POST /ide/taches) ».
   - `read_only` garde son sens, personne n'écrit. Avec `task_writes`, un outil d'écriture est
     permis au chargement.
2. **`code-tree add` pose `task_writes`**, lève `read_only` et garde `edit_file`. `run_command`
   reste retiré, la porte des commandes reste fermée.
3. **Les déclarations d'un arbre inscrit sont dans un dépôt git**, à `<config>/<nom>`. C'est la
   racine qui contient ses gabarits copiés, pas le dossier du manifeste : les chemins relatifs de
   ses outils restent dans le dépôt. `declare` en aurait créé un au dossier du manifeste, qui ne
   contient pas `templates/`.
   - L'identité est celle du dépôt de code (sa configuration locale). Sinon c'est l'identité git
     de l'utilisateur (sa configuration globale). S'il n'y en a aucune, l'ajout est refusé en le
     disant, avant que rien ne soit posé (c7 : jamais une identité inventée).
4. **Une tâche branche aussi le dépôt du code** quand il est à part : un second worktree
   (`<tâche>/code`), sur une branche du même nom.
   - La racine du workspace de la tâche est dans son worktree. C'est une surcharge,
     `Surcharges.workspace`, comme la base et le mode des déclarations, et le manifeste n'est
     pas réécrit.
   - Si le code est dans le dépôt du backend, la racine est dans le même worktree.
   - Si la racine n'est dans aucun dépôt, la tâche la lit et n'y écrit jamais.
   - Une branche `agent/<nom>` qui existe déjà dans l'un des dépôts refuse la tâche avant que
     rien ne soit créé : la défaite d'une étape ne peut plus effacer la branche de quelqu'un.
5. **La relecture** montre ce que porte chaque worktree contre son commit de départ, commité ou
   non. Les fichiers neufs que `.gitignore` laisse en font partie : c'est ce que la fusion
   amènera.
   - Chaque fichier a son genre : `declarations` (relatif au dossier du backend), `code`
     (relatif à la racine du workspace) ou `depot` (un autre fichier du dépôt). Le préfixe le
     plus long l'emporte.
   - Les graphes ne se calculent que sur les déclarations.
6. **La fusion**, dans chaque dépôt de la tâche :
   1. elle commite les éditions de l'agent avec le message donné (par défaut, « la tâche
      <nom> ») ;
   2. elle rebase si la base a bougé ;
   3. elle valide les déclarations sur le principal par recouvrement ;
   4. elle vérifie que les arbres principaux n'ont rien de non commité ;
   5. elle fusionne en avance rapide, le code d'abord, puis les déclarations.

   La montre du principal réindexe le code fusionné. Le registre garde le commit de chacun
   (`commit`, `commit_code`) et le message.

## Le témoin

`l_agent_edite_un_depot_d_a_cote_dans_une_tache_depuis_l_ide` (`daemon_host/ide_sur_un_depot.rs`) :

1. Le test crée un petit dépôt git (`src/lib.rs`, `README.md`) et l'inscrit par `code-tree add`.
   Ses déclarations ont leur dépôt et leur premier commit.
2. L'index trouve `bonjour`, et l'explorateur de l'IDE montre `src/` et `README.md`, puis le
   fichier.
3. L'agent, par une session à lui comme un client MCP, qui édite le principal est refusé avec
   « ouvrez une tâche ». Le fichier est intact.
4. Une tâche s'ouvre par le formulaire ; une indexation en cours la refuse, et le témoin reclique
   tant que la page le dit. L'agent y édite `src/lib.rs` : le worktree change, le dépôt de
   l'utilisateur non, et l'explorateur de la tâche montre le neuf.
5. La relecture montre `src/lib.rs` en genre `code`, avec sa différence, et la branche qui se
   charge.
6. Fusionner, avec « dire salut », amène un commit unique en avance rapide dans le dépôt de
   l'utilisateur. Les branches de la tâche sont effacées dans les deux dépôts, et l'explorateur
   du principal montre le neuf.
7. L'humain édite `README.md` du principal dans CodeMirror (Modifier, taper, Ctrl-S), et le
   disque porte son texte. La console du navigateur est vide.
