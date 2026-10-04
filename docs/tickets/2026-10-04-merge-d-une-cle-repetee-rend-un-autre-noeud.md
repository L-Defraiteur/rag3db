# MERGE d'une clé répétée rend une autre valeur que celle stockée

- **État** : corrigé le 4 octobre 2026, commit « fix(merge): un motif retrouvé dans le lot repose l'identifiant créé, pour les ON MATCH SET comme pour le SET qui suit »
- **Gravité** : réponse fausse
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui, par `batch_link` (voir plus bas)
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : exécuteur — à attribuer

## Ce que c'est

Quand la même clé apparaît deux fois dans un lot de MERGE, la seconde ligne rend une valeur par défaut réévaluée (un UUID jamais stocké), ou l'identifiant d'un autre nœud.

## Recette minimale

```cypher
CREATE NODE TABLE A(id STRING DEFAULT gen_random_uuid(), stuff INT64, PRIMARY KEY(id));
UNWIND [1, 1] AS i MERGE (a:A {stuff: i}) RETURN a.id;   -- deux UUID, un seul stocké
CREATE NODE TABLE B(id SERIAL, stuff INT64, PRIMARY KEY(id));
UNWIND [1, 2, 1] AS i MERGE (b:B {stuff: i}) RETURN i, b.id;   -- la 3e ligne rend id 2, qui n'existe pas
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — MergeOfARepeatedKeyReturnsTheStoredNode

## Cause

`src/processor/operator/persistent/insert_executor.cpp:109-115`, `src/processor/operator/persistent/merge.cpp:74-78` : `skipInsert()` réévalue les valeurs par défaut et ne repose pas l'identifiant du nœud trouvé.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `c5a3c7385` (25 mai 2026). Ne reprendre que la relecture par identifiant ; leur correctif change aussi le nombre de lignes rendues.

## Les formes de rag3weaver (sonde du 4 octobre)

Les quatre formes que rag3weaver fabrique par `dialect::unwind_par_cle`, jouées avec une
clé répétée dans le lot :

- `kb_upsert_index` (MERGE de nœud, ON CREATE SET / ON MATCH SET à plusieurs champs) : juste.
- `batch_upsert` (MERGE de nœud, SET, RETURN ID(n)) : juste, identifiants compris.
- blobs et `upsert_aside` (MERGE de nœud, SET) : justes.
- **`batch_link` (MATCH des deux bouts, MERGE de la relation, SET) : faux.** Quand un lot
  porte deux fois la même relation, créée par le lot lui-même, avec une autre relation créée
  entre les deux, le SET de la seconde occurrence est perdu, sans erreur. Adjacents, ou sur
  une relation qui existait déjà, les doublons sont justes ; aucune autre relation n'est
  touchée. Témoin : `MergeOfARelationRepeatedInTheBatchKeepsTheLastSet`.

```cypher
UNWIND [{from_uuid: 'a', to_uuid: 'b', t: 'x', u: 1},
        {from_uuid: 'a', to_uuid: 'c', t: 'w', u: 5},
        {from_uuid: 'a', to_uuid: 'b', t: 'y', u: 2}] AS item
WITH item, item.from_uuid AS __cle_0 MATCH (a:A {_uuid: __cle_0})
WITH a, item, item.to_uuid AS __cle_1 MATCH (b:B {_uuid: __cle_1})
MERGE (a)-[r:R]->(b) SET r.t = item.t, r.u = item.u;
-- a->b garde x|1 au lieu de y|2
```

Ce qu'on sait du mécanisme :
- l'existence du motif est calculée avant que le MERGE n'écrive ;
- le doublon passe donc par `executeOnCreatedPattern`, où `RelInsertExecutor::skipInsert`
  ne repose pas l'identifiant de la relation retrouvée ;
- la table des motifs créés ne garde d'identifiants que pour les `ON MATCH SET`, pas pour
  un SET séparé.

Le doublon adjacent est juste parce que le vecteur garde l'identifiant de la ligne
précédente, la même relation. Ce qu'on ne sait pas : pourquoi, entre deux doublons non
adjacents, la mise à jour ne tombe pas sur la relation précédente (a->c reste w|5). Arrêté
là le 4 octobre : un correctif sans ce point compris n'est pas « facile ».

## Pour le fermer

le témoin passe au vert.
