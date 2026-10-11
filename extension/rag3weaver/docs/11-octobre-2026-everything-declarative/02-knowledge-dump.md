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
