# Un graphe exécuté par niveaux ne s'annule pas

- **État** : corrigé (12 octobre 2026, session recherche : un hub d'annulation par run par niveaux, page `extension/rag3weaver/docs/12-octobre-2026-recherche/05-l-annulation-sous-tous-les-executeurs.md`)
- **Gravité** : blocage. Une annulation demandée n'arrête rien tant que le graphe n'a pas fini
- **Atteignable en service** : oui. Tout graphe sans flux branché passe par l'exécuteur par niveaux, et une poignée d'annulation prise dessus ne fait rien
- **Touche rag3weaver** : oui (`src/dataflow`)
- **Ouvert le** : 12 octobre 2026, session recherche, pendant la conception du lancement en fond (`extension/rag3weaver/docs/12-octobre-2026-recherche/03-lancer-en-fond.md`, §2.1)

## Ce que c'est

`DataflowRuntime::cancel_handle()` et `NodeContext::cancellation()` n'agissent que sous
l'exécuteur concurrent. Sous les niveaux, le `CancelToken` d'un nœud n'est jamais armé
(`node.rs`, `CancelToken`), et le runtime ne regarde pas la poignée entre deux nœuds. Un
graphe de valeurs dont on demande l'arrêt va donc jusqu'au bout.

## Recette minimale

Pas de Cypher : c'est l'exécuteur. Un graphe de deux nœuds sans flux (le second dort une
seconde). On prend `cancel_handle()` avant `execute`, on annule depuis un autre fil après
100 ms : le run finit quand même, en une seconde, `stopped()` vide.

## Le témoin

Il n'existe pas encore. Il s'écrit dans `src/dataflow/concurrent/tests.rs` ou dans
`flux_tests.rs`, sur la recette ci-dessus : la sortie doit être `stopped() == Some(External)`,
et le second nœud ne doit pas avoir fini.

## La cause

L'annulation est née avec l'exécuteur concurrent (le lot 0 des flux). Le chemin par niveaux
d'avant n'avait pas de poignée.

## Le contournement en place

Un run lancé en fond passe toujours par l'exécuteur concurrent (page 03, §2.1). Il est donc
arrêtable. Les autres runs ne le sont pas.

## Ce qu'il faut pour le fermer

Dans les deux boucles par niveaux de `runtime.rs` :
- la poignée relue avant chaque nœud ;
- le `CancelToken` du contexte armé sur elle ;
- à l'annulation, la sortie rendue avec `stopped()` et ses nœuds non faits.

La parité des graphes de valeurs sans annulation doit être gardée : la lib entière est le
témoin.

## Le correctif

Un run par niveaux crée un hub d'annulation (sans canal ni ouvrier), attaché à
la poignée d'arrêt le temps du run. Les nœuds le reçoivent par leur jeton
(`NodeContext::cancellation`), et la boucle le regarde en tête de chaque
niveau. Annulé, le run rend `Ok`, avec `stopped()` posé ; les nœuds non
lancés sont dits arrêtés (`NodeCancelled`), et `RunFinished.cause` vaut
`requested`. Témoins : `src/dataflow/annulation_tests.rs`, rouges sur le code
d'avant.
