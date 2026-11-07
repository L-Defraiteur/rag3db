# Une session, plusieurs backends

7 novembre 2026, everything declarative. La question de Lucie : « je comprends pas pourquoi une
session doit être limitée à un backend, pourquoi c'est pas l'agent qui les explore ? », et avant :
« après je veux voir ça en mode worktrees : une session prend un lot de worktrees sur plusieurs
repos ». Pas un lot ce soir ; la réponse, et par où commencer.

## Le vocabulaire : celui de git, jamais « backend » à l'écran

Lucie, 7 novembre 2026 : « les gens qui bossent, eux, parlent pas spécialement en backends ; ils
parlent en branches, PR, worktrees, repos attachés ». Donc :

- **Ce qu'on voit et ce que l'agent dit** : une session **s'attache à des dépôts** ; dans un dépôt,
  **un worktree sur une branche** ; une **tâche** vit dans un worktree et finit **en PR ou en
  fusion** ; la mémoire longue est **« la mémoire »**. « Backend » reste un mot du manifeste et du
  code — jamais de l'écran, jamais des commandes ni des outils de l'agent.
- **Deux mots selon la nature** (Lucie : « il faut séparer les deux notions ») : un **dépôt** pour
  le code (worktrees, branches, PR) et un **projet rag3weaver** pour le déclaratif pur, sans dépôt
  git (le constructeur de decks, un carnet…), plus **la mémoire**. « Backend » couvre les trois
  dans le manifeste et le code ; l'hôte les distingue par `kind: code | memory | project` dans
  `/ide/etat` (posé par hooks-essai au lot B2a), et l'écran nomme chacun par son mot.
- **La barre de gauche : la session d'abord, ses dépôts dessous** (Lucie : « on doit quand même
  naviguer par "agent" ou "session", mais dans la barre on doit voir en hiérarchie de lui les dépôts
  qui sont chargés ou pas pour lui, et en accès rag ou pas »). Premier niveau : **la session**
  (un agent : son nom, son modèle, sa jauge de contexte, ses fiches écrites). Sous elle, **ses
  dépôts attachés**, chacun avec deux états : **chargé / non chargé** (son backend ouvert dans
  l'hôte ou non) et **rag : indexé / en cours / absent** (l'index de son worktree) ; ses worktrees
  et branches sous chaque dépôt. **La mémoire** est un enfant de la session, avec son accès
  (chargée, et lue ou non par la session). Les dépôts attachés à aucune session sont en bas, en
  **« disponibles »**.
- **Tranché avec Lucie (7 novembre, par c7)** : une session (un agent) a deux groupes, **Dépôts**
  (› worktrees : branche, chargé, rag) et **Sous-sessions** (le même motif, récursif, avec leur
  état) ; un worktree tenu par une sous-session se voit des deux côtés (un badge) ; un fil
  d'Ariane, un filtre « actifs seulement », les dépôts « disponibles » en bas, la mémoire enfant de
  la session ; un menu « par session / par dépôt » qui inverse l'arbre (dépôt › worktree › les
  sessions qui le tiennent). Les sous-sessions sont les **sous-équipiers** de la page 01 (lot 5) ;
  aujourd'hui, ce sont les équipiers des tâches de la conversation.
- (Avant le manifeste de session, une session = une conversation : ses dépôts sont celui de sa
  conversation et ses tâches, sa mémoire celle que son manifeste déclare.)
- **La liste des dépôts sous une session** : les **dépôts attachés**, puis sous chacun ses worktrees et
  leurs branches (et ses PR ouvertes quand le dépôt a une forge) ; une conversation est rangée sous
  le dépôt ou le worktree qu'elle tient. Plus de « backends ouverts ».
- **Le manifeste de session** se dit « dépôts attachés » (`repos`), chacun avec ses worktrees ; le
  backend d'un dépôt (son index, ses outils) en découle, l'utilisatrice ne le nomme pas.
- **Les outils de l'agent** prennent un dépôt ou un chemin, jamais un id de backend ; l'hôte
  résout le backend.
- **La vue de session** montre ce que cette session a fait, sur quels worktrees (branches,
  commits, PR), pas sur quels backends.
- **À appliquer** dans les lots suivants, l'IDE d'abord : la barre du lot du chat (B2 de
  hooks-essai) dit « dépôts » et liste les dépôts attachés ; le terminal de tty de même.

Dans le reste de cette page, « backend » désigne la pièce du code ; l'écran dira « dépôt ».

## La forme de `/ide/etat` pour cette barre

Accordée avec hooks-essai, qui habille la barre ; les champs d'aujourd'hui (`backends`) restent pour
les vues qui les lisent.

```json
{
  "chat_sessions": [{
    "id": "<backend du chat>", "name": "<nom affiché>", "model": "gemini-3.8-flash",
    "parent": null, "state": "active" | "waiting" | "done" | "error", "awaits_answer": false,
    "repos": [{
      "backend": "<id>", "repo": "rag3db", "branch": "master",
      "worktrees": [{ "backend": "<id>", "branch": "agent/doc", "worktree": "<racine>", "loaded": true,
                      "rag": { "state": "indexed" | "indexing" | "absent", "percent": 87 },
                      "held_by": "<id de session>" | null }]
    }],
    "memory": { "backend": "<id>" | null, "rag": true }
  }],
  "available_repos": [{ "backend": "<id>", "repo": "lucivy", "branch": "main", "loaded": false, "rag": { "state": "indexed" } }]
}
```

