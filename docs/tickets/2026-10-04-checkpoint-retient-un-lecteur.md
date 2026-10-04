# Un CHECKPOINT retient la validation d'un lecteur jusqu'à son délai

- **État** : corrigé le 4 octobre 2026, commit « fix(transactions): un point de reprise n'attend plus en tenant le verrou public, et une écriture validée ne rend plus d'erreur »
- **Gravité** : blocage
- **Atteignable en service** : oui
- **Touche rag3weaver** : probable (lecteurs et points de reprise automatiques)
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : cœur C++ (transactions)

## Ce que c'est

Un CHECKPOINT garde le verrou que la validation d'un lecteur demande, et attend que ce lecteur parte. Le lecteur reste bloqué jusqu'au délai (5 s), puis le CHECKPOINT échoue. Un lecteur de 0,9 s seul dure 5,7 s.

## Recette minimale

```cypher
-- connexion 1 : une lecture d'environ une seconde en auto-commit
MATCH (a:P), (b:P) WHERE a.id + b.id = 7 RETURN count(*);   -- 20 000 nœuds P
-- connexion 2, pendant ce temps :
CHECKPOINT;   -- « Timeout waiting for active transactions … » après 5 s
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — CheckpointLetsAFinishingReaderGo

## Cause

`src/transaction/transaction_manager.cpp:60, 71-76, 108, 186` : le point de reprise attend en tenant `mtxForSerializingPublicFunctionCalls`.

## Correctif de l'amont (à lire, ne pas copier)

Vela `326d40dbd` (dans une fonctionnalité plus large : la rotation du journal).

## Pour le fermer

attendre les transactions sans tenir le verrou que leur validation demande ; le témoin passe au vert.
