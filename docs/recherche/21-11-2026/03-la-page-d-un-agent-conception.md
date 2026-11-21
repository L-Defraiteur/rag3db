# La page d'un agent : conception

Session recherche, 21 novembre 2026, matin. La direction de Lucie (08 h 1x), relayée par c7 : « pour l'instant le
mec il veut des agents de code, pouvoir avoir une vue sur leur mémoire, éventuellement voir la boucle agentique en
graphe si ça l'intéresse, voir la liste des vues qu'un agent voit » ; « les recherches sont plus pour l'agent » ; la
plomberie (hôtes, clients MCP, backends, services, dérives de gabarits, index) va dans des paramètres avancés.

## Ce qu'est un agent, à l'écran

Deux sortes, une seule page :

- **un agent Claude Code** : une session de l'hôte avec un nom Claude (`claude {session_id, name}`), tenue par le
  client MCP, un hook ou le terminal ;
- **un agent de rag3weaver** : une conversation de chat d'un dépôt, et l'équipier d'une tâche.

**Un agent est sa clé stable `agent:<clé>`** (décision de Lucie du 25 octobre 2026), qui traverse les sessions :
une session Claude Code, une conversation de chat et une tâche d'équipier sont des contextes d'exécution de cet agent,
rangés sous sa page ; ses noms sont des alias datés. Un contexte dont l'agent n'a pas encore de clé se liste « sans
nom ». La mémoire (sa page `docs/memoire/21-11-2026/01-ce-que-cet-agent-sait.md`) attribue `author = agent:<clé>` à ce
que l'hôte écrit, puis sert `GET /ide/agent/memoire` : la tranche 4 s'y branche.

L'accueil devient la liste des agents (actifs, puis inactifs) ; la page « Poste » d'aujourd'hui en devient l'entrée
« avancé » (la plomberie).

## Ce que la page montre, et d'où elle le lit

| Bloc | Agent Claude Code | Agent de rag3weaver | Existe côté hôte |
|---|---|---|---|
| **Qui, et depuis quand** | `GET /poste` `sessions[]` (client, pid, `opened_at`, `claude.name`) | `/ide/etat` `chat_sessions[]`, `tasks[]` | oui |
| **Sa conversation** | la transcription Claude Code de sa session (`claude_code_transcript`, déjà lu par les hooks) | `GET /ide/chat?backend=` (les tours) ; pour l'équipier, `GET /ide/teammate` (la boîte) | oui pour rag3weaver ; **manque une route en lecture** de la transcription d'une session Claude (`GET /ide/agent/conversation?session=`) |
| **Ses vues** | `GET /poste` : `open_views` filtrées par `session_id` = `claude.session_id`, l'active dite | sans objet aujourd'hui | oui ; **en option**, `sessions[].views` (tout ce que `views` rendrait, par dépôt), qu'arbre propose |
| **Ses tâches** | `GET /poste` `tasks[]` (pas encore reliées à la session Claude qui les lance) | `tasks[]` de sa conversation, chacune vers sa page vivante | oui ; **manque le lien tâche → session Claude** |
| **Sa mémoire** | ce que la mémoire longue expose pour cet agent (fiches écrites, rappelées) : route à demander à la session mémoire | `memory_writes` de `/ide/chat` (les fiches écrites par ses tours) | partiel ; **manque une route par agent** chez la mémoire |
| **Sa boucle en graphe** (repliée) | sans objet | le run en cours : une forme fixe (pas → tour → outils → porte de vérification → pas suivant / fin) animée par le flux (`RunStarted`, `NodeRun`, `LlmCall`, `ToolCall*`, `Waiting`/`Resumed`, `RunFinished`) | le flux, oui ; **manque** `GET /ide/run?backend=&run=` (l'arbre d'un run d'un coup) et le `RunStarted` d'un pas qui nomme la tâche et son numéro (archi) |

## L'ordre

1. **La liste des agents à l'accueil**, depuis `GET /poste` et `/ide/etat`, sans route neuve : un agent par ligne
   (nom, sorte, actif ou non, depuis quand), un clic vers sa page. La plomberie de « Poste » passe derrière un lien
   « avancé ».
2. **La page d'un agent, première tranche** : qui, ses vues, ses tâches ; la conversation des agents de rag3weaver
   (routes existantes). Rien de neuf côté hôte.
3. **Les vues d'un agent et ce qui n'est pas poussé** : les vues dépliées (route existante), puis leurs commits non
   poussés et leur non-commité, sur la route d'arbre.
4. **La conversation d'un agent Claude** : une route en lecture seule de sa transcription, bornée (les N derniers
   messages), rouge d'abord. À qui : arbre ou moi.
5. **Sa mémoire** : la route de la session mémoire, puis le bloc.
6. **La boucle en graphe**, repliée : la forme fixe animée par le flux ; l'arbre d'un run quand archi aura sa route.

