# Migrations concurrentes : la part moteur **[lu, 13 octobre 2026, 0 h 30 ; rien exécuté, pause de Lucie]**

Le cas d'usage de Lucie : plusieurs développeurs font chacun leurs migrations sur la même base,
aucun bloqué par l'autre. Trois questions au code (master 65cb9a1f3), pour la page de l'arbre
principal et le scénario du banc (`~/.cache/rag3db-banc-notes/scenario-vingt-clients.md`).
Référence : PostgreSQL (DDL transactionnel, ADD COLUMN sans réécriture, verrous par table).

## 1. Le DDL transactionnel : ce qui l'est, ce qui ne l'est qu'à moitié

**Aucun refus générique du DDL en transaction manuelle.** `Statement::requireTransaction()`
(parser/statement.h:22-29) vaut vrai pour tout sauf BEGIN/COMMIT, et `validateTransaction`
(client_context.cpp:438-446) ne refuse que l'écriture dans un `BEGIN TRANSACTION READ ONLY`.
`BEGIN; ALTER TABLE person ADD random INT64; ROLLBACK;` passe (ddl_tinysnb.test:472-482).
La seule exception : **CREATE_VECTOR_INDEX exige la transaction automatique**
(hnsw_index_utils.cpp:150-156, appelé de create_hnsw_index.cpp:331 seulement ; les index spatiaux de
geo forcent aussi un point de reprise, create_spatial_index.cpp:202) — le CREATE se réécrit en `BEGIN … COMMIT` et force
un point de reprise (create_hnsw_index.cpp:299, 342, 368) ; CHECKPOINT est refusé si une
transaction est ouverte (operator/transaction.cpp:87-91).

| énoncé | dans BEGIN…COMMIT | point de reprise forcé | effet stockage immédiat | au ROLLBACK |
|---|---|---|---|---|
| CREATE NODE TABLE / CREATE REL TABLE | permis | non | `StorageManager::createTable` : l'objet table existe (storage_manager.cpp:138-151) | catalogue rendu ; l'objet reste jusqu'au point de reprise (`reclaimDroppedTables`, l.163-197) |
| ALTER … ADD FROM/TO (une relation de plus dans un groupe) | permis | non | `addRelTable` (alter.cpp:342-348), **sans `StorageManager::mtx`** | idem |
| ALTER ADD COLUMN | permis | non | **la colonne est écrite tout de suite, ligne par ligne** (§2) | catalogue rendu ; colonne et pages restent jusqu'au point de reprise, qui ne garde que les colonnes de l'entrée validée (node_table.cpp:993-1024, node_group.cpp:549-553) |
| DROP COLUMN | permis | non | `setHasChanges()` seulement (table.h:181) | catalogue rendu ; rien à défaire |
| RENAME, DROP TABLE | permis | non | catalogue seul | catalogue rendu |
| CREATE VECTOR INDEX | **refusé** | **oui** | `addIndex` | — |
| DROP VECTOR INDEX | permis (seul le CREATE appelle `validateAutoTransaction`) | non | `dropIndex` efface l'index tout de suite (node_table.cpp:1341) | rien ne le remet (la rétention de hotfixs, en cours, versionne le holder) |
| COPY FROM | permis | selon le cas (erreurs ignorées, journal au-delà du seuil) | | |

**Le catalogue est versionné comme une ligne** : chaîne `prev` par nom, version non validée
portée par l'identifiant de sa transaction, visibilité « créée par moi ou validée ≤ mon
instantané » (catalog_set.cpp:133-147), enregistrement d'annulation CATALOG_ENTRY (undo_buffer.cpp
:96-103, commit = horodatage l.219-224, rollback = la version retirée de la chaîne l.273-295).

**Le stockage, lui, n'a aucun enregistrement d'annulation** (undo_buffer.h:73-79 : CATALOG_ENTRY,
SEQUENCE_ENTRY, UPDATE_INFO, INSERT_INFO, DELETE_INFO — rien pour une colonne ni une table).
Le ROLLBACK d'un DDL est donc « vrai » pour qui lit le catalogue, et « différé au point de
reprise » pour ce que le stockage a déjà écrit. Trois trous lus dans le code, à prouver par un
témoin à la reprise :

