# Everything declarative — relevé de connaissances du 11 octobre 2026

Lu sur master `4bb017628` ; chemins relatifs à `extension/rag3weaver/`.

## Le bus et les boîtes (`src/events.rs`)

- `Event::Message { run, from, to, content, conversation }` ; `conversation`
  vide = fil dérivé de la paire, sinon fil nommé.
- `inbox_topic(run)` = `run.<run>.inbox` ; `send_in(run, from, to, content,
  conversation)` publie sur la boîte de `to` puis sur `messages` ;
  `broadcast` boucle sur une liste fournie par l'appelant (le bus ne connaît
  pas les fils).
- Un curseur ne voit que ce qui suit sa création : ouvrir la boîte du lancé
  **avant** de lui écrire. Adressage par run id seulement.

## L'agent (`src/agent.rs`)

- `with_events`, `with_name`, `with_run_id`, `with_inbox` ; `read_inbox` fait
  de chaque message un tour `user` « [message de X] … » et ignore le fil.
- `RunStarted { parent: None }` codé en dur dans `run()` ; pas de
  `with_parent`.
- Outils réservés : `pause_dialogue`, `confirm_pause` (`dialogue_tools()`,
  annoncés par `with_postures`, interceptés par nom, `handle_pause` prévient
  le pair par `send_message`) — le précédent de `send` et de `spawn`.
- La boucle finit quand le modèle rend du texte sans outil : un agent n'attend
  pas sa boîte ; un réacteur (`Reactor::on`) le réveille.
- Commande en fond : `is_async` → `spawn_async_tool` (fil *scoped* : la fin
  du run l'attend), poignée, fin postée dans la boîte ; `RunScope::defer`
  tue à la fin du run ; `ToolInvocation { handle, bus, inbox, scope }`.

## Graphes, outils, trace

- `GraphToolBox::call_with` pose `parent_run`, `tool_handle`, `agent_bus`,
  `agent_inbox`, `run_scope` dans les services du graphe.
- Aucun nœud ne fait tourner la boucle d'agent (`LlmNode` = un seul appel).
- Trace (`dataflow/trace_nodes.rs`) : `Run`, `Message`, `Conversation`,
  `Participant`, `CHILD_OF` (posé si `parent` est renseigné), `PERFORMED`,
  `PARTICIPATES_IN` ; `SendMessageNode` : destinataire en configuration, bus
  du catalogue, `from` = run du graphe.
- Le chat (`chat.rs`) : un `EventBus` privé par run ; `ObservedTools` et
  `CompletionTools` n'implémentent pas `call_with` (l'invocation se perd).

## Les e2e

