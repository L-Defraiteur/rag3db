# Le recalcul des statistiques d'une table — une page avant le code

10 octobre 2026, seconde session cœur C++, lot (c). **Classement : confort.** Seuls le
planificateur (`cardinality_estimator.cpp:167`, `:180-183`, `:199`) et `STATS_INFO`
(`stats_info.cpp:41-43`) lisent ces comptes : au pire un plan moins bon, jamais une réponse
fausse. Hors des conditions de la stèle.

Lignes à `ba11e003b`. **[lu]** vu dans le code (repérage par agents, lignes clés revérifiées) ;
**[déduit]** ; rien n'est encore exécuté.

## 1. Ce qui ne va pas

- **Un compte de distincts ne recule jamais** [lu] : `HyperLogLog::update` et `merge` gardent
  un maximum (`hyperloglog.h:33`, `hyperloglog.cpp:17-21`).
- **`IGNORE_ERRORS`** [lu] : les lignes entrent aux statistiques à l'ajout
  (`node_batch_insert.cpp:180`), avant le contrôle de clé, qui est asynchrone (l'`IndexBuilder`,
  `:134`, `:288`) ; la ligne refusée est retirée par `delete_`, qui ne touche pas aux statistiques
  (`node_batch_insert_error_handler.cpp:34`).
- **`DELETE` et `SET`** ne touchent jamais `TableStats` [lu] (`NodeTable::delete_`, `update`) :
  les distincts ne reculent pas, et les valeurs posées par un `SET` ne sont pas comptées.
- **La cardinalité** est recalée sur `numTotalRows` au point de reprise et à l'ouverture
  (`node_group_collection.cpp:217`, `:290`), qui compte les lignes supprimées [lu] ; et le
  recalage du point de reprise ne joue que pour une table modifiée (`NodeTable::checkpoint`
  ne fait rien sans `hasChanges`, `node_table.cpp:855-886`).
- **Un défaut de plus, trouvé par le repérage** [lu, à rougir] : `ColumnStats::update`
  (`column_stats.cpp:14-28`) range les hachages à `sel[i]` (`computeHash`,
  `vector_hash_functions.cpp:34-36`, `:52-57`) mais les relit aux cases `0 … n-1`. Avec une
  sélection filtrée, l'HyperLogLog reçoit des cases fausses. Qui passe une sélection filtrée
  aujourd'hui n'est pas établi (un `COPY` sous `IGNORE_ERRORS` qui écarte des lignes mal formées
  est le premier suspect) ; le recalcul en passera une (les lignes supprimées en sont retirées).
  Les valeurs nulles sont aussi comptées comme une valeur (`NULL_HASH`).

## 2. Ce que font les moteurs établis (vérifié dans leur documentation le 10 octobre)

