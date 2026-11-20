# La palette des outils de Claude Code, vue de l'intérieur (20 novembre 2026)

Lucie, 16 h 3x : « pour les outils on peut s'inspirer, juste voir ce qu'ils ont de plus que nous, et prévoir des
variantes : quelle palette ils ont exactement ? ». Le code du CLI n'est pas publié (le dépôt `anthropics/claude-code`
porte les issues, la documentation, les plugins, les exemples et un dossier `mods`, sous « All rights reserved,
Commercial Terms »). Mais la palette, elle, est lisible : c'est celle que chaque session voit. Relevé fait depuis la
session c7, ce jour, avec en face ce que rag3weaver a, et ce qui manque.

## Les outils toujours chargés

| Outil | Ce qu'il fait | Chez nous |
|---|---|---|
| `Bash` | une commande, avec `run_in_background` (une notification à la fin) et un timeout | l'hôte lance des tâches ; pas d'équivalent « une commande et préviens-moi » dans le chat |
| `Read` / `Write` / `Edit` | lire par plage de lignes, écrire, remplacer une chaîne unique | `read_file` ; les primitives d'édition du socle d'agent de code (ED) |
| `Agent` | un sous-agent frais (ou un **fork** qui hérite du contexte), en fond, avec un rapport ; un `isolation: worktree` lui donne **son propre worktree git** | l'équipier par pas ; le sous-agent à vues décidé ce jour ; le worktree par `open_view` |
| `ListAgents` / `SendMessage` | voir les sessions vivantes, leur écrire par leur nom | le mode pair d'archi (lot 3 : `send_to_session`) : le même geste, côté hôte |
| `Skill` | charger une procédure nommée (`/commande`), parfois jouée dans un sous-agent | les gabarits et les workflows déclaratifs ; les /commandes de tty |
| `ToolSearch` | **outils différés** : la palette entière n'est pas chargée, on cherche un outil par mots et on reçoit son schéma | rien : nos serveurs MCP exposent tout d'un coup ; c'est la réponse à « trop d'outils dans le contexte » |
| `Workflow` | un script JavaScript qui orchestre des sous-agents (`agent`, `parallel`, `pipeline`, phases), en fond, reprise par run id | l'archiviste tout déclaratif (DAG) ; le lanceur de campagne ; pas de script d'orchestration côté session |
| `AskUserQuestion` | une question à choix à l'humain, qui bloque | rien, et c'est voulu : « une question ne bloque jamais » |
| `ScheduleWakeup` | la session se réveille plus tard, à son rythme (boucle autonome) | les minuteries systemd ; pas d'auto-réveil d'une session |
| `Artifact` | publier une page HTML privée, la relire, la mettre à jour | les pages de docs sur master ; l'IDE |
| `ReportFindings` / `SendFeedback` | rendre une revue typée ; un retour produit | les tickets ; check_tickets |

## Les outils différés (chargés à la demande par `ToolSearch`)

| Outil | Ce qu'il fait | Chez nous |
|---|---|---|
| `Monitor` | un flux d'événements en fond : chaque ligne d'un script devient une notification (30 min au plus, à réarmer) | rien côté session ; c7 s'en sert pour le journal du pousseur et master. La montre de l'index est l'équivalent côté hôte |
| `TaskStop` | arrêter une tâche de fond | `stop_run` |
| `WebFetch` / `WebSearch` | lire une page, chercher | rien, et pas prévu |
| `EnterWorktree` / `ExitWorktree` | la session entre dans un worktree isolé, en sort | `open_view` / `close_view` décidés ce jour, côté hôte, avec l'index en plus |
| `EnterPlanMode` / `ExitPlanMode` | un mode où l'agent planifie sans toucher | la page de cadrage avant le code, par convention |
| `CronCreate` / `CronList` / `CronDelete` | des tâches planifiées de la session | les minuteries systemd (ménage, sonde) |
| `PushNotification` | prévenir l'humain sur son téléphone | rien ; « dire à Lucie quand regarder » est manuel |
| `NotebookEdit` | éditer une cellule Jupyter | rien |
| `SendToProject` | envoyer vers un projet claude.ai | rien |
| les `mcp__…` | les serveurs MCP configurés : les nôtres (`code-rag3db`, `memoire-longue`), le navigateur, Slack, Monday, Docs | c'est là que nous sommes |

