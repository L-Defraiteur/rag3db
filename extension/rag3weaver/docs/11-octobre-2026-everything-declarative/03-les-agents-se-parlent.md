# Un agent est un graphe, un graphe est un outil, les agents se parlent

11 octobre 2026, session « everything declarative ». Page de conception avant
le premier lot de code ; rien n'est codé. Reprise le même jour : les choix
pris (§3), la boucle comme graphe à ports et les flux (§4), les lots revus (§5). Pour l'orchestration et pour Lucie.

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

Un agent lancé en fond ne meurt pas à la fin de son premier tour : il est
réveillé à chaque message de sa boîte, avec son historique. Il vit **tant
qu'un de ses flux d'entrée est ouvert** (§4.5) — jamais seulement le temps du
tour de son appelant (l'appelant peut être un chat, dont chaque tour est un
run).

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

## 3. Les choix pris (orchestration, 11 octobre — renversables par Lucie)

1. **Le mode « en fond »** : un verbe générique `spawn(tool, arguments,
   message)`, pas un paramètre ajouté au schéma de chaque outil.
2. **L'adressage** : un annuaire de session (nom → run, participants d'un
   fil), pas des run ids seuls.
3. **La vie d'un agent lancé** : indépendante du run de l'appelant — et, avec
   les flux (§4), réduite à une règle : **il vit tant qu'un de ses flux
   d'entrée est ouvert**.
4. **Où se déclare un agent** : un `.mmd` qui contient un `AgentNode`, attaché
   comme outil ; pas de section `agents`.

## 4. La boucle est un graphe à ports, et les ports portent des flux

Lucie, le 11 octobre : « j'aimerais à tout moment pouvoir faire un appel
structuré, par exemple selon ce qu'a dit l'user une classification, routing
vers autre agent, ou action spécifique, que ça se voie au possible dans un
graph […] que tout soit traduisible niveau graph de l'agent. Comme un conv
flow mais en beaucoup plus permissif, en synergie avec notre moteur. » Les
cas qu'elle cite sont **des exemples** : le mécanisme est **n'importe quel
sous-graphe sur n'importe quel port**. Et sa règle de méthode : « complexité
on s'en fout, jamais un argument, l'important c'est que ce soit au top ».

### 4.1 Un seul mécanisme : le flux

- **Tout port porte un flux** : des éléments dans le temps, et une fin. Une
  valeur est un flux d'un seul élément, marqué comme tel à la vérification du
  graphe — pas un second type de port.
- **Un flux peut finir vide.** C'est ce qui manque aujourd'hui au moteur pour
  brancher : une branche non prise y est une entrée requise qui ne reçoit
  jamais rien, et le graphe entier finit en « deadlock » (`runtime.rs`, pas de
  statut « sauté » dans `report.rs`). Avec les flux, une branche non prise
  reçoit un flux fini vide ; le nœud qu'elle alimente est **sauté**, et le
  rapport d'exécution le dit (`NodeStatus::Skipped`).
- **Un nœud dit sa posture** : *élément par élément* (il voit chaque élément
  à son arrivée — un message d'un pair, un jeton) ou *sur le flux complété* (il
  voit le tout — le message entier, jamais des jetons). La vérification commune
  du graphe **refuse un nœud « complet » branché sur un flux sans fin**.
- **L'exécuteur** : aujourd'hui, un nœud tourne une fois, ses sorties sont
  rangées à sa fin, les nœuds vont par niveaux topologiques
  (`runtime.rs`, `run_level`, `run_level_async`). Les précédents de flux sont
  hors du moteur de graphes : le puits de jetons (`TokenSink`) et la boîte
  lue entre deux tours. Il faut un exécuteur qui lance les nœuds d'un graphe
  ensemble et les relie par des canaux bornés, et dont **l'exécution d'un
  graphe de valeurs seules redonne exactement l'exécution d'aujourd'hui** :
  c'est la condition de tout le reste (toute la lib verte, les points de
  reprise, le rapport, l'annulation).
