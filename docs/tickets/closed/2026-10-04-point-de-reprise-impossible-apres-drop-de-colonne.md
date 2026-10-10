# Le point de reprise plante, à jamais, après ALTER TABLE … DROP

- **État** : corrigé le 4 octobre 2026, commit « fix(stockage): le point de reprise après ALTER TABLE … DROP d'une colonne indexe ses colonnes par leur position »
- **Gravité** : plantage
- **Atteignable en service** : oui
- **Touche rag3weaver** : non (rag3weaver ne supprime jamais de colonne)
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : cœur C++ (stockage)

## Ce que c'est

Après la suppression d'une colonne qui n'est pas la dernière, sur une table qui a des données sur disque, le CHECKPOINT tue le processus. Les données restent justes après réouverture, mais tout point de reprise suivant plante encore. Vrai pour les tables de nœuds et de relations.

## Recette minimale

```cypher
CALL auto_checkpoint=false;
CREATE NODE TABLE t(id INT64 PRIMARY KEY, a INT64, b STRING, c INT64);
CREATE (:t {id: 0, a: 1, b: 'x', c: 10});
CHECKPOINT;
ALTER TABLE t DROP b;
CREATE (:t {id: 1, a: 2, c: 20});
CHECKPOINT;   -- SIGSEGV ; après réouverture, CHECKPOINT plante encore
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — CheckpointAfterDroppingANodeColumn, CheckpointAfterDroppingARelationColumn

## Cause

Les tableaux de colonnes du point de reprise sont indexés par l'identifiant de colonne au lieu de la position : `src/storage/table/node_group.cpp:515-520`, `src/storage/table/csr_node_group.cpp:596, 608, 822`, `src/include/storage/table/csr_chunked_node_group.h:36-39`.

## Correctif de l'amont (à lire, ne pas copier)

Vela `e5e700e73`.

## Pour le fermer

indexer par la position dans la liste des colonnes du point de reprise ; les deux témoins passent au vert, y compris le point de reprise après réouverture.
