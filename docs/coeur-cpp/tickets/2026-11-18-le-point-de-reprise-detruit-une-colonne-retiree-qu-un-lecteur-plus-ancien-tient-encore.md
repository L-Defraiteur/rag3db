# Le point de reprise détruit une colonne retirée qu'un lecteur plus ancien tient encore

- **État** : ouvert (18 novembre 2026, cœur c++) — lu dans le code, pas encore vu
- **Famille** : moteur
- **Gravité** : plantage possible (une colonne détruite sous un lecteur), à établir
- **Atteignable en service** : à établir. Il faut un ALTER TABLE … DROP validé, un lecteur dont
  l'instantané est plus ancien que ce DROP, et un point de reprise pendant que ce lecteur balaie.
- **Touche rag3weaver** : par le moteur
- **Signalé par** : cœur c++, en écrivant le correctif de l'ADD COLUMN sous des lecteurs
  (branche `add-column-under-readers`)
- **Pour** : cœur c++

## Ce qu'on lit

À la fin de `NodeTable::checkpoint` (`src/storage/table/node_table.cpp`), et de même dans
`RelTableData::checkpoint`, chaque colonne dont la propriété n'est plus au catalogue est détruite :
`columns[columnID].reset()`. Un lecteur dont l'instantané précède le DROP voit encore la propriété,
et a pu lier son balayage à cette colonne : `NodeTableScanState::setToTable` garde des `Column*`.
Depuis V2-1c, un lecteur n'attend plus le point de reprise. Rien n'empêche donc la destruction
pendant qu'il balaie.

Le correctif de l'ADD COLUMN sous des lecteurs (le verrou partagé sur le vecteur des colonnes) rend
sûre la lecture du vecteur. Il ne retient pas la colonne elle-même : le pointeur que le lecteur a
gardé peut viser une colonne détruite.

## Ce qui reste à faire

- Un témoin : un lecteur ouvert avant un DROP, qui balaie en boucle ; le DROP validé ; un CHECKPOINT ;
  sous TSan ou ASan, une lecture de la colonne détruite.
- La forme : ne détruire une colonne retirée qu'au point de reprise où plus aucun lecteur ne tient un
  instantané d'avant son DROP (l'horodatage du plus vieux lecteur, `oldestReaderTS`, est déjà passé au
  point de reprise) ; ou des colonnes partagées (`std::shared_ptr<Column>`), retenues par l'état de
  balayage.

## Pour fermer

Le témoin rouge puis vert, `transaction_test`, known_red et `e2e_test` entiers.