## Ce qu'ils ont de plus, et les variantes à prévoir

1. **Les outils différés** (`ToolSearch`). Nos serveurs MCP exposent une vingtaine d'outils chacun ; avec le mode pair
   et les vues, la palette grossit. Variante : un serveur qui n'expose que les outils de tête (`search_code`,
   `usages`, `outline`, `read_file`, `recall_fiches`) et un outil `find_tool` qui rend le schéma des autres à la
   demande. Le protocole MCP le permet (`tools/list` peut changer, avec `notifications/tools/list_changed`).
2. **Le worktree du sous-agent** (`Agent` avec `isolation: worktree`, `EnterWorktree`). C'est exactement `open_view`,
   mais chez eux sans index : notre variante indexe le worktree et le rend lisible aux autres vues. À garder comme
   argument : le nôtre sait ce qu'il y a dedans.
3. **Le flux d'événements** (`Monitor`). Un agent qui attend un atterrissage, un index prêt, un rouge du pousseur, le
   fait aujourd'hui par un script tail ; variante côté hôte : un outil `watch` qui rend une notification MCP par
   événement (index prêt, lot atterri, fiche créée), borné dans le temps, à réarmer. La notification « index prêt »
   de recherche (ce jour) en est le premier cas.
4. **L'auto-réveil** (`ScheduleWakeup`) et le **cron de session**. Nos minuteries sont systemd ; un équipier de l'hôte
   pourrait porter son propre réveil (« reprends dans 20 min si rien n'a bougé »).
5. **Le script d'orchestration** (`Workflow`). L'archiviste est un DAG déclaratif, c'est mieux pour la mémoire ; mais
   pour une campagne (dix cas, trois phases), un script court qui appelle des équipiers en parallèle manque. Variante :
   un `campaign.js` lu par le lanceur, avec `agent`, `parallel`, `pipeline`, reprise par run.
6. **Le fork** d'un sous-agent qui hérite de tout le contexte. L'équipier par pas repart d'une invite ; une tâche
   « fille » qui hérite du run (L1 et L2 de mémoire) serait notre fork.
7. **La question bloquante** (`AskUserQuestion`) : à ne pas copier. La règle de Lucie est l'inverse, et le mode pair
   fait mieux : la question va à la session humaine par message, le travail continue.

## Ce que nous avons et qu'ils n'ont pas

L'index de code par le sens avec les usages, l'impact et les vues par worktree ; la mémoire longue en fiches
citables ; les workflows déclaratifs édités par l'agent lui-même ; le pousseur avec ses juges ; le mode pair qui
fait de l'hôte un pair des sessions. Le client ne connaît rien du dépôt au-delà de `grep` et des fichiers ouverts :
c'est ce que Lucie veut avoir « en deux secondes plutôt que plein de grep ».

## La référence officielle, et le socle qu'un agent stable doit avoir

Lucie, 16 h 3x : « ça nous donne une ligne directrice ; on n'est pas obligé de tout reprendre, mais pour qu'un agent soit
stable il doit avoir des équivalents ». La page https://code.claude.com/docs/en/tools-reference liste, au 20 novembre
2026, les outils par leur nom exact (ceux des règles de permission, des listes d'outils des sous-agents et des hooks) :
`Agent`, `Artifact`, `AskUserQuestion`, `Bash`, `CronCreate`/`CronDelete`/`CronList`, `Edit`, `EndConversation`,
`EnterPlanMode`/`ExitPlanMode`, `EnterWorktree`/`ExitWorktree`, `Glob`, `Grep`, `ListAgents`, `ListMcpResourcesTool`,
`ListPlugins`/`SearchPlugins`/`SuggestPluginInstall`, `ListSkills`/`SearchSkills`/`SuggestSkills`/`propose_skills`,
**`LSP`** (définitions, références, diagnostics par un serveur de langage), `Monitor` (avec une source WebSocket),
`NotebookEdit`, `PowerShell`, `PushNotification`, `Read`, `ReadMcpResourceTool`, `RemoteTrigger`, `ReportFindings`,
`ScheduleWakeup`, `SendFeedback`, `SendMessage`, `SendUserFile`, `ShareOnboardingGuide`, `Skill`, `SubagentHandback`,
**`TaskCreate`/`TaskGet`/`TaskList`/`TaskUpdate`** (la liste de tâches de la session, `TodoWrite` retiré),
`TaskStop`, `ToolSearch`, `WaitForMcpServers`, `WebFetch`, `WebSearch`, `Workflow`, `Write`. Chaque outil dit s'il
demande une permission ; `Glob` et `Grep` sont absents par défaut sur Linux (le modèle passe par `Bash`).

**Le socle d'un agent stable**, lu dans cette liste et dans ce que treize sessions ont fait aujourd'hui, en huit
familles ; pour chacune, ce que l'équipier de l'hôte (la tâche de rag3weaver qui porte un modèle) a ou doit avoir :

| Famille | Chez eux | L'équipier de l'hôte | À faire |
|---|---|---|---|
| Fichiers | `Read` (plage), `Write`, `Edit` (chaîne unique), `Glob`, `Grep`, `NotebookEdit` | le socle d'agent de code décidé le 15 nov. (read 200 lignes + empreinte, replace_text, insert_lines, write, glob, list, grep ; ED) | vérifier que les sept sont livrés et jouées par un témoin ; `read` doit dire la vue lue |
| Commande | `Bash` (timeout, `run_in_background`, une notification à la fin ; limite mémoire) | une tâche de l'hôte lance des commandes sous `poste` | un outil `run` de l'équipier : timeout obligatoire, sortie bornée, fond avec notification ; sous le bac à sable d'archi |
| Sous-agents | `Agent` (frais, **fork**, `isolation: worktree`), `SubagentHandback`, `TaskStop` | l'équipier par pas ; le sous-agent à vues décidé ce jour ; `stop_run` | l'enfant qui hérite du run (fork) ; l'ensemble de vues donné par le parent ; le rapport final rendu au parent |
| Entre agents | `ListAgents`, `SendMessage` | le mode pair (lots 1 à 3 : registre, socket, `send_to_session`) | la même chose pour les enfants (`/ide/teammate/views`, routes d'archi) |
| Procédures | `Skill`, `ListSkills`, `Workflow` | les gabarits, les workflows déclaratifs, le lanceur de campagne | une procédure nommée que l'équipier appelle (`skill`), et le script court d'orchestration d'une campagne |
| Palette | `ToolSearch` (outils différés), `WaitForMcpServers` | tout exposé d'un coup | `find_tool` : les outils de tête toujours là, les autres par leur schéma à la demande |
| Temps et événements | `Monitor`, `ScheduleWakeup`, `Cron*`, `PushNotification` | la montre de l'index, la notification « index prêt » (recherche, ce jour), les minuteries systemd | `watch` (une notification par événement : index prêt, lot atterri, fiche créée), `wake_me` (auto-réveil borné), « dire à Lucie quand regarder » par notification |
| Travail et plan | `TaskCreate/Get/List/Update`, `EnterPlanMode`, `EnterWorktree`, `LSP` | les tâches de l'hôte ; la page de cadrage par convention ; `open_view`/`close_view` décidés ce jour ; l'index (usages, impact, outline) qui vaut mieux qu'un LSP | la liste de tâches d'un run, lisible par le parent ; un mode « cadrage seul » qui refuse l'écriture |

Trois choses à ne pas reprendre : `AskUserQuestion` (une question ne bloque jamais ; le mode pair porte la question à
la session humaine et le travail continue), `WebFetch`/`WebSearch` (hors du produit), et les outils de catalogue
claude.ai. Trois choses où nous avons déjà mieux : l'index par le sens avec vues (contre `Grep` et `LSP`), la mémoire en
fiches citables (ils n'ont que le fichier `CLAUDE.md` et la mémoire automatique), le pousseur avec ses juges (rien
chez eux). L'ordre que je proposerais : la palette différée et `run` d'abord (ce sont les deux qui rendent un équipier
utilisable sur un vrai dépôt), puis `watch` et la liste de tâches du run, puis le fork.