Binaire unique `tests/e2e/main.rs` : créer `tests/e2e_<nom>.rs` puis
`python3 tests/e2e/generer.py` ; relancer un fils par
`crate::common::test_de_cette_suite!(..)` ; `cargo test --test e2e <suite>::`.
Les précédents à deux et trois agents : `e2e_agent_loop.rs`
(`two_agents_converse_through_their_inboxes`, c'est le test qui relaie) et
`e2e_conversation_a_plusieurs.rs` (vrai modèle).

## Qui possède quoi

`src/dataflow/` et la boucle d'agent : relus par la session recherche ;
`chat.rs`, le démon, le serveur MCP : l'arbre principal ; `declare.rs`,
`atelier.rs`, `serve`, les nœuds scriptés : cette session.

## Ce que le relevé de la boucle et du moteur a appris (master `d88ecfcbb`)

- **Pas de branchement dans le moteur de graphes** : un nœud qui ne pose pas
  une sortie laisse l'entrée **requise** en aval sans donnée, et le graphe
  entier finit en `deadlock: nodes [...] cannot execute` (`runtime.rs`, boucle
  d'exécution) ; une entrée optionnelle reliée laisse tourner le nœud sans
  elle ; un fan-in sur une entrée requise se contente d'une branche (`any`).
  Pas de `NodeStatus::Skipped` (`report.rs` : `Completed`, `Failed`,
  `Resumed`). Graphes strictement sans cycle (`topological_sort`).
- **Un port, une valeur par exécution** : les sorties sont rangées à la fin
  du nœud (`drain_outputs`) ; exécution par niveaux (`run_level` en fils de
  portée, `run_level_async` séquentiel).
- **Décision** : `Decider` (`decider.rs`, une probabilité par option en un
  jeton, `LlamaServerDecider`), service `"decider"` posé par le backend, aucun
  nœud client. `GenOptions::with_response_format` (`ResponseFormat::JsonSchema`)
  n'est honoré que par `OpenAiLlm`, et aucun appelant de production ne s'en
  sert. Pas de décodage contraint : `Choices::Fixed` = un `enum` dans le
  schéma de l'outil, puis `BadChoice` après coup.
- **Les points d'accroche de la boucle** (`agent.rs`) : `peer_in` dans
  `read_inbox` (les résultats d'outils en fond y arrivent aussi, `from =
  "outil #…"`) ; `model_out` après le rattrapage des appels écrits dans le
  texte ; `answer_out` dans la branche finale, à côté du contrôle de
  complétion ; `tool_call_out` autour de l'interception des outils réservés ;
  `tool_result_in` avant l'ajout du résultat ; `turn_end` en fin d'itération ;
  `user_in` est poussé par l'appelant avant `run`. Aucun `interrupt` sur
  message : seul le `TokenSink` coupe (`Flow::Stop`).
- Les tests de la boucle qui serviront de témoin de parité : voir la page 03,
  §4.2.

## Ce que le lot 1 a appris (ports d'agent, base `522fa6780`)

- Les points d'accroche **posés** sont décrits dans `src/agent_ports.rs` (la table
  des ports en tête du module). Le relevé ci-dessus disait « `user_in` est
  poussé par l'appelant avant `run` » : le port lit désormais le dernier tour
  `user`, une fois, avant la boucle.
- `ToolCall` est `Serialize`/`Deserialize` avec son champ opaque : un
  aller-retour JSON est exact, c'est ce qui rend `model_out` et
  `tool_call_out` transparents.
- `Finish.reason` ne suit pas `tool_calls` d'elle-même : qui retire les appels
  doit passer la raison à `Eos`, sinon le run finit en `Finished(ToolCall)`.
- Un outil en fond rend un accusé (par `tool_result_in`) et poste son vrai
  résultat dans la boîte sous l'émetteur `outil #<outil>-<n>` (la constante
  `ASYNC_RESULT_FROM` d'`agent.rs`). La dernière lecture de la boîte suit la
  jointure des fils de portée, si bien que le résultat est toujours dans
  l'historique, mais à une place qui dépend du moment où le fil a rendu.
- `GraphToolError::NotTaken { node, port, reason }` (recherche, `522fa6780`) :
  le nœud de résultat a été sauté. `NoResult` veut dire qu'il a tourné sans
  poser son résultat. Pour un port d'agent, les deux veulent dire « retenu ».
- Un nœud scripté rhai qui rend `#{}` ne pose aucune sortie (sortie
  `required: false`) : c'est la branche non prise la plus simple. `in` est un
  mot réservé de rhai : un port nommé `in` ne se lit pas par
  `input.inputs.in`.

## Ce que le lot 2 a appris (`Decide`, `Route`)

- Les ports d'un type de nœud se lisent sur le **schéma de sa fabrique**
  (`ports_declares`), qui ne voyait pas la configuration :
  `NodeFactory::schema_for(config)` existe pour les types dont les ports en
  dépendent. Le test de dérive (`le_schema_declare_les_memes_ports_que_le_noeud`)
  compare désormais `schema_for(minimum)` au nœud.
- Tout nœud intégré est aussi une **fiche d'outil** pour l'agent
  (`tool_defs(&nodes)` = intégrés + `SearchTool`) : ajouter un nœud change
  `BUILTIN_NODE_COUNT` (51 + 14 sous `code`).
- `node_config()` est ce que `to_definition` garde pour la reprise : un
  paramètre qui n'y est pas est perdu au rebâti.
- Les valeurs JSON passent entre nœuds en `PortType::Map` portant un
  `serde_json::Value` (comme les nœuds scriptés) ; la configuration mermaid ne
  connaît que des scalaires (`infer_scalar`) : une liste s'écrit `a|b`.
- `MockDecider` refuse lui-même un nombre de probabilités différent du nombre
  d'options : un témoin du compte passe par un `Decider` à soi.
- Après un `git reset`/rebase du worktree, `git submodule update` : sinon
  `codeparsers` reste à l'ancien pointeur et la feature `code` ne compile plus.

## Ce que la montre de fichiers a appris (W1-W2)

- Réindexer un fichier et le retirer existaient (`code_sync::reingest_file`,
  `remove_file`, la reprise par `note_change` pendant une indexation entière) :
  il ne manquait que qui les appelle.
- `Event::to_json` a un bras par variante : une variante neuve sans bras arrive
  aux graphes en `{"kind", "detail": "<Debug>"}` — l'ajouter.
- Les nœuds tournent dans les fils du runtime : un `thread_local` posé par
  l'appelant (`CELL_OF_CALL`) n'y est pas visible ; la cellule de l'appel leur
  arrive par le service `"scope"` que pose `build_services`.
- `poste lourd` ne transmet pas forcément l'environnement : les filtres du
  rejeu court se lisent dans un fichier (`I/court-filtres`).
- Sous le verrou de push, un rebase réécrit les fichiers du lot, donc
  rag3weaver recompile une fois par jeu de `check_features` : ~10 min même
  worktree chaud. La passe complète se fait avant le verrou.

## Ce que la nuit du 12 octobre a appris

- **Une seule instruction peut faire tout le temps.** La mesure par phase n'a servi qu'à la
  trouver ; ce qui l'a départagée, c'est une sonde jetable par instruction (lecture,
  suppression, relecture), puis une variante (le vecteur remis à NULL d'abord). La sonde est
  gardée en patch : `~/.cache/rag3weaver-build/I/sonde-suppression.patch`.
- **Le moteur (vector)** : supprimer une ligne indexée HNSW coûte des secondes dans la forme
  « index rempli par `SET` depuis NULL, suppressions répétées sur la même connexion » ; une
  ligne dont le vecteur est NULL se supprime en quelques millisecondes. Écrire 15 631 vecteurs
  sous index : 309 s (environ 20 ms chacun). Le banc ne reproduit pas encore (20 à 50 ms) ; sa
  piste est un coût qui croît d'un lot à l'autre depuis le dernier point de reprise de la base.
- **Deux instructions qui doivent aller ensemble** (vider puis supprimer) : la première laisse
  la ligne dans un état honnête si la seconde échoue. Vider le vecteur sans son marqueur
  rendait un morceau réputé embarqué, sans vecteur.
- **Un chronomètre global sous variable ne suffit pas à une ligne de production** : les
  durées des nœuds venaient d'un abonnement armé seulement sous la variable.
  `ingest_profile::record` (par fil) et `recording()` l'arment pour un seul travail.
- **Prouver la lib** : `build/lecteurs-csv` a été réécrite en place à 03:48:57, en pleine
  série de mesures. Les journaux datés ont montré que les quatre mesures « avant/après » sont
  toutes antérieures, et que la passe finale tournait sur `f6179ade0` (md5 de la lib et de
  l'extension contre le magasin).
- **Le push optimiste perd ses courses la nuit** : du code arrive sur master pendant chaque
  rejeu de 10 minutes. La fenêtre se prend par le verrou (`poste push … --lourd`) : rebase,
  rejeu et push dessous (`~/.cache/rag3weaver-build/I/scripts/sous-verrou.sh`), les docs
  passent quand même.

## Ce que le matin du 12 octobre a appris

- **Profiler avant de corriger, sur la même ligne que la production.** La ligne `[montre]`,
  attribuée par phase avec son « non détaillé », a désigné chaque fois le prochain poste : la
  suppression HNSW, puis « planifier », puis la lecture des mentions. Un agrégat qui contient
  d'autres étapes fausse le non détaillé : il se déclare (`AGREGATS`).
- **Un budget de tenue plutôt qu'une taille de lot fixe.** 32 fichiers tenaient le verrou plus
  d'une seconde, et une lecture attend au plus une seconde. Le budget (400 ms, d'après le coût du
  lot précédent) garde les lecteurs vivants. Sur la base entière, un fichier coûte déjà ~2 s : le
  lot et la lecture tirent en sens contraires, d'où les lectures sans attente de l'arbre principal.
- **Une lecture restreinte qui garde le même résultat.** Une mention ne devient une arête que si sa
  cible ou son mentionneur est dans le lot. On peut donc ne pas lire celles qui ne peuvent rien
  donner, à condition de garder ce que le choix de la cible lit (les réexportations).
- **`run_e2e.sh` ne garde que le dernier filtre** : une suite par appel, et lire « running N
  tests ».
- **La lib du magasin plutôt que le dossier de bâti** : un rebâti en place sous une passe donne
  des rouges d'environnement (un en-tête neuf, une lib ancienne).

## Ce que le 18 octobre a appris

- **Rejouer sous la charge qui a fait rougir, des deux côtés, avant d'accuser un lot.** Un rouge
  vu une fois sous charge se compte sur dix tours alternés, avec le parent du lot suspect. Si le
  parent rougit aussi, c'est le test ou le moteur.
- **Un test qui attend un signal (le retard tombé) au lieu de ce qu'il garde** a une course dès
  que le signal peut manquer (la montre ne note la réconciliation qu'à son premier tour). Attendre
  ce qu'on garde, dans le même délai, garde le rouge quand le défaut est vrai.
- **La vue de lecture n'a pas d'instantané par appel** : chaque requête est sa propre
  transaction, et `usages` en fait trois. Un rapport peut chevaucher un COMMIT. C'est l'objet du
  L6 de l'arbre principal.
- **Avant d'ajouter un paramètre à un outil, regarder ce que sa réponse porte déjà** :
  `search_code` rendait sa liste structurée dans `result` et son markdown dans `presentation`.
- **Une passe à distance doit survivre à une coupure** (`systemd-run --user --collect`), et son
  attente doit guetter aussi la mort de l'unité, pas seulement la ligne de fin : une attente qui ne
  lisait que FINI a perdu 90 minutes sur un script mort à la première ligne.
- **Un en-tête `equiv` de tiny_http exige un `&'static str`** ; et `git worktree remove` refuse un
  worktree qui porte des sous-modules (`rm -rf` puis `git worktree prune`).
- **luciepc n'a pas de PostgreSQL** : `RAG3WEAVER_PG=host=lucie-tablette port=5433 …` par Tailscale.

## Ce que la nuit du 19 octobre a appris

**Les composants et l'aperçu** (`src/components/`, `src/ide/apercu.html`) :

- Les ids de modules :
  - `comp:<Nom>` : le composant, ou son graphe de clic compilé s'il en déclare un ;
  - `vue:<Nom>` : sa vue écrite à la main, sous la famille React Refresh `<Nom>/vue`, importée par
    le spécifier réservé `rag3weaver:vue` ;
  - `lib:<paquet>@<version>/<fichier>`.
- Le registre (`modules::REGISTRE`) : `define` ne redéfinit un module que si son hash change ;
  `mettre_a_jour(frontieres)` remonte jusqu'aux modules qui n'exportent que des composants, ou
  rend `recharger`.
- L'aperçu tient `window.__apercu` (mises à jour, dernière, recharge, clics), que les témoins
  lisent.
- Les crochets d'un graphe de clic vers la page : `__rag3weaverClic(nom, allumes, tour)`,
  `__rag3weaverConfirmer`, `__rag3weaverNaviguer`.
- `EcranDuBackend.manifeste` est le chemin canonique que `/ide/call` reçoit.
- Les paquets React des témoins viennent de `scripts/paquets_react.sh`
  (`RAG3WEAVER_PAQUETS_REACT`) ; G réduit le magasin (`prune_store`).

**Les routes de l'IDE** (`src/ide.rs`) :

- `/ide/flux`, avec les sujets `files`, `dataflow`, `agent`, `tokens`, `catalog`, `components`,
  `reactions` ;
- `/ide/graphe?backend&tool|reaction|component` ;
- `/ide/reactions?backend` ;
- `/ide/apercu/<id>/<Comp>[/modules.js]` et `/ide/registre.js` ;
- `/ide/call`, `/ide/describe`, `/ide/etat`.

Les vues de `page.html` : arbre, fichier, symbole, graphe, recherche, `dag`, `reactions`.

**Les runs** :

- `RunStarted.target` (`events::GraphTarget`, la forme des arguments de l'outil `graph`) est posé
  pour un seul run par le service `run_target` `{run, target}`. Le posent `execute_plan` (l'appel
  d'outil) et `run_tool` du réacteur (`Reactor::named`).
- Les appels d'outil publient sur `dataflow` (le bus pris de la vue, jamais sous
  `catalog.lock()`). Les réactions publient sur `reactions` (`run_topic`), que
  `refuser_le_sujet_des_reactions` interdit d'écouter. Les ingestions publient sur `catalog`.
- `ReindexFileNode` s'arrête entre deux lots : quatre fichiers d'abord, puis ce qui tient en
  `HOLD_MS`.

**Les passes sur luciepc** (`ssh lucied@luciepc`, fish : `bash -c`) :

- worktree `~/git_workspaces/rag3db-I-apres`, target `target-I-apres` ;
- la lib épinglée par `I-court-reprise.sh` (`RAG3DB_LIB` 87a362e30) ;
- `l2/deux.sh` joue la lib deux fois et `check_features` ;
- chaque lot a son `I-pousse-<lot>.sh`, lancé par `systemd-run --user --collect`, et
  `poste push` refuse le code arrivé sur master depuis la base (« REJOUER ») ;
- le nettoyage du worktree doit couvrir `extension`, `docs` et `.github`.

**Les e2e à jouer selon ce qu'un lot touche** :

- `e2e_lectures` si `Backend::run`, l'hôte ou le bus ;
- `e2e_reaction_montee` si la publication des runs ;
- `e2e_postgres` si `code.rs`, l'IR ou un gabarit.