- **Pas de cycle dans un graphe** : « le retour au début » d'une boucle
  d'agent n'est pas une arête qui remonte. Le tour est le graphe ; la boucle
  est la répétition du tour à sa frontière (la règle d'août : « une boucle
  réagit à sa frontière, pas au milieu »).

### 4.1 bis Ce que l'exécuteur de flux doit tenir (lot 0, porté par la session recherche)

La session recherche tient l'exécuteur, les points de reprise et le rapport :
elle porte le lot 0 ; cette page en écrit la spécification et le témoin, et
cette session relit ses lots.

**Ce qu'il doit faire**
1. Un port porte un flux : des éléments, puis une fin. Une valeur est un flux
   d'un élément, **marqué à la vérification du graphe** (la forme d'un port
   se connaît avant d'exécuter). Le schéma d'un port ne change pas ; c'est le
   moment où chaque élément arrive qui change (la note du 8 octobre, §4 :
   « le schéma ne change pas »).
2. Un nœud émet plusieurs éléments sur un port et le ferme ; il lit un flux
   d'entrée élément par élément, ou attend sa fin. **Sa posture se déclare**
   (dans son schéma, comme ses ports) : *élément* ou *complété*.
3. **Un flux qui se ferme sans élément** saute le nœud aval qui attendait une
   entrée requise : `NodeStatus::Skipped`, un événement le dit, le rapport
   le montre, et les nœuds qu'il alimentait reçoivent à leur tour des flux
   vides. Plus de « deadlock » pour une branche non prise ; l'erreur reste
   pour un vrai défaut (un producteur qui échoue).
4. **La vérification commune refuse** un nœud *complété* branché sur un flux
   sans fin, en le nommant, avant l'exécution.
5. Les nœuds reliés par des flux tournent ensemble, par des **canaux bornés**
   (la contre-pression, pas une file qui grossit) ; un fan-out donne un
   canal par arête (ou une diffusion), un fan-in devient un multiplexage.

**Ce qu'il ne doit pas casser — la parité**
Tout graphe d'aujourd'hui (des valeurs seules) donne **le même rapport** :
mêmes nœuds, mêmes statuts, mêmes sorties, même ordre des `NodeCompleted` à
l'intérieur d'un niveau, les niveaux en parallèle comme aujourd'hui. Les
témoins qui le verrouillent déjà (`runtime.rs`) :
`runtime_linear_pipeline`, `runtime_fanout`, `runtime_fanin`,
`fan_in_waits_for_longer_branches_in_live_and_checkpoint_execution`,
`nodes_of_one_level_run_in_parallel_and_finish_in_order`,
`un_niveau_mixte_sequentialise_et_ca_se_voit`,
`un_noeud_asynchrone_traverse_le_pont_synchrone`, le pont réentrant,
`runtime_execute_with_report`, les taps (`runtime_tap_specific_edge`,
`runtime_tap_all`) ; les points de reprise (`checkpoint_full_execution`,
`checkpoint_resume_after_failure`, `checkpoint_graph_hash_mismatch`,
`checkpoint_already_completed_is_noop`,
`un_lot_se_checkpointe_en_octets_et_se_reprend`,
`en_mode_operations_la_reprise_rejoue_tout`) ; l'annulation dans l'ordre
inverse (`migration_nodes.rs`, `catalog.rs` `migration_rollback_graph`) ; et
toute la lib.

**Ce qu'un flux demande aux points de reprise et à l'annulation**, à trancher
par la recherche et à dire : un flux n'a pas de « valeur finale » — soit il se
matérialise à sa fin pour le point de reprise, soit ses nœuds se rejouent
(comme le mode `Operations`) ; l'annulation garde un ordre de fin connu.

**Le témoin de flux** (le premier, avant toute généralisation) : un nœud qui
émet des jetons un par un, un nœud *élément* qui les voit dans l'ordre pendant
que le premier travaille, un nœud *complété* qui voit le message entier ; une
branche qui se ferme vide, sautée et dite au rapport ; un nœud *complété* sur
un flux sans fin refusé à la vérification.

