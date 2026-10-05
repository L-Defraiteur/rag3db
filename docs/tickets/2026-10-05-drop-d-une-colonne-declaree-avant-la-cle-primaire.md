# Retirer une colonne déclarée avant la clé primaire casse l'insertion

- **État** : ouvert
- **Gravité** : blocage (toute insertion ordinaire dans la table est refusée ou lit la mauvaise colonne comme clé)
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

## Pour le fermer

Une seule fonction qui donne la position de la clé parmi les propriétés d'une table, utilisée par l'exécuteur d'insertion, le commit des lignes locales et le rejeu ; les trois témoins ci-dessus.