1. **ADD COLUMN, ROLLBACK, puis ADD COLUMN avant tout point de reprise** : le catalogue rend
   `nextColumnID` à N (property_definition_collection.cpp:56), le stockage garde N+1 colonnes
   (`columns.push_back`, node_table.cpp:845) ; la colonne suivante reçoit l'ID N au catalogue et
   l'indice N+1 au stockage — lire `y` par N lirait la colonne annulée. Aucun test ne joue
   « ADD, ROLLBACK, ADD sans RELOADDB ».
2. **Deux ALTER concurrents sur la même table** : `alterTableEntry` et `dropEntryNoLock` ne
   vérifient pas le conflit écriture-écriture (`checkWWConflict` n'est appelé que par
   `createEntryNoLock`, catalog_set.cpp:26-31, 101-104, 171-220) ; la seconde version se pose
   par-dessus une version non validée de l'autre.
3. **ADD COLUMN sous des lecteurs** : il ne prend ni `inMemAppendMtx` ni de verrou sur
   `NodeTable::columns` ; `columns.push_back` et `chunks.push_back` réallouent sous un lecteur qui
   ne tient que `inMemAppendMtx` (node_group.h:280-285).

**Ce qu'il faudrait pour qu'ADD COLUMN, DROP COLUMN et ADD RELATION soient transactionnels au
sens de PostgreSQL** (une migration = une transaction, deux migrations = deux transactions qui
se voient ou se refusent, jamais qui se mélangent) :

- (a) le conflit écriture-écriture du catalogue sur ALTER et DROP, avec l'erreur de sérialisation
  nommée (comme deux UPDATE de la même ligne) : deux développeurs qui altèrent la même table, le
  second attend ou est refusé, jamais une chaîne mêlée ;
- (b) des enregistrements d'annulation du stockage pour ADD COLUMN (rendre la colonne et ses pages
  au ROLLBACK, et ne pas rendre `nextColumnID` tant que le stockage tient l'indice — ou adresser
  les colonnes du stockage par columnID et non par position) et pour CREATE REL TABLE / ADD
  RELATION (retirer la table au ROLLBACK, sans attendre le point de reprise) ;
- (c) un verrou nommé de table pour le DDL : `LockResource::table` EXCLUSIF au DDL, PARTAGÉ aux
  écritures, la forme déjà prise pour INDEX (il n'existe aujourd'hui que ROW et INDEX,
  lock_manager.h:40-48) — avec lui, un ADD COLUMN ne croise plus un lecteur ni un écrivain de la
  table, et c'est ce qui donne « aucun bloqué par l'autre » : bloqué le temps du DDL de la table
  qu'il touche, jamais par le DDL d'une autre table ;
- (d) ADD COLUMN paresseux (§2) pour que ce verrou soit tenu des millisecondes et non des secondes.

## 2. Le coût d'un ADD COLUMN sur des millions de lignes

**Il écrit chaque ligne, tout de suite, sous le verrou de la liste des groupes de la table.**
`NodeGroupCollection::addColumn` (node_group_collection.cpp:181-190) tient `nodeGroups.lock()`
pendant toute la boucle ; `NodeGroup::addColumn` (node_group.cpp:416-424) tient `chunkedGroups` ;
`ChunkedNodeGroup::addColumn` (chunked_node_group.cpp:474-485) alloue un `ColumnChunk` de pleine
capacité, le remplit par vecteurs de 2 048 (`populateWithDefaultVal`, column_chunk_data.cpp:524-
539 — l'expression par défaut évaluée même quand il n'y en a pas : des NULL), et **pour un bloc
sur disque l'écrit immédiatement** (`flush` → `allocatePageRange` + `flushBuffer`, l.314-322).
Pour une table de relations, deux fois (FWD et BWD). Coût : O(lignes) en temps, un chunk par
bloc en mémoire, des pages neuves dans le fichier de données avant même la validation.

Le verrou du catalogue, lui, n'y est pour rien : `CatalogSet::mtx` exclusif n'est tenu que
pendant `alterTableEntry` (catalog_set.cpp:185), relâché avant le stockage ; aucun mutex au niveau
de `Catalog`. Ce qui bloque, c'est `nodeGroups.lock()` : pendant l'ADD COLUMN, tout `append`,
`getOrCreateNodeGroup`, `getNumTotalRows` de **cette table** attend (node_group_collection.cpp:
21, 34, 72, 114, 159, 165) — et comme les appends ont lieu dans `localStorage->commit()`, sous le
mutex public du gestionnaire, **toutes les validations du processus attendent derrière lui**, y
compris celles des autres tables.

