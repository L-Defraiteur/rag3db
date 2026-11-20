# Le socle de l'équipier de l'hôte : cadrage — le vendredi 20 novembre 2026

Lucie a dit oui à 17 h et a élargi le socle : `run`, la palette différée, les serveurs MCP connectés
à chaud, le fetch d'une page, le cron de l'équipier, le fork, et la recherche web. La référence est la
page `docs/orchestration/14-11-2026/04-la-palette-des-outils-de-claude-code.md`. Elle classait
`WebFetch` et `WebSearch` hors du produit : la décision de Lucie les y fait entrer.

Hors de cette page : ED fait le socle fichiers vérifié et la liste de tâches du run, tty fait l'outil
navigateur (le pont jackwener), et arbre fait les outils des vues.

## Les règles de tous les lots

- **Déclaré** : chaque outil neuf est déclaré dans le gabarit (nom, description, paramètres, bornes),
  selon la décision « tout déclaratif » et le ticket du rangement déclaré. L'hôte n'en tient que le
  mécanisme. Ses bornes vivent dans `teammate.*` du manifeste.
- **Borné et dit** : une sortie trop longue est coupée en disant combien, une attente a son délai, un
  refus nomme sa cause. Rien ne se coupe en silence.
- **Sous la porte** : ce qui lance un processus passe par la porte des commandes et le bac à sable de
  la tâche. Ce qui sort sur le réseau passe par une politique déclarée.
