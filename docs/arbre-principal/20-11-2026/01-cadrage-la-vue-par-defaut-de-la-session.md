# Cadrage : la vue par défaut de la session qui appelle

20 novembre 2026, à la demande de c7. **Page de cadrage, pas de code.** Lucie et c7 décident ensuite qui le code.
Elle prolonge le ticket `arbre-principal/tickets/2026-11-16-les-vues-des-worktrees-des-sessions.md` (destiné à Codex)
et le reste dit du ticket fermé `closed/2026-10-11-l-index-ne-suit-pas-l-arbre-edite.md` : **qu'un agent qui appelle
sans `view` lise par défaut le worktree de sa session, et non le backend.** Treize sessions cherchent chaque jour dans
un arbre qui n'est pas le leur.

État des lieux relevé sur `origin/master` 213f3d00cc. Les lignes citées sont celles de ce commit.

## 1. Ce qui existe

- **Le paramètre `view`** des outils de lecture (2b-C) : `{layer | worktree, rank?}` (`backend.rs`, `parse_view`
  ~1172-1198 ; le sucre `worktree` donne `worktree:<chemin canonique>`). La pile est chargée par appel
  (`load_call_view_stack` ~1028), une couche finie est refusée, la réponse dit `view`.
- **Sans `view`**, on lit la couche du backend, et la réponse ne dit **pas** quelle vue a été lue
  (témoin `backend.rs` ~9678 : pas de champ `view`).
- **Les couches de travail** (2a′-3) : `worktree:<chemin>` → un id stable (`suivies.rs`, `identifiants_des_couches`
  ~775-830), écrites par `write_work_generation` (`history_layers.rs:188`) et `refresh_work_layers` (plafond
  `tracked.writes.max_work_layers`, 32).
- **Les hooks de Claude Code** (`claude_code_hook.rs`) reçoivent `session_id` et `cwd`, et écrivent au `SessionStart`
  une fiche `~/.local/state/rag3weaver/canal/<pid de claude>.json` (`{session_id, cwd, claude_pid}`, `mail.rs`
  ~250-269). Le serveur MCP `canal` retrouve son ancêtre `claude` et lit cette fiche (`canal.rs`, `mail::fiche_du_pid`).

## 2. Ce qui manque, et un piège

- **L'hôte ne sait rien de l'appelant.** Le client MCP mince ouvre `POST /session {client: "mcp (pid N)", keys, pid}`
  (`daemon_client.rs` ~464-495) ; il ne transmet ni son cwd, ni l'identité de la session Claude Code, ni `roots`
  (`mcp.rs` ignore `clientInfo` et `capabilities`, et ne demande jamais `roots/list`). `Session` (`daemon_host.rs`
  ~79-96) n'a pas de vue.
- **Le piège : le cwd ne dit pas le worktree de nos sessions.** Elles sont lancées dans l'arbre de Lucie
  (`~/git_workspaces/rag3db`), puis travaillent dans `~/git_workspaces/<session>/<jj-mm-aaaa>-<lot>` **par chemins
  absolus**, sans y changer de dossier (le `cd` d'un outil Bash ne survit pas à l'appel). Le cwd du client MCP et
  celui de la fiche du hook sont donc l'arbre de Lucie. Une vue par défaut tirée du cwd lirait le backend pour tout le
  monde : exactement ce qu'il faut corriger.
- **Aucun worktree de session n'est indexé aujourd'hui.** Le manifeste en service n'a pas de `tracked` ; le défaut
  (`suivies.rs` ~89-97) ne suit que `worktree:backend` et les tâches.
- **Une couche de travail exige la couche d'histoire de sa branche au HEAD** (`write_work_generation` refuse « la
  couche {branche} n'a pas de génération au HEAD »). Les branches des sessions (`arbre-principal/…`, `cx-…`) ne sont
  pas suivies, et en suivre une par lot coûterait un rattrapage d'histoire par branche.

## 3. Ce que je propose

### 3.1 Qui dit le worktree : la session, explicitement, et l'activité en indice

- **La déclaration** : un outil `use_view` (ou un argument du `/session`) pose la vue par défaut **de la session qui
  appelle** : `{worktree: <chemin>}` ou `{layer}`. Elle vit sur la `Session` de l'hôte, et se retire par
  `use_view {}`. C'est la seule source qui ne se trompe pas pour nos sessions : elles savent où elles travaillent,
  la règle du 19 novembre leur fait déjà nommer chaque worktree.
- **L'identité stable** : le client mince remonte à son ancêtre `claude` (comme `canal`) et envoie au `/session` le
  `claude_pid` et le `session_id` de la fiche. La vue déclarée se garde par `session_id` : un client mince qui
  redémarre, ou un second serveur MCP de la même session, la retrouve.
- **L'indice, pas la décision** : les hooks `PostToolUse` voient le `file_path` des outils d'édition. Quand une session
  édite sous un worktree suivi et lit sans vue déclarée, la réponse le dit : « vue lue : backend ; tu édites dans
  `<worktree>` : `use_view {worktree: …}` ». Jamais de bascule silencieuse : une vue qui changerait au gré des éditions
  rendrait deux lectures successives incomparables.
