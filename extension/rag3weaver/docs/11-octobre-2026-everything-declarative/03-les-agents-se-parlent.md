# Un agent est un graphe, un graphe est un outil, les agents se parlent

11 octobre 2026, session « everything declarative ». Page de conception avant
le premier lot de code ; rien n'est codé. Pour l'orchestration et pour Lucie.

## 0. La demande

> « un agent est un dag, un dag est potentiellement un outil quand on a besoin
> qu'il le soit (dont les agents), ce que ça veut dire est pas forcément
> d'attendre son retour, peut-être aussi juste de le lancer en background avec
> message initial et ref de qui je suis qu'il puisse me parler… etc » — et :
> « je veux qu'ils puissent se parler ». (Lucie, 11 octobre.)

Ce que les pages d'août ont déjà décidé et que ce chantier tient
(`26-aout-2026-20h29/12-conversations-a-plusieurs.md`,
`27-aout-2026-13h01/09-le-terminal-a-plusieurs.md`,
`vision_roadmap_09_2026/07` et `09`) :

- **un fil ne se ferme pas, un run finit** ; aucun plafond de tours codé en
  dur, on s'arrête de parler par une pause (`pause_dialogue`, son genre est
  sa condition de réveil) ;
- **l'identité d'un run est son adresse** (`run.<id>.inbox`) ; publier ne
  bloque jamais ; un accusé est un second message ;
- **une boucle réagit à sa frontière** : un agent lit sa boîte entre deux
  tours ; un réacteur le réveille sur un événement ;
- l'attente se **voit** (un bloc dérivé dans l'invite, pas un message) ;
- un prompt est lié à la **tâche**, pas à une personnalité ; le rôle est une
  enveloppe de capacités que le moteur fait respecter.

## 1. Ce qui existe, et ce qui manque (relevé sur master `4bb017628`)

Existe : les boîtes (`inbox_topic`, `send_in`, `broadcast`, curseurs nommés,
`events.rs`) ; un agent qui lit sa boîte et en fait des tours `user`
(`Agent::with_inbox`, `read_inbox`) ; les outils réservés que la boucle
annonce et intercepte (`pause_dialogue`, `confirm_pause`, `handle_pause`
prévient le pair) ; la commande en fond (un outil qui rend tout de suite une
poignée et dont la fin arrive dans la boîte : `is_async`, `spawn_async_tool`,
`RunScope`, `ToolInvocation`) ; un graphe comme outil (`GraphTool`,
`GraphToolBox`, qui pose déjà `parent_run`, `agent_bus`, `agent_inbox`) ; la
trace (`Run`, `Message`, `Conversation`, `Participant`, `CHILD_OF`, fil nommé
ou dérivé de la paire) ; le réacteur.

Manque :
1. **aucun outil n'écrit à un pair** au choix du modèle (`SendMessageNode` a
   son destinataire et son contenu en configuration, et lit le bus du
   catalogue, pas celui de l'agent) ;
2. **un agent n'est pas un graphe** : aucun nœud ne fait tourner la boucle
   d'agent ; un agent n'est déclaré que par un `ChatConfig` JSON ;
