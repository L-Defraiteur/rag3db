# L'UUID nul empêche tout point de reprise

- **État** : corrigé le 4 octobre 2026, commit `ed2f9d5db` (« fix(compression): l'UUID nul ne fait plus échouer le point de reprise »)
- **Gravité** : blocage
- **Atteignable en service** : oui
- **Touche rag3weaver** : non vérifié
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : banc (périmètre levé le 4 octobre)

## Ce que c'est

Une colonne UUID qui contient l'UUID nul fait échouer la compression : « cannot negate INT128_MIN ». Aucun point de reprise ne passe plus, même après réouverture ; le journal grossit.

## Recette minimale

```cypher
CALL auto_checkpoint=false;
CREATE NODE TABLE T(id INT64 PRIMARY KEY, u UUID);
CREATE (:T {id: 1, u: UUID('00000000-0000-0000-0000-000000000000')}),
       (:T {id: 2, u: UUID('00000000-0000-0000-0000-000000000001')});
CHECKPOINT;   -- « INT128 is out of range: cannot negate INT128_MIN »
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — CheckpointOfAColumnHoldingTheNilUuid

## Cause

`src/storage/compression/compression.cpp:563-566` prend la valeur absolue de INT128_MIN ; l'UUID nul est stocké ainsi (`src/common/types/uuid.cpp:64`).

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `231a0b854` (30 juin 2026).

## Pour le fermer

le témoin passe au vert, avant et après réouverture.
