# Le sous-agent à vues : cadrage côté hôte — le vendredi 20 novembre 2026

Décidé par Lucie et c7 le 20 novembre à 16 h 13. Une tâche de l'hôte (un équipier, en mode pair) est
**l'enfant d'une session** : elle travaille sous les vues que la session lui donne, et sous son
identité, sans jamais rien posséder. La moitié « vues » (`views`, `use_view`, `views open`,
`views close`) est celle d'arbre, qui écrit sa page. Celle-ci couvre l'hôte : la naissance de
l'enfant, son ensemble de vues, la porte qui le tient, et l'identité qu'il porte. L'API commune est
à accorder avec arbre avant tout code.

## Ce qui existe

- **La vue d'un appel** : chaque outil de code lit un argument `view`. Depuis 2b-C (fc4460b1b2), le
  `context.view` d'une invocation va là où l'appel lit sa vue.
- **La porte des appels du chat** : `OutilsDuChat` (`src/daemon_host/chat.rs`) voit chaque appel
  d'outil avant son exécution, et sait déjà refuser (`refus`, pour les outils retirés).
- **Le mode pair** (lots 1 à 4) : une tâche nommée a son entrée au registre des sessions, et elle
  reçoit et envoie des messages.

## Ce que l'hôte ajoute

1. **La naissance d'un enfant.** `POST /ide/taches` prend en plus :
   - `parent_session` : le nom de la session parente, qui donne son nom à la tâche
     (`<session>-<nom>`) et son identité ;
   - `views` : l'ensemble de vues donné, une liste de noms de vues de la session (ou `"*"` pour
     toutes) ;
   - `active` : la vue active de départ, qui doit être dans l'ensemble.

   L'ensemble est gardé au registre des tâches, à côté du nom de l'agent.
2. **Le parent modifie l'ensemble à tout moment.** `POST /ide/teammate/views` prend
   `{backend, task, add?, remove?, active?}`, au nom de la session parente seulement. La réponse rend
   l'ensemble et l'active. L'enfant le voit à son prochain appel : la porte relit l'ensemble à
   chaque appel, sans cache.
3. **La porte tient la règle, pas l'invite.** Dans `OutilsDuChat`, pour une tâche enfant :
   - un appel sans `view` prend l'active (comme `context.view`) ;
   - un appel dont la vue est hors de l'ensemble est refusé, en nommant la vue et l'ensemble ;
   - `use_view` (arbre) ne bascule qu'à l'intérieur de l'ensemble ;
   - `views open` (arbre) ouvre au nom de la session : la vue ouverte appartient à la session, et
     n'entre dans l'ensemble de l'enfant que si le parent l'y ajoute.
4. **Le domaine des fichiers** : le bac à sable de l'enfant a pour domaine
   `~/git_workspaces/<session>/`, et rien hors de ce dossier (Landlock, `sandbox`). La
   lecture-écriture de ses commandes suit l'ensemble de vues : seules les racines des vues de
   l'ensemble sont accordées.
5. **L'identité** : les commits de l'enfant portent l'identité git de la session (`GIT_AUTHOR_*` et
   `GIT_COMMITTER_*` posés dans l'environnement déclaré de ses commandes), jamais une identité à
   lui. Une attestation qu'il joue est signée au nom de la session.

## Les questions pour arbre, avant de coder

- Le nom d'une vue : un nom stable de la session (`19-11-2026-docs`), ou un identifiant de vue de
  l'index ?
- `use_view` change-t-il l'active de l'enfant à l'hôte, ou seulement la vue de son prochain appel ?
- `views open` rend-il une vue déjà dans l'ensemble du demandeur, ou faut-il toujours le parent ?
- `views close` sur une vue de l'ensemble d'un enfant : l'enfant la perd-il, et le dit-on dans sa
  boîte ?

## L'accord avec arbre (20 novembre, page `docs/arbre-principal/20-11-2026/02-les-outils-des-vues.md`)

1. **Le nom d'une vue est stable**, jamais un identifiant d'index. C'est `backend`, une couche de
   branche (`refs/heads/master`), ou `<nom court>/<jj-mm-aaaa>-<lot>` pour un worktree ouvert,
   unique sur le poste. Un chemin de worktree est accepté et ramené à son nom.
2. **`use_view` change l'active à l'hôte.** Elle est gardée au registre des tâches pour un enfant,
   et vaut pour tous ses appels suivants. Elle ne sort jamais de l'ensemble.
3. **Ce qu'un enfant ouvre appartient à la session** et n'entre dans son ensemble que si le parent
   l'y ajoute.
4. **Une vue fermée sort de l'ensemble de chaque enfant**, qui le lit dans sa boîte. Si c'était son
   active, elle retombe sur la première vue qui reste dans son ensemble, et l'enfant le lit aussi.
- Les routes de l'enfant sont celles de cette page. arbre retire son `/views/child`. Ses routes
  (`GET /views`, `POST /views/use`, `/views/open`, `/views/close`) rendent ce que dit sa page.
- **Une limite** : un sous-agent de Claude Code (l'outil Agent) appelle par le serveur MCP de son
  parent, et l'hôte ne distingue pas ses appels. L'ensemble d'un enfant ne vaut que pour un enfant
  qui a sa propre session d'hôte : une tâche de l'hôte, en mode pair.

## Les témoins, rouges d'abord (côté hôte)

- un appel hors de l'ensemble est refusé en nommant la vue ;
- le parent ajoute une vue, et l'appel suivant de l'enfant y passe sans relance ;
- le parent la retire, et l'appel suivant refuse ;
- une commande de l'enfant ne lit rien hors de `~/git_workspaces/<session>/` ;
- un commit de l'enfant porte l'identité de la session.

## Taille

Une journée côté hôte, après l'accord avec arbre, et derrière le raisonnement borné.