3. **pas de parent** : `RunStarted.parent` d'un agent vaut toujours `None` ;
4. **un agent ne reste pas à l'écoute** : quand il rend son texte, sa boucle
   finit ; il faut un réacteur sur sa boîte pour le réveiller (les tests le
   font à la main, et c'est le test qui relaie la réponse) ;
5. **pas d'annuaire** : on adresse par run id ; les tests posent nom = run id ;
6. le chat a un bus **privé par run** : sa boîte ne reçoit que les fins de
   ses propres outils ; et `ObservedTools` / `CompletionTools` perdent
   l'invocation (`call_with`), donc bus, boîte et portée.

## 2. La forme

### 2.1 Un agent est un graphe : `AgentNode`

Un nœud de plus dans le moteur de graphes, de la même forme qu'un
`GraphNode` : il fait tourner la boucle d'agent.

- Entrée : `message` (le message initial, ou celui qui le réveille).
- Sorties : `answer` (le texte final du tour), `run` (sa référence).
- Configuration : la **tâche** (l'invite, liée à la tâche), les **outils
  qu'il voit** (des noms d'outils du backend : c'est le gabarit qui dit
  quels graphes sont exposés à quel agent), ses bornes (`max_iterations`,
  budget). Le modèle vient du service du backend (`models.llm`), comme le
  chat de la page vivante.
- Il lit `parent_run` (déjà posé par `GraphToolBox::call_with`) pour son
  `RunStarted.parent`, et l'adresse de l'appelant pour pouvoir lui écrire.

Un agent déclaré est donc **un graphe `.mmd`** qui contient un `AgentNode`,
attaché comme outil au manifeste — **pas de section de plus, pas de type de
plus** : la projection manifeste → outil existe déjà (`describe()`), et un
agent devient un outil comme n'importe quel graphe. Un agent peut appeler un
agent.

### 2.2 Deux modes d'un même outil

- **Attendre** : l'appel ordinaire ; le retour est la valeur (`answer`).
- **Lancer en fond** : le retour est **la référence du run** tout de suite ;
  le lancé reçoit son message initial avec **la référence de l'appelant**,
  peut lui écrire pendant toute sa vie (`send`), et sa fin arrive dans la
  boîte de l'appelant (comme la commande en fond).

### 2.3 Deux verbes réservés, comme `pause_dialogue`

Annoncés par la boucle quand l'hôte le permet (`with_messaging()`),
interceptés par nom :

- **`send(to, content, conversation?)`** : à un run, à un participant par son
  nom, ou dans un fil nommé (à tous ses participants) ; le modèle choisit le
  destinataire et le contenu. La réception n'a pas d'outil : un message reçu
  est déjà un tour `user`, qui dira désormais aussi le fil.
- **`spawn(tool, arguments, message)`** : lance en fond un outil déclaré (un
  agent, ou n'importe quel graphe permis), rend sa référence. C'est le mode
  « en fond » de 2.2, sans toucher aux schémas des outils.

### 2.4 Rester à l'écoute

Un agent lancé en fond ne meurt pas à la fin de son premier tour : un
réacteur sur sa boîte le réveille à chaque message, avec son historique.
Il s'arrête quand il le dit (une pause de genre `finished`, ou `leave`),
quand sa durée déclarée est passée, ou à l'arrêt de l'hôte — jamais à la fin
du tour de son appelant (l'appelant peut être un chat, dont chaque tour est
un run).

### 2.5 Un annuaire des participants

Un registre de la session (à côté des `Postures`) : nom → run en cours,
participants d'un fil. `send` adresse par nom ou par run ; un fil nommé sait
qui le lit. Rien de neuf en base : `Participant`, `PARTICIPATES_IN`,
`CHILD_OF` suffisent, et la trace pose `CHILD_OF` dès que le parent est
renseigné.

### 2.6 Les bornes, au manifeste de l'appelé

Sur l'attachement de l'outil qui porte l'agent : qui peut le lancer (quels
agents, quelles clés), combien à la fois, sa durée, son nombre de réveils.
Quand deux demandes se contredisent : « à débattre » (vision du 4 octobre) —
ce chantier ne tranche pas, il garde l'ordre d'arrivée dans la boîte et le
dit.

### 2.7 Le chat et la page vivante

Leur boîte reçoit les messages des pairs, pas seulement les fins d'outils :
un bus et une boîte **de session**, plus seulement de run, et l'invocation
relayée par toute la chaîne d'outils. La page vivante est à moi ; le chat
(`chat.rs`), le démon et le serveur MCP sont à l'arbre principal : je lui
propose l'interface, je ne touche pas à son fil.

## 3. Quatre choix

1. **Le mode « en fond »** : A (recommandé) un verbe générique `spawn(tool,
   arguments, message)` ; B un paramètre réservé `background` ajouté au
   schéma de chaque outil qui le permet (le précédent de `run.mmd`). A ne
   touche aucun schéma et met la politique au même endroit pour tous.
2. **L'adressage** : A (recommandé) un annuaire de session, nom → run ; B
   des run ids seulement (ce que font les tests aujourd'hui, nom = run id).
3. **La vie d'un agent lancé** : A (recommandé) jusqu'à sa pause `finished`
   ou `leave`, sa durée déclarée, ou l'arrêt de l'hôte ; B lié au run de
   l'appelant (tué à sa fin, comme une commande en fond).
4. **Où se déclare un agent** : A (recommandé) un `.mmd` avec un `AgentNode`,
   attaché comme outil ; B une section `agents` au manifeste, projetée en
   outils.

## 4. Les lots

Chacun : témoin rouge d'abord (faux modèle, `MockLlm`/`CallbackLlm`), tests
du changement, fusion en avance rapide, worktree effacé. Un vrai modèle
seulement pour l'expérience finale.

| Lot | Ce qu'il pose | Témoin rouge | Fichiers (et relecture) |
|---|---|---|---|
| 1 | `send` réservé ; l'annuaire ; le fil dit dans le tour reçu | deux agents : c'est l'**agent** qui répond à l'autre par `send`, pas le test | `agent.rs`, `postures.rs` voisin (relu par la recherche) |
| 2 | `with_parent` ; `AgentNode` ; un agent appelé comme un outil qui attend | un agent appelle l'outil d'un autre et reçoit sa réponse ; `CHILD_OF` en trace | `agent.rs`, `dataflow/` (relu par la recherche) |
| 3 | `spawn` ; rester à l'écoute ; les bornes au manifeste | un agent en lance un autre en fond, reçoit sa réponse dans sa boîte ; la durée l'arrête | `agent.rs`, `backend.rs`, `dataflow/reactor.rs` (relu) |
| 4 | la boîte de session dans la page vivante ; l'interface proposée au chat et au MCP | un pair écrit au chat de l'atelier, le tour suivant le lit | `atelier.rs` ; `chat.rs`/`harness.rs` proposés à l'arbre principal |
| 5 | l'expérience à vrai modèle : deux agents du jouet qui se parlent pour une tâche | — (expérience, gardée avec son artefact) | — |

Hors de ce chantier, dits pour ne pas y tomber : la boîte durable entre
processus (`MessageSourceNode`), `interrupt`, les rôles attachés à une
identité, les gabarits `builtin/*`, la présence par curseurs.
