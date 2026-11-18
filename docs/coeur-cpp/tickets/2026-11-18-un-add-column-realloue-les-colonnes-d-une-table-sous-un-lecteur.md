# Un ADD COLUMN réalloue les colonnes d'une table sous un lecteur

- **État** : ouvert (18 novembre 2026, cœur c++)
- **Famille** : moteur
- **Gravité** : plantage possible (une lecture après libération), vu par TSan, pas encore en Release
- **Atteignable en service** : oui, pour un ALTER TABLE … ADD posé pendant qu'une requête lit la table
- **Touche rag3weaver** : par le moteur (les montées de schéma de l'hôte)
- **Signalé par** : cœur c++. La passe TSan du 16 novembre sur le dictionnaire
  (`~/.cache/rag3db-moteur-notes/dict-tsan/tsan-5.*` à luciepc).
- **Pour** : cœur c++

## Ce qu'on voit

TSan, `TwentyWriters.DdlEveryMinute` sous 200 Mio, sur master `64eee2e1f`, tour 5 :

```
Write of size 8 by thread T4 : operator delete ← std::vector<std::unique_ptr<Column>>
  (la réallocation du vecteur, dans NodeTable::addColumn, src/storage/table/node_table.cpp)
Previous read of size 8 by thread T8 : std::unique_ptr<Column>::get
  ← NodeTable::getColumn (src/include/storage/table/node_table.h)
  ← NodeTableScanState::setToTable ← ScanNodeTableInfo::initScanState
```

`NodeTable::addColumn` fait `columns.push_back(...)` : le vecteur peut se réallouer et libérer son
ancien tampon. Un lecteur qui prépare son balayage lit `columns[i]` au même moment. Les lecteurs ne
prennent pas le verrou de schéma de la table, qui n'exclut que les écrivains (lot du 15 novembre,
`101ff50a6`). Un lecteur peut donc lire un pointeur de colonne dans un tampon libéré.

## Ce qui reste à faire

- Un témoin : un ADD COLUMN répété pendant que des lecteurs balaient la table, sous TSan, ou en Release
  avec le crochet `TestHooks::setPoisonFreshBuffers` de la branche `dictionary-index-after-string-2`.
- La forme du correctif, à choisir : réserver le vecteur `columns` à sa taille maximale ; une liste de
  colonnes publiée d'un coup (copie puis échange atomique) ; ou un verrou de lecture des colonnes. Une
  règle des moteurs établis : PostgreSQL fait prendre aux lecteurs un verrou de table partagé (ACCESS
  SHARE) que l'ALTER (ACCESS EXCLUSIVE) attend.

## Pour fermer

Le témoin rouge puis vert, `transaction_test`, known_red et `e2e_test` entiers.