**La frontière de fichiers** (à confirmer à deux avant le premier commit) :
la recherche tient `src/dataflow/runtime.rs`, `graph.rs`, `node.rs`,
`port.rs`, `report.rs`, les points de reprise et `rt.rs` ; cette session tient
les points d'accroche de la boucle (`agent.rs`, relus par la recherche), un
fichier neuf pour l'exécution des sous-graphes de ports, les nœuds `decide`
et `route` (fichiers neufs dans `src/dataflow/`, relus), `send`, `spawn`,
l'`AgentNode`, et la page vivante.

### 4.2 Le graphe par défaut d'un agent

L'`AgentNode` est la forme compacte d'un graphe de tour, à ports nommés :

| port | flux | ce qui y passe |
|---|---|---|
| `user_in` | de un | le message de la personne |
| `peer_in` | ouvert | les messages des pairs, avec leur fil |
| `tool_result_in` | ouvert (un outil synchrone : de un) | les résultats d'outils, ceux des outils en fond compris |
| `model_out` | de jetons, qui se complète en message | la réponse du modèle avant tout aiguillage |
| `tool_call_out` | un par appel | les appels d'outils annoncés |
| `answer_out` | jetons pour l'écran, valeur pour la trace | la réponse finale |
| `turn_end` | de un | la fin du tour : continuer ou s'arrêter |

**Sans rien de branché, il se comporte exactement comme `agent.rs`
aujourd'hui** : la boîte lue avant chaque appel au modèle, les outils réservés
interceptés, les outils en fond, les bornes, l'annulation, le contrôle de
complétion.

