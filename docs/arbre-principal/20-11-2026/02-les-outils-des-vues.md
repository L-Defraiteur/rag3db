# Les outils des vues : ce que l'hôte offre à une session et à ses enfants

20 novembre 2026. Suite de la page 01 (même dossier), en la forme élargie que Lucie et c7 ont décidée entre 16 h 00 et
16 h 13. **Page de cadrage, pas de code.** L'API de l'hôte est à accorder avec archi avant de coder (§ 6) ; archi fait
le sous-agent qui reçoit son ensemble, G branche la fermeture au pousseur.

## 1. Les quatre outils d'une session

| outil | arguments | ce qu'il fait |
|---|---|---|
| `views` | aucun | les vues que la session connaît (ses worktrees suivis, les couches de branche, le backend), l'active et sa source |
| `use_view` | `{name}` | rend `name` active pour la session ; un nom inconnu refuse en listant les vues connues |
| `open_view` | `{lot}` | crée un worktree pour le lot, l'indexe, le souscrit à la session et le rend actif |
| `close_view` | `{name}` | retire un worktree ouvert par l'hôte, et oublie sa vue |

Les noms d'outils MCP n'ont pas d'espace : « `views open <lot>` » de la décision devient `open_view {lot}`, et
« `views close <nom>` » devient `close_view {name}`. Ce sont des outils **de l'hôte**, servis par le client MCP à côté
de ceux du backend, pas des outils d'un graphe.

**Une vue a un nom** : le backend (`backend`), une couche de branche (`refs/heads/master`), ou un worktree suivi (son
nom court, `20-11-2026-vue-de-la-session`, ou son chemin). `use_view` et `close_view` prennent l'un ou l'autre.

