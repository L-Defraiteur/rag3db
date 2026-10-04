# MERGE avec deux ON MATCH SET et une clé répétée plante

- **État** : corrigé le 4 octobre 2026, commit `f19977332` (« fix(merge): un motif retrouvé dans le lot repose l'identifiant créé, pour les ON MATCH SET comme pour le SET qui suit »)
- **Gravité** : plantage
- **Atteignable en service** : oui
- **Touche rag3weaver** : non d'après la sonde du 4 octobre (`kb_upsert_index`, la seule forme à ON MATCH SET multiple, est juste avec une clé répétée : la clé y est extraite dans un WITH)
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : exécuteur — à attribuer

## Ce que c'est

Un MERGE de relation avec deux ON MATCH SET, quand la même clé apparaît deux fois dans le lot, lit un identifiant non initialisé et tue le processus.

## Recette minimale

```cypher
CREATE NODE TABLE A(id STRING PRIMARY KEY);
CREATE NODE TABLE B(id STRING PRIMARY KEY);
CREATE REL TABLE R(FROM A TO B, t STRING, u INT64);
CREATE (:A {id: 'a'}), (:B {id: 'b'});
UNWIND [{s: 'a', t: 'b', v: 'x', n: 1}, {s: 'a', t: 'b', v: 'y', n: 2}] AS row
MATCH (s:A {id: row.s}) MATCH (t:B {id: row.t})
MERGE (s)-[r:R]->(t) ON CREATE SET r.t = row.v, r.u = row.n ON MATCH SET r.t = row.v, r.u = row.n;
-- SIGSEGV ; attendu : une relation, r.u = 2
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — MergeWithTwoOnMatchSetsAndARepeatedKey

## Cause

`src/processor/map/map_merge.cpp:56, 66`, `src/processor/result/pattern_creation_info_table.cpp:11` : la colonne d'identifiant du second exécuteur de SET n'est pas remplie.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `a0063158d` (1er septembre 2026).

## Pour le fermer

le témoin passe au vert.
