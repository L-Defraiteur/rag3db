# Un ADD COLUMN réalloue les colonnes d'une table sous un lecteur

- **État** : corrigé sur master `d6fb388ad9` (19 novembre 2026, cœur c++), atterri dans la pile `9906f44d69`
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

## Fermé (19 novembre 2026)

Le vecteur des colonnes de `NodeTable` et de `RelTableData` se lit sous un verrou partagé
(`columnsMtx`) et ne change que sous un verrou exclusif : l'ajout, son annulation, la remise à zéro du
point de reprise. Le témoin `AddColumnUnderReaders.ReadersScanWhileColumnsAreAdded`
(`concurrence_test`) : sous TSan, 13 lignes de rapport sur les colonnes en 3 tours avant le correctif
(`60b21c753`), 0 après (`f4dea9df5`), journal `~/.cache/rag3db-moteur-notes/columns-race/` à luciepc.
Reposé (`d6fb388ad9`) et poussé avec la pile C++ du 19 novembre (tête `9906f44d69`) : passe de la tête
verte, `transaction_test` 399, known_red, `e2e_test` 1 867 avec `DropNodeTableReclaim` admis.

La forme de fond, que les lecteurs prennent le verrou de table et qu'un ALTER les attende, reste au
ticket `2026-11-18-un-alter-ou-un-drop-n-attend-pas-les-lecteurs-de-sa-table.md`.
