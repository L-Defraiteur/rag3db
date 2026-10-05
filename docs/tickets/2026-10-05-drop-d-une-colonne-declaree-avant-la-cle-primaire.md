# Retirer une colonne déclarée avant la clé primaire casse l'insertion

- **État** : ouvert
- **Gravité** : réponse fausse et perte, plus que le blocage d'abord vu. Une ligne acceptée reste introuvable par sa clé, la mauvaise propriété d'une relation est écrite, et deux cas plantent (SIGSEGV). Mesuré au banc le 5 octobre, voir « Étendue »
- **Atteignable en service** : oui — un `ALTER TABLE … DROP` d'une colonne déclarée avant la clé primaire
- **Touche rag3weaver** : non (il ne retire jamais de colonne ; ses clés sont déclarées en premier)
- **Ouvert le** : 5 octobre 2026, session cœur C++ (en écrivant les témoins du chargement journalisé après un `DROP`)
- **Pour** : cœur C++

## Ce que c'est

Les lignes d'une insertion ordinaire voyagent comme la liste des propriétés du catalogue, dans leur ordre. Plusieurs endroits y cherchent la clé primaire au **numéro de sa colonne dans le stockage**. Les deux coïncident tant qu'aucune colonne déclarée avant la clé n'a été retirée ; après, la clé est lue une position trop loin.

## Recette minimale

```cypher
CREATE NODE TABLE Item(extra STRING, id INT64, name STRING, PRIMARY KEY (id));
ALTER TABLE Item DROP extra;
UNWIND range(0, 499) AS i CREATE (:Item {id: i, name: 'item ' + CAST(i AS STRING)});
-- Runtime exception: Found duplicated primary key value item 2, which violates the
-- uniqueness constraint of the primary key column.
```

Exécuté le 5 octobre 2026 sur le moteur de la branche de l'étape 4 (un brouillon de témoin, depuis recentré) : la valeur citée dans l'erreur est celle de `name`, pas de `id`. Avec une colonne retirée déclarée **après** la clé, tout est juste.

## Où (par lecture, non exhaustif)

- `src/processor/operator/persistent/insert_executor.cpp:43` : `pkVector = columnDataVectors[table->getPKColumnID()]` — les vecteurs sont rangés par propriété, l'indice est un numéro de colonne.
- `NodeTable::commit` (`src/storage/table/node_table.cpp`) : l'inscription des clés à l'index balaie les groupes **locaux**, rangés par propriété, avec les numéros de colonnes de l'index.
- `WALReplayer::replayNodeTableInsertRecord` (`src/storage/wal/wal_replayer.cpp`) : `ownedVectors[table.getPKColumnID()]`, même confusion au rejeu.

Le correctif `308ebd17e` (le point de reprise après `DROP` indexe ses colonnes par leur position) est de la même famille et ne couvre pas ces endroits.

## Témoin

Aucun au dépôt. La recette ci-dessus ; puis la même suivie d'une mort base ouverte et d'un rejeu ; puis un `COPY` dans la même table.

## Le correctif de l'amont

Non regardé.

## Étendue (banc, 5 octobre 2026)

Une passe de lecture systématique de `src/` et des extensions a trouvé la même confusion à
une dizaine d'endroits, dans **deux fenêtres** :

- **A, entre le DROP et le point de reprise** : les numéros de colonne du catalogue ne
  valent plus les positions des propriétés. Or les lignes d'une insertion
  (`columnDataVectors`, `propertyVectors`), les groupes locaux (`LocalNodeTable`) et les
  enregistrements d'insertion du journal sont rangés par position ;
- **B, après le point de reprise** : il renumérote le catalogue (`vacuumColumnIDs`) et
  compacte les colonnes, mais ne touche ni `NodeTable::pkColumnID`, calculé une fois à la
  construction, ni les numéros de colonnes des index (`IndexInfo::columnIDs` : clé
  primaire, hnsw, fts). L'index les garde jusque sur disque.

Témoins, tous rouges et stables, dans `known_red.txt` (`upstream_fixes_test.cpp`,
`Cases/ColumnIdTakenForAPosition.*` et `Copy/DropBeforeThePrimaryKey.*`) :

| Cas | Fenêtre | Ce qu'on voit |
|---|---|---|
| InsertAfterDrop | A | l'insertion passe ; la ligne est introuvable par sa clé |
| InsertWhenTheKeyIsLastAfterDrop | A | `vector::_M_range_check` |
| ReadOfARowOfTheTransactionAfterDrop | A | ligne de la transaction introuvable |
| UpdateOfARowOfTheTransactionAfterDrop | A | idem |
| CopyAfterDrop | A | le COPY échoue |
| ReplayOfAnInsertAfterDrop | A | après le rejeu, ligne introuvable par sa clé |
| ReadOfARelationOfTheTransactionAfterDrop | A | SIGSEGV |
| UpdateOfARelationOfTheTransactionAfterDrop | A | la mauvaise propriété est écrite (« v/z » au lieu de « z/w ») |
| UpdateAfterDropAndCheckpoint | B | « Cannot update pk » sur une autre propriété |
| InsertAfterDropAndCheckpoint | B | ligne introuvable par sa clé |
| InsertAfterDropCheckpointAndReopen | B | idem, **après réouverture** (les numéros de l'index sont persistés) |
| VectorIndexAfterDropCheckpointAndReopen | B | SIGSEGV |
| DropBeforeThePrimaryKey (COPY forcé, COPY journalisé) | A | « duplicated primary key » à l'insertion |

Les endroits, par lecture : `insert_executor.cpp:43` ;
`NodeTable::validateUniquenessConstraint` (`node_table.cpp:444`, 448) ;
`NodeTable::insert` (`node_table.cpp:524`, les vecteurs passés aux index, donc fts mal
indexé) ; `NodeTable::commit` (le balayage des colonnes d'index sur les groupes locaux, et
`node_table.cpp:761`, 764) ; le balayage et la lecture d'une ligne locale
(`ChunkedNodeGroup::scan` et `lookup` sur `chunks[columnID]`) ; `LocalNodeTable::update` ;
`LocalNodeTable`, le type de l'index de hachage local en fenêtre B ;
`node_batch_insert.cpp:235` ; la fusion des statistiques locales (`node_table.cpp:886`) ;
`LocalRelTable::rewriteLocalColumnID` (`columnID + 1`) ; le rejeu
(`wal_replayer.cpp:623`) ; en fenêtre B, `pkColumnID` et `IndexInfo::columnIDs` (hnsw
`hnsw_index.cpp:483` et suivants, fts `fts_index.cpp`, `fts_update_state.cpp`).

Deux formes de correctif, à choisir :

- **(i)** garder le local, les vecteurs d'insertion et le journal rangés par position, avec
  une seule conversion « numéro de colonne → position » tirée du catalogue, utilisée à chaque
  accès au local et aux `propertyVectors` ;
- **(ii)** ranger le local par numéro de colonne, comme les groupes validés.

Dans les deux cas, la fenêtre B demande que le point de reprise réécrive `pkColumnID` et les
`columnIDs` de chaque index au moment de la renumérotation, ou qu'on ne les garde plus (les
recalculer depuis le catalogue). Les bases déjà écrites après un tel DROP portent des numéros
d'index périmés sur disque : une réécriture à l'ouverture, ou un rebâti.

## Pour le fermer

Une seule fonction qui donne la position de la clé parmi les propriétés d'une table, utilisée par l'exécuteur d'insertion, le commit des lignes locales et le rejeu ; les trois témoins ci-dessus.