- **PostgreSQL** ne tient pas ses statistiques au fil des écritures. `ANALYZE` les estime « for
  large tables, … a random sample of the table contents, rather than examining every row » ; « the
  statistics are only approximate » ; il ne prend qu'« a read lock on the target table ». L'autovacuum
  le relance quand les lignes insérées, modifiées ou supprimées depuis le dernier passent « analyze
  base threshold + analyze scale factor * number of tuples ».
  ([ANALYZE](https://www.postgresql.org/docs/current/sql-analyze.html),
  [§24.1.3](https://www.postgresql.org/docs/current/routine-vacuuming.html))
- **Neo4j** tient de façon transactionnelle les comptes de nœuds par étiquette et de relations par
  type ; seule la sélectivité des index est échantillonnée, en fond, quand les changements
  atteignent `db.index_sampling.update_percentage` (5 % par défaut), ou à la main par
  `db.resampleIndex("…")` et `db.resampleOutdatedIndexes()`.
  ([Statistics and execution plans](https://neo4j.com/docs/operations-manual/current/performance/statistics-execution-plans/))

**Ce que nous prenons** : de PostgreSQL, le recalcul explicite (`CALL analyze`), et que les
distincts sont une estimation entre deux recalculs ; de Neo4j, l'idée que le compte de lignes, lui,
peut être juste sans échantillon (nous le recalons sur les lignes vivantes). **Écart** : nous
balayons tout au lieu d'échantillonner — l'HyperLogLog est déjà une estimation, et un échantillon
fausserait le compte de lignes. À revoir sur la mesure (§5).

## 3. La forme

- **`CALL analyze('Table')`**, fonction autonome (`STANDALONE_TABLE_FUNCTION`, comme
  `clear_warnings`, `function_collection.cpp:237-241`), `isReadOnly = false` : une transaction
  d'écriture, comme tout `CALL` qui modifie. Tables de nœuds seulement (`STATS_INFO` refuse déjà
  les autres, `stats_info.cpp:62-64`).
- **Un balayage des lignes validées vivantes** de chaque colonne à statistiques, par le chemin de
  `cache_column.cpp:95-140` (`NodeTableScanState`, `TableScanSource::COMMITTED`, la sélection
  qui exclut les lignes supprimées). Il bâtit un `TableStats{types}` neuf (`table_stats.h:14`) :
  cardinalité = lignes vivantes, un HyperLogLog neuf par colonne.
- **Le remplacement** : à la validation de la transaction, le `TableStats` neuf remplace celui de
  la table (et non `merge`, qui ne ferait que monter). Ce qui a été validé par d'autres entre le
  balayage et la validation n'est pas compté : une estimation, comme ailleurs. La table est marquée
  `setHasChanges()` pour que le point de reprise suivant écrive les statistiques (sinon elles ne
  le sont pas : `node_table.cpp:855-886`, `checkpointer.cpp:137-145`).
- **`STATS_INFO`** dit, sur sa déclaration (`simple_table_function.h:113-115`), que ses comptes sont
  des estimations à jour du dernier `analyze` ou du dernier chargement.

## 4. Ce qui change en chemin (src/storage, accord de la session cœur C++ du 10 octobre)

1. **`ColumnStats::update`** relit les hachages à `sel[i]` : le défaut du §1, rougi d'abord.
2. **Le commentaire périmé** de `node_group_collection.cpp:214-216` (le `COPY` ne fusionne plus
   ses statistiques avant sa validation, depuis `1177f5794`).
3. **Le recalage de la cardinalité sur les lignes vivantes** au point de reprise, seulement si un
   compte de lignes supprimées par groupe s'y trouve sans rebalayer les colonnes
   (`VersionInfo::getNumDeletions`, déjà appelé par `NodeGroup::checkpoint`,
   `node_group.cpp:445`, est le candidat) ; sinon un ticket. **`numTotalRows` n'est jamais
   touché** : il est aussi l'allocateur des décalages.

## 5. Les témoins, rouges d'abord

`STATS_INFO` comparé au vrai compte (`count(*)`, `count(DISTINCT …)`) :
1. après un `COPY` sous `IGNORE_ERRORS` qui écarte des clés en double portant des valeurs
   propres ;
2. après un `DELETE` de la moitié des lignes ;
3. après un `SET` qui pose des valeurs neuves ;
4. le défaut de sélection de `ColumnStats::update`, s'il se fabrique par l'interface (sinon par
   un test de `ColumnStats` seul) ;
puis, après `CALL analyze`, les trois premiers justes (cardinalité exacte, distincts dans la marge
de l'HyperLogLog, à fixer d'après sa précision à 64 registres), et une réouverture qui garde les
statistiques recalculées.

**La mesure demandée** : le coût du balayage sur une table de 100 000 lignes (clé, chaîne,
`FLOAT[768]`), seul sur le poste ; le déclenchement automatique se décide après, sur ce chiffre.

## 5 bis. Ce qui est fait (10 octobre, soir)

- **Le défaut de sélection** : corrigé seul d'abord (`d10b92306`), rouge « estimation 1 pour
  1000 » puis vert.
- **`CALL analyze('Table')`** (`src/function/table/analyze.cpp`) : un balayage des colonnes
  validées, la sélection écarte les lignes supprimées, `TableStats::update` par colonne ; puis
  `NodeTable::replaceStats`, qui marque la table pour le point de reprise.
- **Le recalage** au point de reprise et à l'ouverture se fait sur les lignes vivantes
  (`NodeGroup::getNumLiveRows` : par bloc, lignes moins suppressions, d'après les informations de
  version persistées avec lui) ; `numTotalRows` intact. Témoin rouge sur l'ancien recalage (2 000
  au lieu de 1 000), vert après.
- **Les témoins** (`TableAnalyzeTest`) : rouges avant (cardinalité 2 000 pour 1 000 après
  `IGNORE_ERRORS` et `DELETE` ; noms distincts 2 051 et 1 931 pour 1 000, 10 pour 1 000 après
  `SET` ; encore après réouverture), justes après `analyze` et après la réouverture.
- **Un écart à la forme du §3** : le remplacement se fait pendant l'exécution du `CALL`, pas à la
  validation de sa transaction. Un `analyze` dans une transaction annulée laisse donc ses
  statistiques — des estimations, que rien d'autre ne lit qu'un planificateur. Le faire à la
  validation demanderait un « remplacement en attente » dans `LocalStorage`, à côté des
  statistiques en attente d'un `COPY`. **Accepté tel quel par l'orchestration** (Lucie peut
  renverser) : c'est ce que fait PostgreSQL pour `reltuples`/`relpages`, mis à jour sur place et
  gardés après un `ROLLBACK`, seul `pg_statistic` étant transactionnel (Tom Lane, pgsql-bugs,
  22 octobre 2014, [BUG #11638](https://www.postgresql.org/message-id/10043.1413988524%40sss.pgh.pa.us)).
  Témoin `AnAnalyzeInARolledBackTransactionLeavesAStaleEstimate` ; ticket
  `2026-10-10-analyze-non-transactionnel.md`.
- **Un effet de bord voulu** : un `COPY` forcé (ce qu'est encore un `COPY` sous
  `IGNORE_ERRORS`) fait son point de reprise, qui recale maintenant la cardinalité sur les lignes
  vivantes : après le `COPY` qui écarte des clés, elle est juste avant même `analyze`.

## 6. Ce qui reste dehors

- Le déclenchement automatique (après la mesure).
- Les tables de relations : aucune statistique tenue, l'estimation est `nextRelOffset`, qui est
  aussi l'allocateur d'identités (ticket `2026-10-05-copy-de-relations-annule-gonfle-l-estimation`).