- **`roots` de MCP** : à demander quand le client le déclare, comme troisième source, sous la déclaration. Claude Code
  n'y met que les dossiers de travail ajoutés, pas le worktree d'un lot : il ne suffit pas seul.

### 3.2 Ce que change `search_code` et ses sœurs sans `view`

- La vue lue est, dans l'ordre : `view` de l'appel, puis la vue déclarée de la session, puis le backend.
- **Chaque réponse dit la vue lue et d'où elle vient** (`view: {layer, rank, source: "appel" | "session" |
  "défaut"}`), y compris par défaut : aujourd'hui elle se tait.
- L'état de la vue (`ViewState`, avec son retard sans montre depuis aujourd'hui) suit la vue lue, pas le backend.
- Les outils qui écrivent (`index`, `reindex`) ne prennent pas la vue par défaut : une écriture nomme sa couche.

### 3.3 Hors de tout worktree, ou dans l'arbre de Lucie

- **L'arbre de Lucie** est le worktree du backend : une déclaration qui le nomme vaut le backend, sans couche de plus.
- **Un chemin hors de tout worktree suivi** : la déclaration est refusée en le nommant (« `<chemin>` n'est dans aucun
  worktree suivi ; suivis : … »), et la session reste sur sa vue d'avant.
- **Un worktree retiré** (la règle de Lucie : dans la minute de l'atterrissage) : sa couche finit ; la vue déclarée
  retombe sur le backend, et la réponse suivante le dit une fois (« la vue `<worktree>` a fini ; vue lue : backend »).
  Pas de refus à chaque appel.

### 3.4 Suivre les worktrees des sessions sans suivre leurs branches

- `tracked.sources` déclare le motif `worktree:~/git_workspaces/*/*` (le sélecteur existe, lot 6a).
- **La couche de travail d'un worktree de session prend pour parente la couche de master au point de fourche**
  (`git merge-base HEAD master`), et porte **tout l'écart** au point de fourche : les commits du lot et ce qui n'est
  pas commité. Une seule couche d'histoire suivie, celle de master, que le rattrapage (lot 8) sait écrire au rang de
  n'importe quel commit de sa ligne.
- Le plafond de 32 couches de travail se relit avec ce motif : treize sessions à deux ou trois worktrees chacune
  l'approchent ; au-delà, refus nommé, comme aujourd'hui.

## 4. Les lots, leurs témoins et leur taille

« Jour d'agent » au sens des estimations des sessions (≈ 1 h 30 réelle).

1. **V1, la vue lue est dite, et la vue par défaut de la session** (½ jour). `Session.default_view`, l'ordre appel →
   session → backend, et `view.source` dans chaque réponse.
   Témoin rouge : une session déclare `{layer: "a"}`, un `search_code` sans `view` lit `a` et le dit
   `source: "session"` ; une autre session lit le backend, `source: "défaut"`.
2. **V2, l'identité stable et l'outil `use_view`** (½ jour). Le client mince envoie `claude_pid` et `session_id` ;
   `use_view` résout un chemin en couche suivie, refuse un chemin hors suivi en le nommant.
   Témoin rouge : deux clients minces du même `session_id` partagent la vue déclarée ; un chemin hors suivi est
   refusé avec la liste des suivis.
3. **V3, les worktrees des sessions suivis par leur point de fourche** (1 jour). Le motif `~/git_workspaces/*/*`, la
   parente au `merge-base` sur master, l'écart entier comme couche de travail.
   Témoin rouge : un worktree branché sur master avec un commit et une modification non commitée ; sans suivre sa
   branche, sa vue lit les deux, et la vue du backend ne lit aucun.
4. **V4, l'indice des éditions** (½ jour, facultatif). Le hook `PostToolUse` note le worktree de la dernière édition
   par `session_id` ; une lecture sans vue déclarée le dit.
   Témoin rouge : une session édite sous un worktree suivi, lit sans vue : la réponse nomme le worktree et
   `use_view`, et la vue lue reste le backend.

**Ordre** : V1 seul vaut déjà (une session qui passe `view` une fois par lot), V2 le rend durable, V3 donne des
couches à lire. V3 peut se faire en parallèle de V1-V2. **Total : 2 jours d'agent, 2 ½ avec V4.**

## 5. Ce que je ne sais pas encore

- Si Claude Code transmet des `roots` à un serveur MCP stdio, et lesquels : à éprouver avant de les compter (§ 3.1).
- Le coût d'une couche de travail qui porte l'écart entier d'un lot de plusieurs centaines de fichiers (une
  réécriture de master suivie, par exemple) : à mesurer sur une copie au premier worktree réel de V3.
- Si l'hôte de luciepc doit suivre les worktrees des sessions de la tablette : non, chaque poste suit les siens (un hôte
  par poste) ; une session qui travaille sur l'autre poste déclare la vue de cet hôte-là.