**PostgreSQL** : ADD COLUMN sans défaut, ou avec un défaut constant, n'écrit que le catalogue
(`pg_attribute.attmissingval`, PostgreSQL 11) ; la valeur est rendue à la lecture des lignes qui
ne l'ont pas, et matérialisée seulement à la réécriture de la table. Avec un défaut volatile
(`now()`, une séquence), la table est réécrite.

**La forme ici** : une colonne « déclarée, non matérialisée » — `Column` avec sa valeur par défaut
constante et aucun chunk dans les blocs existants ; `ChunkedNodeGroup::scan/lookup` rend la valeur
par défaut quand le bloc n'a pas de chunk pour cette colonne (le chunk est créé au premier SET du
bloc, ou au point de reprise qui l'écrit à plat) ; un défaut non constant garde la réécriture
immédiate, comme PostgreSQL. ADD COLUMN redevient une écriture de catalogue : des millisecondes
sous le verrou de table de §1-(c), et l'annulation (§1-(b)) n'a presque rien à défaire.

## 3. Vingt écrivains dans un processus : où ça se sérialise

L'écriture multiple est le défaut (`enableMultiWrites{true}`, db_config.cpp:33). Dans l'ordre
du poids, lu dans le code :

1. **La validation est entièrement sérialisée, fsync compris.** `TransactionManager::commit`
   tient `mtxForSerializingPublicFunctionCalls` de bout en bout (transaction_manager.cpp:107-195),
   et sous lui `Transaction::commit` (transaction.cpp:175-212) fusionne le stockage local dans
   les tables (`localStorage->commit()`, O(lignes écrites)), date le tampon d'annulation, compacte
   les listes CSR, puis `WAL::logCommittedWAL` copie le journal local et **`flushAndSyncNoLock`
   fait un fsync** (wal.cpp:28-37, 61-64). Vingt écrivains = vingt validations l'une après
   l'autre, un fsync chacune, sans regroupement. C'est la borne du débit : à ~1 ms de fsync sur
   SSD, 20 écrivains qui valident sans cesse font la queue, et un p99 à 20 ms par instruction est
   atteignable seulement si les validations sont courtes (peu de lignes locales). PostgreSQL :
   group commit (`commit_delay`), WAL écrit par un seul walwriter, fsync regroupé.
2. **Le point de reprise bloque les BEGIN.** `checkpointPending` fait attendre tout
   `beginTransaction` (l.57, 383-402) : 250 ms au plus pour un point de reprise automatique
   (`AUTO_CHECKPOINT_WAIT`), sans délai pour un forcé ou un CHECKPOINT explicite. Un point de reprise
   forcé (CREATE_VECTOR_INDEX, COPY avec erreurs ou journal au-delà du seuil) attend d'abord le
   départ de **toutes** les autres transactions, lecteurs compris, jusqu'à 5 s
   (`stopNewTransactionsAndWaitForOthersNoLock`, l.263-322 ; `DEFAULT_CHECKPOINT_WAIT_TIMEOUT` 5 s,
   constants.h:21) ; au-delà, « Timeout waiting for active transactions to leave », rien n'est
   validé et le CREATE échoue. Les autres validations forcées font la queue derrière (A7,
   l.324-381). Un écrivain à deux instructions (BEGIN ; … ; COMMIT) qui dure, ou un lecteur qui
   traîne, fait donc échouer un CREATE INDEX concurrent — ou le retarde de 5 s.
3. **Le LockManager** (lock_manager.cpp) : ROW (table + clé) et INDEX (table), SHARED/EXCLUSIVE
   gardés jusqu'à la fin de la transaction ; **un seul mutex et une seule condition pour toute la
   table de verrous**, réveil de tous à chaque libération (`notify_all`, l.251) par tranches de
   10 ms ; file FIFO par ressource ; interblocage détecté à la demande par le graphe d'attente
   (l.100-109, 189-221), délai 30 s. Vingt écrivains qui attendent des clés différentes se
   réveillent tous à chaque libération : un coût de réveil, pas un blocage. Une insertion prend
   INDEX (exclusif si la table a un index secondaire, partagé sinon) et ROW(clé) exclusif, puis
   reprend son instantané (node_table.cpp:624-637) : **sur une table indexée HNSW, les écrivains
   sont sérialisés par INDEX exclusif pour toute leur transaction** — c'est la contention la plus
   forte du scénario dès que CREATE_VECTOR_INDEX a été joué (minute 4).
