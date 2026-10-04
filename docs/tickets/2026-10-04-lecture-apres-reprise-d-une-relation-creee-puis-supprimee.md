# La lecture plante après le point de reprise d'une relation créée puis supprimée

- **État** : corrigé le 4 octobre 2026, commit « fix(stockage): un point de reprise ne libère plus les relations des régions qu'il n'a pas réécrites »
- **Gravité** : plantage
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (il crée et supprime des relations)
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : cœur C++ (stockage)

## Ce que c'est

Une relation créée puis supprimée depuis le dernier point de reprise : le point de reprise suivant passe, puis la lecture de la table tue le processus. Un redémarrage remet les choses en ordre.

## Recette minimale

```cypher
CALL auto_checkpoint=false;
CREATE NODE TABLE N(id INT64 PRIMARY KEY);
CREATE REL TABLE R(FROM N TO N, k INT64);
CREATE (:N {id: 1}), (:N {id: 2});
MATCH (a:N {id: 1}), (b:N {id: 2}) CREATE (a)-[:R {k: 1}]->(b);
CHECKPOINT;
MATCH (a:N {id: 1}), (b:N {id: 2}) CREATE (a)-[:R {k: 2}]->(b);
MATCH ()-[r:R]->() WHERE r.k = 2 DELETE r;
CHECKPOINT;
MATCH (:N {id: 1})-[r:R]->() RETURN r.k;   -- SIGSEGV
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — ReadAfterCheckpointOfACreatedThenDeletedRelation

## Cause

`src/storage/table/csr_node_group.cpp:517-528` : quand aucune région n'est à réécrire, le point de reprise sort sans finaliser l'en-tête CSR ; les sentinelles laissées par la suppression restent, et la lecture suivante les suit.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `ba5f38815` (22 juin 2026).

## Pour le fermer

finaliser l'état du groupe aussi sur la sortie anticipée ; le témoin passe au vert.
