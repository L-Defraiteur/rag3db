# Le CREATE_VECTOR_INDEX concurrent lit une ligne au-delà de la colonne mise en cache

- **État** : fermé (19 novembre 2026, branche `create-index-reads-its-cached-column-3`) ; ouvert le 8 novembre
  2026, trouvé par l'ASAN du cœur sur la lib dadbf0041
- **Famille** : moteur
- **Gravité** : plantage possible (lecture hors d'une allocation ; en Release, une valeur quelconque
  lue comme nulle ou non nulle, ou une page voisine)
- **Atteignable en service** : oui, dès qu'un CREATE_VECTOR_INDEX tourne sous des écritures dans
  sa table (le CREATE concurrent, 97daa41ed, 26 octobre)
- **Touche rag3weaver** : par le moteur (les reconstructions de l'index vectoriel sous la montre)

## Ce que c'est

Sous ASAN (luciepc, worktree rag3db-bissect, `build/asan`), `TwentyWriters.DdlEveryFifteenSeconds`
du banc rend un heap-buffer-overflow :

- **la lecture** : READ of size 8, 0 octet après une région de 6 776 octets, par le fil T112, dans
  `NullMask::isNull`, appelé par `ColumnChunkData::isNull`, puis `isNull`
  (`extension/vector/src/index/hnsw_graph.cpp:92`), `InMemEmbeddings::getEmbedding`,
  `InMemHNSWIndex::insert` et `createInMemHNSWTableFunc`
  (`extension/vector/src/function/create_hnsw_index.cpp:98`) ;
- **l'allocation** : le masque de nuls d'un `ListChunkData` alloué par le fil T8 dans
  `_CACHE_COLUMN` (`tableFunc`, `src/function/table/cache_column.cpp`).

6 776 octets de masque font 54 208 lignes : l'insertion HNSW demande une ligne au-delà de la colonne
mise en cache. Le journal ASAN complet est dans
`~/.cache/rag3db-moteur-notes-local/bissect/asan-banc.log` à luciepc.

## La lecture du cœur

Les morceaux de `createInMemHNSWTableFunc` (`morsel.startOffset` à `morsel.endOffset`) sont bornés
par un nombre de lignes lu **après** la mise en cache de la colonne, qui compte les lignes ajoutées
entre-temps par les écrivains concurrents. Ou bien la colonne est mise en cache sur un instantané plus
court que le compte qui borne l'insertion. Les deux bouts doivent venir du même instantané : le
CREATE en deux transactions de (A) bâtit sous T_B ce que T_A a vu.

## Le témoin

À écrire : un CREATE_VECTOR_INDEX sous un écrivain qui insère dans la table pendant la mise en cache,
réglé pour que l'insertion lise au-delà ; sous ASAN, le rapport ; en Release, la borne vérifiée par
un compte (les nœuds insérés dans le graphe égalent les lignes de la colonne en cache).

## Ce qui n'est pas lié

Le SIGSEGV de l'hôte du portable (ticket
[un SIGSEGV dans optimisticRead…](../../coeur-cpp-hotfixs-tickets/tickets/2026-11-08-un-sigsegv-dans-optimisticread-sous-lecture-concurrente-pendant-une-indexation.md))
lit une page sur disque dans `optimisticRead`. Ce défaut-ci lit hors d'une allocation en mémoire.
Probablement distinct ; le défaut est sans doute antérieur à 17a094d12.

## La fermeture

`HNSWGraph` (`extension/vector/src/index/hnsw_graph.cpp`, `isNull`) tient pour nulle une ligne hors des morceaux de la
colonne en cache, comme le cache le fait d'une ligne qu'il ne voit pas : le bâti ne lit plus au-delà.

Le témoin `VectorIndexCachedColumn.TheBuildNeverReadsPastItsCachedColumn` (`test/transaction/vector_index_cached_column_test.cpp`)
rejoue la réécriture du CREATE en requêtes séparées, une autre connexion insérant 6 000 lignes entre la mise en cache
et le bâti. À luciepc, sous ASan : rouge sans le correctif (heap-buffer-overflow dans `ColumnChunkData::isNull`, appelé
par `InMemEmbeddings::getEmbedding` et le constructeur de `InMemHNSWIndex`), vert avec.

La passe de la tête de la pile C++ du 18 au 19 novembre (22f813f91a, `journaux-hotfixs-pile-matin`) : les témoins des
quatre lots, transaction_test 399, `concurrence_test.known_red`, les 74 cas des .test de vector, e2e_test 1 867
(DropNodeTableReclaim, rouge de master admis).