4. **Le stockage** : `inMemAppendMtx` exclusif à l'ajout d'un groupe (sous le mutex public de
   toute façon) contre les lectures sans verrou de ce groupe ; `csrIndexMtx` exclusif aux ajouts et
   compactions de relations contre les balayages des listes CSR du groupe (csr_node_group.cpp:443,
   459, 575) ; `chunkedGroups` (mutex simple par groupe) ; `versionInfoMtx` ; `StorageManager::mtx`
   sur `getTable`. Le stockage local est propre à chaque transaction : aucune contention entre
   écrivains pendant l'exécution, tout se joue à la validation.
5. **Le journal** : un `LocalWAL` par transaction (sans contention), `WAL::mtx` à la validation,
   déjà couvert par le mutex public.

**En une phrase pour la page** : aujourd'hui, vingt développeurs ne se bloquent pas pendant
qu'ils écrivent, ils font la queue pour valider (une validation à la fois, avec son fsync), ils
attendent à BEGIN pendant un point de reprise, et un ADD COLUMN sur une grosse table arrête les
validations de tout le processus le temps d'écrire la colonne ; un CREATE d'index vectoriel exige
que tous partent dans les 5 s, sinon il échoue.

## 4. Pour le scénario du banc (vingt clients, un DDL par minute)

- **Un ALTER en transaction explicite est permis** : la migration 6 se joue bien en
  `BEGIN ; ADD a ; ADD a ; COMMIT`, le second ADD est refusé par le binder (propriété existante)
  et la transaction s'annule ; « rien ne reste » se juge sur le catalogue tout de suite, et sur
  le stockage après un CHECKPOINT. À ajouter au scénario, c'est le trou 1 de §1 : **après la
  migration échouée, un `ADD COLUMN b` puis une lecture de `b`, avant tout point de reprise**.
- **Seul CREATE_VECTOR_INDEX force un point de reprise** (et les index spatiaux) ; ADD COLUMN,
  CREATE REL TABLE, DROP COLUMN, non.
- **L'attente d'un verrou n'est pas lisible du dehors** : aucune trace ni compteur dans le
  LockManager ; la latence de bout en bout est la mesure (une trace `RAG3DB_TRACE_LOCKS` se
  code en une heure, après la pause).
- **Les bornes** : (1) ADD COLUMN sans défaut ≤ 100 ms ne tiendra pas sur une grosse table et
  tient peut-être sur 50 000 Doc (chunks alloués, NULL évalués, pages écrites) — borne à fixer
  après une première mesure, l'écart à PostgreSQL (catalogue seul) est le constat, pas une couleur ;
  et « personne n'attend plus que sa durée + 50 ms » vaut pour les écrivains de cette table **et
  pour toutes les validations du processus** (le mutex public) — mesure-le sur un écrivain d'une
  autre table aussi. (2) avec défaut : même forme. (3) CREATE REL TABLE : catalogue et un objet,
  ≤ 100 ms tient. (4) CREATE_VECTOR_INDEX : les écrivains n'attendent pas pendant le bâti (pas
  de verrou INDEX pris par le CREATE, forme (α) de hotfixs) mais **leurs BEGIN attendent pendant
  le point de reprise forcé**, jusqu'à 5 s, et le CREATE échoue si une transaction explicite ou
  un lecteur dure plus de 5 s : la borne est « les écrivains hors transaction explicite attendent
  au plus la durée du point de reprise ; un CREATE refusé après 5 s est un rouge nommé, pas une
  perte » ; les lecteurs ne sont jamais refusés, leurs BEGIN attendent aussi. (5) CHECKPOINT
  explicite : refusé si la connexion a une transaction ouverte ; attend les autres ≤ 5 s ; les
  écrivains attendent à BEGIN pendant sa durée — borne « durée + 50 ms » tient. (6) comme dit
  ci-dessus, plus l'ADD b. (7) p99 < 20 ms hors DDL : à mesurer ; la queue des validations avec
  fsync est le suspect si ça dépasse, et le INDEX exclusif après la minute 4.