**Chaque réponse de lecture répète la vue lue** : `view: {layer, rank, source}` (V1, en jeu). La source est l'une de
`appel`, `déclarée`, `racines`, `défaut`. L'ordre : la vue de l'appel, puis celle que la session a déclarée
(`use_view`, `open_view`), puis un worktree suivi parmi les `roots` de MCP (le dossier de lancement et ceux d'`--add-dir`),
puis le backend.

## 2. L'identité d'une session

- **Ce qui ne marche pas** : le pid du client mince (il change à chaque relance du serveur MCP), le cwd (l'arbre de
  Lucie pour toutes nos sessions, page 01, § 2), un identifiant passé par Claude Code au serveur MCP (il n'y en a
  pas : documentation de Claude Code, `CLAUDE_CODE_CHILD_SESSION` n'y est pas posé).
- **Ce qui marche** : le client mince remonte à son ancêtre `claude` (comme le serveur `canal`) et lit
  `~/.claude/sessions/<pid>.json` : `sessionId`, `name`, `cwd`. Il les envoie à `POST /session`
  (`{client, keys, pid, claude: {session_id, name, pid}}`). À défaut du registre, la fiche du hook
  (`~/.local/state/rag3weaver/canal/<pid>.json`).
- **L'hôte garde l'état d'une session par son `session_id`**, sur disque (`~/.local/state/rag3weaver/vues/<session_id>.json`) :
  ses vues, l'active, les worktrees qu'il a ouverts pour elle, son nom court. Un client mince relancé, un second serveur
  MCP de la même session, ou un hôte redémarré la retrouvent.
- **Le nom court** donne le dossier `~/git_workspaces/<nom court>/` : il ne se déduit pas du nom de la session
  (« arbre principal » a choisi `arbre`, « optimiseur » `G`). Il se déclare une fois, au premier `open_view`
  (`{lot, session_dir}`), et l'hôte le garde ; un `session_dir` déjà pris par une autre session est refusé.

## 3. Ouvrir et fermer

**`open_view {lot}`**, sur le poste de l'hôte :

1. le chemin `~/git_workspaces/<nom court>/<jj-mm-aaaa>-<lot>`, et une branche `<nom court>/<lot>` depuis
   `origin/master` ; un chemin qui existe déjà refuse ;
2. `git worktree add`, puis `git submodule update --init` ;
3. son target déclaré, `~/.cache/rag3weaver-build/target-<nom court>/<jj-mm-aaaa>-<lot>`, rendu dans la réponse (l'hôte
   ne lance pas cargo) ;
4. sa couche de travail, au point de fourche avec master (page 01, § 3.4) : la couche d'histoire de master au rang du
   `merge-base`, et l'écart entier comme couche de travail ;
5. la vue souscrite à la session et rendue active.

Il rend `{name, path, branch, target, layer}`.

**`close_view {name}`** refuse, en le disant :

- un worktree que l'hôte n'a pas ouvert pour cette session (« ouvert ailleurs : le mot de Lucie le ferme ») ;
- un arbre sale, sous-module compris (`git status --porcelain --ignore-submodules=none`), ou une tête qu'aucune
  branche distante ne porte (rien de local ne se perd) ;
- un processus qui y travaille (cwd ou fichier ouvert sous le chemin).

Sinon il retire le worktree (`--force` après ces contrôles, à cause du sous-module), `prune`, efface son target, finit
sa couche, et retire la vue de la session et de ses enfants. **G branche la fermeture au pousseur** : à l'atterrissage
d'un lot, le pousseur appelle `close_view` pour la vue de ce lot.

**Un worktree retiré autrement** (à la main) : sa couche finit, la vue retombe sur le backend, et la réponse suivante
le dit une fois.

## 4. Les gardes, dès le cadrage

- L'hôte ne crée ni ne ferme rien dans l'arbre de Lucie (`~/git_workspaces/rag3db`), dans `sairen*`, ni dans le dossier
  d'une autre session : le chemin est refusé, nommé.
- Il ne ferme jamais un worktree qu'il n'a pas ouvert, sans le mot de Lucie.
- Il ne pousse rien et ne commite rien : ouvrir, c'est créer l'arbre et la branche locale ; la session commite et
  dépose comme aujourd'hui.

## 5. L'ensemble de vues d'un enfant

- **Un enfant** est nommé `<session>-<nom>`. Son parent lui donne, au lancement, un ensemble de vues (une, plusieurs, ou
  toutes les siennes) et l'active : `POST /views/child {child, views, active}`.
- **Le parent le modifie à tout moment** (ajouter, retirer, basculer) pour un enfant nommé.
- **L'enfant** bascule dans son ensemble (`use_view`) s'il en a plusieurs, mais ne l'élargit jamais. Un appel hors de
  son ensemble refuse en le nommant. Ce qu'il ouvre (`open_view`) appartient à la session parente, et entre dans son
  propre ensemble.
- **La session est le parent de ses propres vues** : même mécanique à deux niveaux. L'ensemble d'une session est tout
  ce qu'elle a ouvert ou souscrit, plus le backend et les couches de branche.
- **Une limite à dire à archi** : un sous-agent de Claude Code (l'outil Agent) appelle par le serveur MCP de son parent.
  L'hôte ne peut pas distinguer ses appels de ceux du parent : rien dans une requête MCP ne dit quel agent l'émet.
  L'ensemble d'un enfant ne vaut donc que pour un enfant qui ouvre **sa propre session d'hôte** : une tâche de
  rag3weaver, ou un enfant lancé avec son client à lui. C'est le sous-agent qu'archi fait.

## 6. L'API de l'hôte, à accorder avec archi

| route | corps | rend |
|---|---|---|
| `POST /session` | `{client, keys, pid, claude?: {session_id, name, pid}, child?: {parent_session_id, name}}` | `{session}` |
| `GET /views` | — | `{views: [{name, layer, kind, path?}], active: {name, source}}` |
| `POST /views/use` | `{name}` | `{active}` |
| `POST /views/open` | `{lot, session_dir?}` | `{name, path, branch, target, layer}` |
| `POST /views/close` | `{name}` | `{closed: name}` |
| `POST /views/child` | `{child, views, active}` | `{child, views, active}` |

Toutes prennent la session par `X-Rag3weaver-Session`. Une session d'enfant n'a pas `/views/child`, et `/views/open`
y range la vue chez le parent.

## 7. Les lots, dans l'ordre de c7

1. **V1** (en jeu) : la vue de la session et la source dite.
2. **`views` et `use_view`** (1 jour d'agent) : l'identité par `session_id`, l'état sur disque, les `roots` en
   troisième source. Témoin rouge : deux clients minces du même `session_id` voient la même vue active, un nom inconnu
   refuse en listant, un hôte relancé la retrouve.
3. **`open_view` et `close_view`, avec la couche de travail au point de fourche** (1 ½ jour). Témoin rouge : `open_view`
   crée l'arbre sous le dossier de la session, sa vue lit un commit du lot et une modification non commitée, la vue du
   backend n'en lit aucune ; `close_view` refuse un arbre sale, l'arbre de Lucie, un worktree ouvert ailleurs, puis
   retire un arbre propre et son target.
4. **L'ensemble des enfants** (1 jour). Témoin rouge : un enfant à deux vues bascule entre elles, refuse une troisième
   en la nommant, et voit l'ajout de son parent sans relance.

**Total** : 3 ½ jours d'agent après V1.