- **Rouge d'abord**, avec un faux modèle, un faux serveur ou une fausse horloge, jamais le réseau.
- **Une phrase par paramètre** (Lucie, le 20 novembre à 17 h 25), pour tout outil exposé, MCP ou
  `ToolDef` : ce qu'il fait, son défaut, et un exemple quand la valeur a une forme. Un témoin de lib
  rougit sur tout paramètre sans description dans les schémas rendus (`tools/list`, et les outils que
  le modèle de l'équipier reçoit). Les descriptions d'OpenCode donnent le ton. Le 20 novembre, les
  outils de l'équipier ne la tenaient pas : `step_done` (ses quatre paramètres), `report.text`,
  `suggest.text`, `send_to_session.text` et la plupart des champs d'`edit_plan` n'avaient pas de
  description.

## Les lots, dans l'ordre

### 1. `run` : une commande bornée, au premier plan ou en fond — 1 jour

- `run {command, timeout_s, background?}`. `timeout_s` est obligatoire et plafonné par
  `teammate.run_timeout_max_s`.
- La sortie est bornée : début et fin gardés, et le nombre d'octets coupés est dit.
- En fond, `run` rend un identifiant. La fin dépose une ligne `notice` dans la boîte, avec le code et
  la fin de la sortie, et `run_output {id}` relit la sortie bornée.
- Tout passe par la porte et le bac à sable de la tâche (sans les wrappers du poste, lot
  `bac-sans-sccache`), et sous `poste lourd` pour une commande lourde.
- **Témoins** :
  - sans `timeout_s`, l'appel est refusé ;
  - une sortie de 5 Mo rend début et fin, et dit la coupure ;
  - une commande en fond finit par une ligne `notice` ;
  - une écriture hors du domaine est refusée dans le bac.

### 2. La palette différée — 1 jour

- Les outils de tête (`teammate.tools.head`) sont toujours chargés. Les autres ne sont donnés que par
  leur nom et une ligne, dans l'outil `find_tool {query}`, qui rend leurs schémas et les charge pour la
  suite du pas.
- Un appel à un outil non chargé est refusé, en nommant `find_tool`.
- Un changement de palette (un serveur MCP ajouté, retiré) se dit au pas suivant, comme
  `tools/list_changed`.
- **Référence** (c7) : OpenCode, en clone de lecture `~/git_workspaces/c7/20-11-2026-opencode` (MIT).
  Il n'a pas de ToolSearch, mais un catalogue à espaces de noms
  (`packages/codemode/src/codemode/catalog.ts`) :
  - tous les espaces restent visibles en résumé, et un seul est affiché en entier par tour ;
  - quelques outils sont épinglés (`pinned`) ;
  - `scanTools` et `search` trouvent le reste.

  La forme reprise pour `find_tool` : les épingles (les outils de tête), les espaces (un par serveur
  MCP et un pour l'hôte), la recherche, et `tools/list_changed` vers le client. Leurs descriptions
  d'outils (`packages/core/src/tool/plugin/*.ts`, une phrase par paramètre) servent de modèle pour
  `run` et `fetch`.
- **Témoins** :
  - le modèle ne voit que la tête et `find_tool` ;
  - après `find_tool`, l'outil trouvé s'appelle au tour suivant ;
  - un appel non chargé est refusé en nommant `find_tool`.

### 3. Les serveurs MCP connectés à chaud — 1,5 jour

- `mcp_connect {name}` connecte, en pleine session et sans redémarrer, un serveur nommé du catalogue
  déclaré (`teammate.mcp_servers` : commande ou URL, environnement). Lancer un serveur, c'est lancer un
  processus : un serveur hors du catalogue est refusé, et une commande passe par la porte.
- Ses outils entrent dans la palette différée, préfixés de son nom. `mcp_disconnect {name}` les
  retire. La palette change au pas suivant (lot 2).
- **Témoins** :
  - un faux serveur MCP (stdio) se connecte pendant un run ;
  - son outil se trouve par `find_tool` et s'appelle ;
  - après la déconnexion, l'outil est refusé ;
  - un nom hors du catalogue est refusé.

### 4. Le fetch d'une page — une demi-journée

- `fetch {url, max_bytes?}` rend le texte de la page (HTML ramené au texte), borné, avec son statut
  et son type.
- Une politique déclarée (`teammate.fetch` : schémas permis, hôtes refusés, adresses locales
  refusées par défaut) s'applique. Aucun cookie n'est gardé.
- **Témoins** :
  - un serveur local de test rend une page, coupée à `max_bytes` en le disant ;
  - une adresse locale est refusée par défaut ;
  - un statut 404 est dit, pas caché.

### 5. Le cron de l'équipier — 1 jour

- `schedule {at | every, request, until?}` pose un réveil ou une tâche planifiée. À l'échéance,
  l'hôte dépose `request` dans la boîte de la tâche, ce qui relance l'équipier. `schedule_list` et
  `schedule_cancel` les gèrent.
- Les échéances sont gardées au registre des tâches et survivent au redémarrage de l'hôte. Leur
  nombre et leur fréquence sont bornés (`teammate.schedule_max`, `teammate.schedule_min_interval_s`).
- **Témoins**, sur une fausse horloge :
  - un réveil dépose sa demande à l'heure ;
  - une échéance survit au redémarrage ;
  - un intervalle trop court est refusé ;
  - un réveil annulé ne dépose rien.

### 6. Le fork — 1,5 jour

- `fork {name, request, views?}` ouvre une tâche enfant `<tâche>-<name>`, qui hérite du contexte du
  run :
  - L1, ses tranches rangées, par `compose_restart` ;
  - L2, le report du pas d'avant ;
  - ses vues, dans la limite de l'ensemble du parent (sous-agent à vues).
- L'enfant rend son rapport final à la boîte du parent.
- Il dépend du sous-agent à vues et du rangement au fil.
- **Témoins** :
  - l'enfant voit au premier pas ce que le parent savait ;
  - il ne peut pas élargir les vues du parent ;
  - son rapport arrive dans la boîte du parent.

### 7. La recherche web par le grounding de Gemini — une demi-journée

- `web_search {query}` appelle le modèle avec l'outil de recherche de Vertex (le grounding) et rend
  une réponse courte, avec ses sources (titre, URL). Le nombre de sources est borné.
- Elle passe par le modèle déclaré, et seulement s'il est un Gemini sur Vertex. Sinon, elle est
  refusée en le disant.
- **Témoins** :
  - un faux modèle rend des métadonnées de grounding, et l'outil rend les sources ;
  - un modèle qui n'est pas Gemini est refusé en le disant.

## Qui fait quoi (c7, le 20 novembre au soir)

- **archi** : `run`, la palette différée, les serveurs MCP à chaud et le fork, dans cet ordre. Ils
  viennent après la repose de `mode-pair-4`, le raisonnement borné et le sous-agent à vues.
- **mémoire** : `fetch` et `web_search` (le grounding relève de `models.llm`), après sa pile.
- **recherche** : le cron, après son ticket du chat.

Chacun code depuis cette page. Tant que les outils de l'équipier ne sont pas déclarables dans le
gabarit (ticket `2026-11-19-le-rangement-de-l-hote-devient-un-mecanisme-generique-declare.md`, point
4), un outil d'équipier est une `ToolDef` dans `src/daemon_host/teammate.rs` : sa définition dans
`StepTools::tool_defs`, son traitement dans `Step::ours`, et sa borne dans `TeammatePolicy`. Il
migrera avec les autres. archi donne à chacun la forme exacte de son outil quand il commence, pour
qu'il n'y ait qu'une écriture.

## Le total

Environ 7 jours. Les lots 1 et 2 rendent l'équipier utilisable sur un vrai dépôt. Les lots 3 et 4
l'ouvrent. Les lots 5, 6 et 7 viennent ensuite. Chaque lot porte sa ligne de nouveautés.
