# L'annulation d'un COPY efface les lignes qu'un autre écrivain valide ensuite

- **État** : ouvert, pour la marche A3′ (câblage des verrous, stèle étape 5).
- **Gravité** : perte (une validation annoncée réussie dont les lignes disparaissent), et
  réponse fausse (une clé en double acceptée ensuite).
- **Atteignable en service** : non. Il faut le mode multi-écrivains
  (`debug_enable_multi_writes`), éteint hors du banc.
- **Touche rag3weaver** : non tant que le mode multi-écrivains reste éteint.

## Ce que c'est

Deux écrivains, chacun dans sa transaction, font un COPY dans la même table de nœuds. Le
COPY écrit directement dans les blocs de la table, et les deux partagent le dernier bloc.
Si celui qui a copié **le premier** annule, son annulation efface aussi les lignes de
l'autre et retire leurs clés de l'index de clé primaire. L'autre valide ensuite sans
erreur. Après sa validation, la table est vide, et une ligne de même clé est acceptée.
Dans l'autre ordre (celui qui annule a copié en second), tout est juste.

C'est plus que la limite écrite le 4 octobre (`5c8507577`, `node_table.cpp`,
`RollbackPKDeleter`), qui ne parlait que des clés.

## Recette minimale

Mode multi-écrivains, point de reprise automatique coupé, deux connexions A et B :

```
CREATE NODE TABLE Item(id INT64 PRIMARY KEY, v INT64);
B: BEGIN TRANSACTION;   A: BEGIN TRANSACTION;
A: COPY Item FROM 'cles-1000-a-1099.csv';
B: COPY Item FROM 'cles-0-a-99.csv';
A: ROLLBACK;
B: COMMIT;              -- réussit
MATCH (n:Item) RETURN count(n);          -- 0, attendu 100
CREATE (:Item {id: 50, v: -1});          -- accepté
```

Déterministe : 3 passes sur 3, le 5 octobre.

## Témoin

`test/transaction/concurrence/lock_bench_test.cpp` :

- `Order/RollbackOfACopy.RemovesOnlyItsOwnKeys/RolledBackCopiedFirst`, rouge
  (`committed-rows`, `other-writer-keys-kept`, `other-writer-key-unique`,
  `primary-key-index-agrees`) ; sa variante `RolledBackCopiedSecond` est verte ;
- `LockBench.TwoCommitsThatEachWantACheckpointDoNotWaitForTheTimeout`, rouge, qui y retombe.
  C'est la seconde limite écrite le 4 octobre (`68ff9d5e2`) : deux validations de COPY qui
  veulent chacune leur point de reprise. La première attend le départ de l'autre jusqu'au
  délai (5 s) et échoue. Son annulation efface alors les lignes de la seconde, qui a
  pourtant validé.

Les deux sont dans `known_red.txt`, sous A3′.

Deux conséquences de plus, depuis le 5 octobre, sans témoin propre :

- le versement des lignes locales avant un COPY (`f1d8c7190`) écrit lui aussi directement dans
  les blocs : des insertions ordinaires suivies d'un COPY y tombent de même ;
- sur une table à index vectoriel, le crochet d'annulation (`e1049934e`) ramène le compte des
  lignes reliées au premier décalage annulé, sous les lignes de l'autre écrivain : elles
  seraient reliées une seconde fois à la fin du COPY suivant (relevé par la session cœur
  C++) ;
- par lecture, le 5 octobre (la chasse aux pointeurs gardés à travers une réallocation) :
  `NodeGroupCollection` prend `lastNodeGroup` sous le verrou, le relâche, puis s'en sert
  encore. Entre-temps, l'annulation d'un autre écrivain (`removeTrailingGroups`) peut effacer
  ce groupe s'il est encore vide (`node_group_collection.cpp`, ~121-153 et 228).

## Cause

Pas cherchée au-delà du constat. L'annulation d'un COPY défait les insertions du bloc à
partir de son début (`ChunkedNodeGroup::rollbackInsert`,
`VectorVersionInfo::rollbackInsertions`, `RollbackPKDeleter`). Rien n'y distingue les lignes
d'un autre écrivain ajoutées après. **[déduit]**

## Pour le fermer

Il faut que l'annulation ne touche qu'aux lignes de sa transaction, ou que deux COPY d'une
même table ne puissent plus se croiser (verrou de table pour le COPY, à décider avec A3′).
Le chargement journalisé (stèle, étape 4) change aussi ce terrain : à regarder avec lui.
Les deux témoins passent alors au vert, et leurs lignes sortent de `known_red.txt` dans le
commit qui corrige.
