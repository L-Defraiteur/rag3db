# Un COMMIT qui écrit sous un index vectoriel saute des tables de nœuds

- **État** : ouvert (13 novembre 2026, cœur c++) — corrigé par le lot qui porte ce ticket (branche
  `insertion-defaite-reouverture`), à fermer après la lib et le rejeu d'arbre principal
- **Famille** : moteur
- **Gravité** : perte — un COMMIT qui rend succès ne publie pas les nœuds de certaines tables ;
  leurs relations, publiées, restent orphelines
- **Atteignable en service** : oui. Toute transaction qui écrit sous un index vectoriel et touche
  assez de tables pour qu'une table de hachage se réorganise : l'ingestion de rag3weaver en
  transaction par paquets (`RAG3WEAVER_TX_PAR_PAQUET=1`, quatre paquets par validation) en est une.
- **Touche rag3weaver** : par le moteur (l'ingestion par paquets, le lot 1a des vues)
- **Signalé par** : arbre principal (le repreneur de `e2e_tx_vecteurs_apres_rollback`), réduit par
  cœur c++
- **Pour** : cœur c++

## Ce qu'on voyait

Un second processus, seul écrivain, refusait un `SET` sur des relations
`ScopeContent_DERIVED_FROM` : « could not serialize access due to concurrent update — row 128 of
table ScopeContent was deleted by another transaction after this one started ». Sa transaction
précédente avait validé, avec succès, 128 `ScopeContent` et 128 `Scope` neufs avec leurs relations.
Juste après ce COMMIT, sa propre connexion ne voyait plus que les 128 d'avant : les nœuds n'étaient
pas publiés, les relations l'étaient. Le SET suivant, parti de relations vers des nœuds absents,
cherchait à verrouiller la ligne 128 et concluait à tort à une suppression concurrente.

## La cause

`LocalStorage::commit` (`src/storage/local_storage/local_storage.cpp`) parcourait par un `for` de
portée la `std::unordered_map` de ses tables locales, et y validait chaque table de nœuds. Valider
une table sous index vectoriel insère ses lignes dans l'index HNSW, qui pose des arêtes dans ses
tables de relations internes par `RelTable::insert` ; celui-ci crée leur table locale par
`getOrCreateLocalTable`, dans la même table de hachage, en plein parcours. Quand cet ajout la
réorganisait, les itérateurs étaient invalidés (comportement indéfini) : le parcours sautait des
tables de nœuds, leurs lignes n'étaient jamais publiées, et le COMMIT rendait succès. Les tables de
relations, parcourues ensuite, étaient publiées.

Le paquet défait du premier processus n'y était pour rien : il faisait seulement coïncider les
places, si bien que des nœuds neufs reprenaient plus tard ces places et ramenaient les relations
orphelines.

## Comment on l'a trouvé

- La base gardée par arbre principal, sondée : 128 relations orphelines, aucune après le premier
  processus seul.
- Le second processus observé de l'intérieur par une bibliothèque `LD_PRELOAD` qui intercepte
  `Connection::query`, `prepare` et `executeWithParams` : 256 nœuds avant le COMMIT, 128 après.
- Ses 787 instructions capturées avec leurs paramètres en littéraux, rejouées en C++ sur une copie,
  puis réduites par ddmin à 15 : un `SET` de vecteurs sous index HNSW et une écriture dans une autre
  table, dans la même transaction, sont tous deux nécessaires (`~/.cache/rag3db-moteur-notes/
  insertion-defaite/` sur le portable).

## Le correctif et le témoin

- Les tables de nœuds sont relevées avant de valider la première, puis les tables de relations,
  celles que les index ont créées en route comprises. Une garde de debug refuse la création d'une
  table locale de nœuds pendant cette validation.
- Témoin : `CommitLocalTablesTest.ACommitWithAVectorIndexKeepsTheRowsOfEveryTable`
  (`test/transaction/commit_local_tables_test.cpp`) : une transaction insère une ligne dans k tables
  de nœuds et une ligne sous index HNSW, puis valide, pour k de 1 à 40. Rouge sur master à luciepc
  (dès k = 12, T0 et T1 perdent leur ligne, 86 pertes) ; vert avec le correctif.
- Les variantes écartées en chemin restent comme gardes : `RolledBackInsertReopenTest`
  (`test/transaction/rolled_back_insert_reopen_test.cpp`).

## La portée

La sonde de hotfixs (`docs/coeur-cpp-hotfixs-tickets/13-11-2026/01-sonde-des-bases-de-lucie.md`) ne
trouve aucune orpheline dans les copies de code-lourd et de code-principal : l'ingestion par défaut
y valide plus petit, sans franchir le seuil. Une base ingérée en transaction par paquets sous une lib
d'avant ce correctif peut porter des nœuds perdus et des relations orphelines.

## Pour le fermer

Le correctif sur master, la lib bâtie par G, et le repreneur d'arbre principal vert dessus.
