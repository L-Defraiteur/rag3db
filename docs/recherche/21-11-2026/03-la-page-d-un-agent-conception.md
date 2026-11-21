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
3. **La conversation d'un agent Claude** : une route en lecture seule de sa transcription, bornée (les N derniers
   messages), rouge d'abord. À qui : arbre ou moi.
4. **Sa mémoire** : la route de la session mémoire, puis le bloc.
5. **La boucle en graphe**, repliée : la forme fixe animée par le flux ; l'arbre d'un run quand archi aura sa route.

Les tranches 1 et 2 ne demandent rien à personne ; 3 à 5 attendent leurs routes.

## Ce qui reste hors de la vue principale

Les recherches (au second rang, pour l'agent), et la plomberie (hôtes, clients MCP, backends, services, dérives de
gabarits, index) sous « avancé ».

## Questions ouvertes

- Un agent Claude sans nom (un vieux client MCP) : le dire « sans nom » avec son client et son pid, ou le cacher
  sous « avancé » ?
- La conversation d'un agent Claude montrée à l'écran : toute la transcription, ou seulement ses messages et les
  noms des outils (sans les sorties) ?
