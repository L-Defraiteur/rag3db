# Un groupe vidé par le point de reprise garde la colonne supprimée

- **État** : ouvert, **à vérifier par un témoin** (lu dans le code le 19 octobre 2026, carte des
  numéros de colonnes pour V2-1a)
- **Gravité** : si confirmé, un bloc sur disque dont les colonnes ne correspondent plus aux
  types du groupe : lecture fausse ou assertion à la réouverture, au prochain ADD ou au prochain
  balayage du groupe.
- **Atteignable en service** : oui si confirmé : supprimer toutes les lignes d'un groupe et une
  propriété entre deux points de reprise.
- **Touche rag3weaver** : oui si confirmé.
- **Ouvert le** : 19 octobre 2026, session cœur C++.
- **Pour** : cœur C++, lot V2-1a (la forme à trous retire la question).

## Ce que c'est

Dans `NodeGroup::checkpoint` (`src/storage/table/node_group.cpp:458-486`), un groupe dont
toutes les lignes sont supprimées est vidé par `flushEmpty(…, dataTypes, …)` (:466-472) avec
`dataTypes` **non filtré** (toutes les colonnes, supprimées comprises), puis `dataTypes` est
filtré par `checkpointDataTypesNoLock` (:485). Le bloc vide écrit sur disque a une colonne de
trop ; son format est un vecteur dense sans numéros (`chunked_node_group.cpp:631-663`). À la
relecture, les types du groupe (filtrés) et les colonnes du bloc (non filtrées) sont décalés.

## Recette (lue, pas jouée)

```cypher
CREATE NODE TABLE T(id INT64 PRIMARY KEY, a INT64, b STRING);
UNWIND range(1, 10) AS i CREATE (:T {id: i, a: i, b: 'x'});
CHECKPOINT;
MATCH (t:T) DELETE t;
ALTER TABLE T DROP a;
CHECKPOINT;
// réouvrir, puis :
CREATE (:T {id: 11, b: 'y'});
MATCH (t:T) RETURN t.id, t.b;   // attendu 11, y
```

## Témoin

À écrire dans `test/transaction/` : la recette, jouée avec réouverture, sous ASAN. S'il est vert,
fermer ce ticket en disant pourquoi le code lu n'y mène pas.

## Pour le fermer

Le témoin, puis soit le correctif (vider avec les types filtrés), soit la forme à trous de
V2-1a où un bloc vide porte toutes les colonnes numérotées, trous compris.