`chat_sessions` et non `sessions` (le mot désigne déjà les clients de l'hôte). Une session, avant le
manifeste de session, = une conversation (une entrée de `chats`) : ses `repos` sont son dépôt et
les worktrees de ses tâches ouvertes ; les sessions MCP de Claude Code n'y entrent qu'avec
l'identité d'agent à clé stable. `loaded` : le slot du backend est ouvert dans l'hôte (lu sans
attendre). `rag` se lit dans `index_states` (déjà dans `backends[i].index`) : `indexed` si le texte
est là, `indexing` avec un pourcentage si un lot est en cours, `absent` sinon. La jauge et les
fiches écrites d'une session se lisent dans `GET /ide/chat`. **Les sous-sessions sont à plat** :
la page reconstruit l'arbre par `parent` (les sous-sessions d'une session sont les `chat_sessions`
dont `parent` vaut son id) — aujourd'hui les tâches d'un backend, `parent` = `tache.parent` ; le
lien entre équipiers viendra avec les sous-équipiers (lot 5 de la page 01), même forme. `state`
suit `teammate_list` (`active` = un pas en vol), `awaits_answer` dit une question ou une
approbation sans réponse ; `held_by` = la tâche qui tient ce worktree, tant qu'elle est ouverte —
d'où le « tenu par » des deux côtés. La vue « par dépôt » se calcule dans la page.

## 1. Pourquoi aujourd'hui un chat = un backend

Rien de profond : c'est l'endroit où chaque pièce a été rangée, une à une.

- **La session est rangée par backend** : l'hôte tient `chats: id du backend → session`, le fil se
  garde sous `<racine des chats>/<id>`, la route est `/ide/code/<id>/chat`.
- **Les outils sont ceux d'un manifeste** : un tour construit sa boîte depuis `b.prepared()`
  (`OutilsDuChat::de`, `DeclarationTools`, `BackendTools`) — les outils attachés à *ce* backend,
  appelés par `Backend::call` sur *sa* base.
- **Le modèle et la politique se lisent dans ce manifeste** : `models.llm`, `declarations`,
  `teammate`, `bilans`, la porte des commandes du workspace.
- **Une tâche est un worktree du dépôt qui porte ce manifeste** (`ouvrir_une_tache(parent, nom)`),
  avec la copie de sa base ; la boîte de l'équipier est celle de cette tâche.
- **La mémoire est un autre backend**, atteint par côté : le juge y écrit (`bilans.memoire`), et
  chez Claude Code le hook parle au backend « memory » à part. Le chat lui-même ne la lit pas.

Claude Code fait déjà l'autre forme : ses serveurs MCP (`code-rag3db`, `memoire-longue`) sont des
backends distincts, et c'est l'agent qui choisit lequel appeler. Notre chat n'a simplement jamais
eu qu'une boîte d'outils.

## 2. Ce que serait une session à plusieurs backends

- **Un manifeste de session** (déclaratif, comme le reste) : ses backends (`{alias, manifest}` :
  le code de plusieurs dépôts, la mémoire longue…), ses worktrees (un par dépôt, s'il travaille
  dans une tâche), son modèle, sa politique d'équipier. Une session devient une entité à elle, avec
  son id, sa route `/ide/session/<id>`, son fil et sa boîte.
- **Une seule boîte d'outils, union des backends** : les outils d'un backend secondaire préfixés
  par son alias (`memoire.recall_fiches`, `lucivy.search_code`) ; les outils de fichiers résolus
  par le chemin (le worktree qui le contient), si bien que `read_file` reste un seul outil.
  L'étanchéité du lot 1 vaut par backend : un backend qui manque rend ses outils indisponibles, les
  autres continuent.
- **Un seul fil, une seule boîte** pour la session ; les sous-équipiers (lot 5 de la page 01)
  peuvent alors porter chacun un sous-ensemble des worktrees.
- **Les worktrees en lot** : « une session prend un lot de worktrees sur plusieurs repos » =
  la session ouvre autant de worktrees qu'il lui en faut, plusieurs dans un même dépôt s'il le
  faut (Lucie : « pas obligé un worktree par repo » ; page 03), chacun (une branche, un worktree, une vue de l'index par dépôt
  quand les vues de la vision du 6 novembre seront là), et les fusions restent par dépôt, à un
  humain.

## 3. Ce que ça coûte, ce que ça casse, par où commencer

- **Le journal des runs et le juge** sont rangés par backend (`juger(backend_id, run)`, le
  journal par backend) : il faut un journal par session, chaque appel d'outil portant le backend
  qu'il a touché ; le juge lit le run de la session.
- **L'IDE ouvre un backend** : la vue du chat, la barre, les runs par rôle se lisent par backend ;
  la vue de session les agrège (le flux porte déjà le `backend` de chaque trame).
- **La relecture d'une tâche** devient une relecture par dépôt dans la même page.
- **Le prompt grossit** (deux backends ≈ trente outils) : les préfixes et les descriptions doivent
  rester sobres ; c'est ce que Gemini et Claude tiennent déjà chez Claude Code.
- **Rien ne casse si on garde l'ancienne forme comme cas particulier** : une session à un seul
  backend est exactement le chat d'aujourd'hui.

**Par où commencer** (ce que fait déjà Claude Code) : un chat qui a **le backend de code et la
mémoire longue à la fois**. Le manifeste de code déclare déjà sa mémoire (`bilans.memoire`) ; la
boîte du tour y ajoute les outils de lecture de la mémoire, préfixés (`memoire.recall_fiches`,
`memoire.get_fiche`), appelés sur ce backend-là, l'étanchéité comprise. Un lot court, qui ne
change ni la route ni le journal, et qui prouve l'union des boîtes et les préfixes avant le
manifeste de session. Puis le manifeste de session et le journal par session, puis les worktrees
en lot.