**Comment, dans l'ordre** : d'abord, la boucle d'`agent.rs` reste le moteur
et chaque port y devient un **point d'accroche** où un sous-graphe déclaré est
exécuté (les endroits sont relevés au `02`, ligne par ligne : la lecture de la
boîte pour `peer_in`, après le rattrapage des appels pour `model_out`, la
branche finale pour `answer_out`, autour de l'interception des outils réservés
pour `tool_call_out`, avant l'ajout du résultat pour `tool_result_in`, la fin
d'itération pour `turn_end`). Ensuite, quand l'exécuteur de flux tient la
parité, le tour lui-même devient l'exécution du graphe par défaut, et le code
de la boucle n'est plus que son repli. **Le témoin de parité** est le même aux
deux étapes : les tests de la boucle (`a_plain_answer_ends_the_loop`,
`parallel_calls_are_all_announced_before_all_results`,
`an_async_tool_answers_with_a_handle_and_the_result_comes_later`,
`an_interruption_mid_call_leaves_a_replayable_history`, les pauses, les bornes,
l'absorption…) passent à l'identique, et un scénario à faux modèle donne le
même historique, les mêmes appels, et **les mêmes jetons dans le même ordre**
sur `model_out` — le premier cas de flux, avant toute généralisation.

Une contrainte du protocole, dite : un appel annoncé reçoit toujours un
résultat. Un sous-graphe qui retient un appel en fabrique un (comme le fait
déjà `close_orphan_tool_calls`).

### 4.3 Le plus petit jeu de nœuds

Les exemples de Lucie (classer un message, aiguiller vers un agent ou une
action, transmettre ou retenir le message d'un pair, faire quelque chose de
plus après un outil ou un agent) se composent de **deux nœuds neufs** :

- **`decide`** : un appel structuré au modèle — le texte du flux complété, N
  étiquettes — qui rend une étiquette (et sa probabilité quand le fournisseur
  la donne). Deux voies, choisies par ce que le modèle déclaré sait faire :
  le `Decider` (déjà au crate, une probabilité par option en un jeton, servi
  par llama-server, sans aucun nœud client aujourd'hui) ; la sortie structurée
  (`GenOptions::with_response_format`, schéma à `enum`, honorée par le client
  compatible OpenAI, donc Vertex et llama-server). Le décodage contraint n'existe
  pas encore : la vérification après coup, comme `BadChoice`, refuse une
  étiquette hors liste.
- **`route`** : un flux d'entrée et une clé (une étiquette, un champ) ; il
  transmet chaque élément sur le port de sortie de cette clé et **finit les
  autres vides**.

Et ce qui n'est pas un nœud de plus :

- **`filter` est un `route`** dont une sortie ne mène nulle part (retenir) ;
  réécrire un message est un nœud scripté ou un appel au modèle posé avant.
- **`act` est l'appel d'outil qui existe** : un graphe-outil posé comme nœud
  (`GraphTool::as_node_factory`), ou `send` / `spawn` posés comme nœuds (§4.5).
- Ce que deux nœuds ne disent pas, un **nœud scripté TypeScript** le dit.

Pas d'états : un flot de conversation classique fige des états, ici il n'y a
que des ports, des flux et des sous-graphes.

### 4.4 La synergie avec le moteur

Chaque décision, message et aiguillage retombe en base avec la trace (`Run`,
`Message`, `Conversation`, `Participant`, `CHILD_OF`) ; un `decide` ou un
`route` peut lire le graphe par l'IR (`Hop`, `Select`) avant de choisir ; les
fiches mémoire et le catalogue sont dans la même base que l'agent.
Configuration et exécution sont le même objet : tout se rejoue et se mesure
(« un nœud de décision, pas une boucle d'agent : elles se mesurent »).

### 4.5 Ce que les flux donnent au reste

- `send` et `spawn` sont des verbes que le modèle choisit **et** des nœuds
  qu'on pose sur un port : le déclaré et le choisi, un seul mécanisme.
- **`interrupt`** = fermer un flux avant sa fin : `model_out` coupé (le texte
  partiel gardé, l'appel d'outil inachevé refermé comme aujourd'hui).
- **La politique de livraison** = le moment où le nœud modèle lit `peer_in`
  (par défaut : avant chaque appel au modèle, comme aujourd'hui).
- **La vie d'un agent lancé** = tant qu'un de ses flux d'entrée est ouvert ;
  sa pause `finished` ou `leave` ferme le sien, sa durée déclarée ou l'arrêt de
  l'hôte les ferment.
- La voix (STT, TTS) : des ports de flux ordinaires.

## 5. Les lots

Chacun : témoin rouge d'abord (faux modèle), tests du changement, fusion en
avance rapide, worktree effacé. `src/dataflow/` et la boucle d'agent passent
par la relecture de la session recherche avant master.

| Lot | Ce qu'il pose | Témoin rouge |
|---|---|---|
| 0 | (**porté par la session recherche**, spécification au §4.1 bis) **les flux dans le moteur de graphes** : un flux fini vide, `NodeStatus::Skipped`, la posture d'un nœud, la vérification (pas de « complet » sur un flux ouvert), l'exécuteur à canaux bornés — avec la parité exacte des graphes de valeurs | une branche non prise est sautée et dite, pas un « deadlock » ; un flux de jetons lu élément par élément ; un nœud complet sur un flux ouvert refusé à la vérification ; toute la lib verte |
| 1 | la boucle comme graphe à ports : les points d'accroche, le premier flux (`model_out` en jetons) | parité : mêmes historiques, appels et jetons sur un scénario à faux modèle |
| 2 | `decide` et `route`, posés sur un port | un exemple de Lucie : le message de la personne classé, aiguillé vers un outil ou vers rien |
| 3 | `send` et l'annuaire | c'est l'**agent** qui répond à l'autre par `send`, pas le test |
| 4 | `AgentNode`, le parent, un agent appelé en attendant | un agent appelle l'outil d'un autre et reçoit sa réponse ; `CHILD_OF` en trace |
| 5 | `spawn`, rester à l'écoute (flux ouvert), les bornes au manifeste | un agent en lance un autre en fond et reçoit sa réponse dans sa boîte ; la durée ferme ses flux |
| 6 | la boîte de session dans la page vivante ; l'interface proposée au chat et au MCP | un pair écrit au chat de l'atelier, le tour suivant le lit |
| 7 | l'expérience à vrai modèle, gardée avec son artefact | — |

**Le lot 0 est porté par la session recherche** (décidé par l'orchestration) ;
le lot 1 n'attend pas : ses points d'accroche ne demandent pas de flux.

Hors de ce chantier, dits pour ne pas y tomber : la boîte durable entre
processus (`MessageSourceNode`), les rôles attachés à une identité, les
gabarits `builtin/*`, la présence par curseurs, le décodage contraint.
