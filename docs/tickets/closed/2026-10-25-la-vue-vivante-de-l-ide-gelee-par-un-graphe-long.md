# La vue vivante de l'IDE gelée par un graphe long

- **État** : corrigé le 25 octobre 2026 (session « everything declarative ») : chaque relais du
  flux de l'IDE a son propre fil, en réception bloquante, hors du runtime des graphes. Ouvert et
  fermé le même jour.
- **Gravité** : la vue vivante se fige. Les runs, le chat, les jetons et les propositions n'arrivent
  plus à la page tant que des graphes tiennent les fils, alors que l'hôte les a publiés.
- **Atteignable en service** : oui. Il suffit de deux nœuds qui bloquent en même temps (une
  attente, un sommeil, un modèle lent), dans n'importe quel backend de l'hôte.
- **Touche rag3weaver** : oui (`Flux::relayer`, `src/ide.rs`).

## Ce que c'était

`Flux::relayer` relayait le bus de chaque backend vers les fenêtres de l'IDE par des tâches du
runtime tokio du crate (`dataflow::rt::runtime()`). Ce runtime n'a que deux fils pour tout le
processus, et les nœuds des graphes qui bloquent les y tiennent. Pendant ce temps, rien n'allait du
bus à la page : le tour du chat publiait `RunStarted` et ses jetons, et la page restait sur
« l'agent travaille… », sans Arrêter.

Vu par trois rouges du témoin `le_chat_se_mene_et_s_arrete_dans_la_page`, sous la lib entière
seulement, à trois pas différents (tickets
`2026-10-25-le-temoin-du-chat-dans-la-page-rougit-sous-la-lib-entiere` et
`2026-10-25-le-temoin-du-chat-rougit-encore-sur-le-tour-lent`). Dans la lib entière, les graphes
des autres témoins du même processus tenaient les deux fils.

## La recette

Le témoin K2 seul, avec les deux fils du runtime tenus 25 s pendant le tour lent (une sonde) :
3 rouges sur 3, au même pas, avec la même vue que les rouges de la lib entière. Le tour du chat
lui-même ne passe pas par ce runtime.

## Le témoin

- `ide::tests::the_relay_does_not_wait_for_the_graph_runtime` : les deux fils du runtime tenus
  1,5 s ; un événement du bus arrive à la fenêtre en moins de 500 ms.
- La sonde des 25 s, rejouée sur le correctif.
- `le_chat_se_mene_et_s_arrete_dans_la_page`, qui attend désormais l'hôte (le tour en vol, puis
  arrêté) avant de regarder la page.