Les tranches 1 et 2 ne demandent rien à personne ; 3 à 6 attendent leurs routes.

## L'accueil en arbre : la parenté, et parler à un sous-agent

Lucie (08 h 2x) : « parler à un agent ET à ses sous-agents si tu déplies les sessions, voir la liste des agents
racines, quelles tâches a chacun, plans en cours, à coup de dépliage de hiérarchie ». L'accueil est un arbre : les
agents racines ; dépliés, leurs sessions et leurs sous-agents ; sous chacun, ses tâches et l'élément en cours de son
plan ; chaque nœud ouvre sa conversation, et la saisie va au nœud ouvert.

| Ce qu'il faut | Ce que l'hôte donne déjà | Ce qui manque |
|---|---|---|
| **Les racines** | les sessions de `/poste` (nom Claude ; « sans nom » = serveur MCP d'avant le 21 nov., à reconnecter par `/mcp`) et les conversations de `/ide/etat` | la clé `agent:<clé>` qui regroupe les contextes d'un même agent |
| **Une tâche sous sa conversation** | `tasks[].session` : le backend dont la conversation a ouvert la tâche | le lien tâche → session Claude (une tâche ouverte par un agent Claude via le terminal ou le MCP) |
| **Un sous-agent sous son agent** | un sous-agent est une tâche d'équipier ouverte par `POST /ide/taches {backend, nom, parent_session, views, active}` (nommée `<session>-<nom>`) ; `GET /ide/taches?backend=` rend les tâches ouvertes et fermées avec leur `parent_session` : racine = la session Claude nommée par `parent_session`, enfants = les tâches qui la portent ; une tâche sans `parent_session` est une racine | une route qui rende l'arbre d'un coup sur tous les dépôts du poste (`GET /ide/agents`, archi la prend) ; un troisième niveau (un équipier qui ouvre une sous-tâche) n'existe pas : l'équipier n'a pas cet outil (`parent_task` reste nul) ; les sous-agents de Claude Code (l'outil Agent) ne sont pas connus de l'hôte |
| **Le plan en cours** | `GET /ide/teammate` : `plan.json`, ses éléments et leur statut (`todo`, `doing`, `done`…) | rien pour l'équipier ; un agent Claude n'a pas de plan que l'hôte connaisse |
| **Parler au nœud ouvert** | une conversation : `POST /ide/chat` ; un sous-agent (une tâche) : `POST /ide/teammate/mailbox {backend, task, kind, text}` — `request` ouvre la tâche et lance l'équipier, `answer` répond à sa question, `stop` arrête le pas ; il lit à sa prochaine frontière (joué cette nuit au scénario 1) | écrire à une racine, une session Claude Code : l'IDE ne le peut pas ; le chemin est le bus entre sessions (le relais par pair, chez archi) |

Ce que la tranche 1 peut montrer sans route neuve : les racines (sessions Claude et conversations), leurs
sous-agents par `parent_session` de `GET /ide/taches`, le plan en cours de chacun, et la saisie vers une conversation
ou la boîte d'un sous-agent. Ensuite, dans l'ordre proposé par archi : `GET /ide/agents` (l'arbre d'un coup), puis
écrire à une racine par le relais ; un troisième niveau seulement si Lucie veut des sous-agents de sous-agents.

## Les vues d'un agent et ce qui n'est pas encore poussé

Lucie (08 h 2x, précisé ensuite) : sous chaque vue, **seulement ce qui n'est pas encore poussé**. Dans l'arbre, chaque
agent déplie ses vues (`open_views` de `/poste` par `session_id`, l'active dite) ; chaque vue, et le worktree courant,
déplie, replié par défaut :

- **les commits non poussés** : sujet, date et stat (fichiers, + et −) ; un clic sur un commit montre son patch ;
- **le non-commité** : sa stat, puis le patch d'un fichier au clic ;
- un filtre « non commité seul ».

Il n'y a pas de « diff sur le parent » à part.

- Ce que l'hôte donne : la liste des vues, leur chemin, leur branche, leur couche, leur base et leur commit de départ
  (`/poste`).
- Ce qui manque : la route de lecture `GET /vues/diff?session=&vue=` dans cette forme (arbre la fait) : un binaire
  ou un renommé y est dit, le patch se borne par fichier, `vue=backend` avec `manifest=` désigne le worktree courant.

## Ce qui reste hors de la vue principale

Les recherches (au second rang, pour l'agent), et la plomberie (hôtes, clients MCP, backends, services, dérives de
gabarits, index) sous « avancé ».

## Questions ouvertes

- Un agent Claude sans nom (un vieux client MCP) : le dire « sans nom » avec son client et son pid, ou le cacher
  sous « avancé » ?
- La conversation d'un agent Claude montrée à l'écran : toute la transcription, ou seulement ses messages et les
  noms des outils (sans les sorties) ?
