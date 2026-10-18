# La rétention des index ne couvre que l'index vectoriel : une recherche spatiale perd son index sous un DROP, et rend zéro ligne sans le dire

- **État** : ouvert (18 octobre 2026, seconde session cœur C++, à la demande de l'orchestration).
  La forme est arrêtée par l'orchestration : la rétention vit au niveau du holder d'index, pour
  tous les genres d'index (HNSW, R-tree, et ceux qui viendront), et non dans chaque extension. À
  faire après le ROLLBACK sous un tampon plein, avant (A), le CREATE concurrent, qui en dépend.
- **Gravité** : réponse fausse (zéro ligne, sans erreur). Peut-être aussi un accès à un index
  détruit : à vérifier.
- **Atteignable en service** : oui, dès qu'une recherche spatiale croise un DROP ou un re-CREATE
  de son index.
- **Touche rag3weaver** : oui, s'il emploie l'index spatial. Le constructeur de decks, peut-être.

## Ce que c'est

La rétention (`9b861579a`) versionne les holders d'index comme des lignes : un index retiré ou
créé ne change, pour une transaction, qu'à la validation qui la précède. Mais seuls les chemins
de l'index vectoriel lisent cette visibilité :
- `getVisibleSharedIndexes` ;
- `OnDiskHNSWIndex::captureForQuery`, qui capture l'index en propriété partagée ;
- `hasIndexHolder(name, transaction)`.

Les accès par nom, `getIndex`, `getIndexHolder`, `getSharedIndex` et `loadIndex`
(`node_table.cpp`), filtrent `!isDropped()`, pas la visibilité pour la transaction. L'extension
geo passe par eux :
- **`query_spatial_index.cpp:152`** : `nodeTable->getIndex(bd.indexName)`. Un DROP validé après le
  début du lecteur, ou encore non validé, est déjà marqué retiré : l'index n'est pas trouvé, et la
  fonction rend **0** ligne (`return 0`) au lieu de lire l'index que son instantané voit, ou de le
  refuser par un nom. L'index est rendu par un pointeur nu (`Index*`), pas en propriété partagée :
  si le holder est effacé pendant la recherche, le pointeur ne le tient pas. C'est à confirmer
  par un témoin.
- **`drop_spatial_index.cpp:66`** : le DROP lit le chemin du fichier de l'index par `getIndex`
  avant de le retirer du catalogue.
- Un nom recréé par une transaction non validée peut aussi être pris par une autre (le holder
  neuf n'est pas retiré).

**Le défaut voisin de la finalisation d'un COPY venait de la rétention elle-même** (`9b861579a`,
seconde session cœur C++). La rétention garde, pour les lecteurs d'avant, le holder d'un index
retiré. `NodeBatchInsert::finalize` appelait finalize sur tous les holders de la table, sans
filtre : un CREATE, un DROP puis un COPY finalisaient dans l'index retiré, dont les tables de
relations n'existent plus (« _0_vec_idx_UPPER does not exist in catalog », puis « must reopen »).

Ce chemin n'a pas été relu quand la rétention a été écrite : seuls l'ont été les chemins de
l'écriture ordinaire (`isWritableIndex`, `initUpdateState`, l'étape 3 du commit). La régression
a d'abord été attribuée à `5e3df4563` (le rollback sous lecteurs du cœur), puis reproduite par le
cœur sur `8f4779bdc` rebâti, qui contient déjà la rétention.

Le cœur la corrige avec le filtre de visibilité, à la transaction du COPY, dans
`NodeBatchInsert::finalize`, et du même coup dans `NodeTable::rollbackIndexInsert`. Les magasins
de libs sont revenus à `87a362e30`, la dernière lib saine d'avant la rétention.

## La forme

- **Les accès par nom filtrent la visibilité.** `getIndex(name, transaction)`,
  `getIndexHolder(name, transaction)` et `getSharedIndex(name, transaction)` prennent la
  transaction et ne rendent que l'exemplaire qu'elle voit (`isIndexVisible`). Le cas d'un nom
  retiré puis recréé est tranché par la visibilité, comme pour HNSW. Les appels sans transaction
  (le point de reprise, le rejeu) passent `DUMMY_CHECKPOINT_TRANSACTION` ou ont leur propre
  fonction, dite comme telle.
- **Une recherche capture son index en propriété partagée**, comme `captureForQuery` le fait pour
  HNSW. C'est une fonction du holder, commune à tous les genres d'index, pas une par extension.
- **Un index absent se refuse par un nom** (« does not exist »), jamais par zéro ligne.
- **Le genre d'index n'a rien à faire.** La visibilité, la capture et le refus nommé sont dans
  `NodeTable` et `IndexHolder`. Une extension n'appelle que l'accès par nom avec sa transaction.

## Témoins, à écrire

1. Une recherche spatiale ouverte avant un DROP validé lit encore l'index (ou le refuse par un
   nom, si son genre ne le permet pas), jamais zéro ligne.
2. Une recherche spatiale pendant un DROP non validé lit l'index.
3. Le même pour un DROP puis un re-CREATE du même nom.
4. Les témoins de la rétention HNSW (`VectorIndexRetention.*`) restent verts.

## Pour le fermer

Les témoins verts ; les accès par nom de `NodeTable` sans transaction retirés, ou réservés au
point de reprise et au rejeu, et dits comme tels.
