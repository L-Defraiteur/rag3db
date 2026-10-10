# Cœur C++ — ce que la session sait

Relevé de connaissances sur le moteur. Ce qui est affirmé porte un fichier, un commit ou
une mesure ; ce qui ne l'est pas est dit « non vérifié ». Les numéros de ligne datent du
3 octobre 2026 et bougent : chercher le nom de la fonction.
**Dernière mise à jour : 10 octobre 2026.**

Le relevé du 2 octobre (journal, point de reprise, lecteurs concurrents, mode
multi-écrivains) reste valable :
`../../2-octobre-2026-01h07/moteur-concurrence/02-knowledge-dump.md`.

## 1. Le journal et la reprise

- **Le rejeu a lieu dans le constructeur de `Database`** (`Database::initMembers` →
  `StorageManager::recover`, `src/main/database.cpp`), donc avant tout `LOAD EXTENSION` de
  l'appelant. À la fin du rejeu, la reprise écrit elle-même un point de reprise : après la
  première réouverture il n'y a plus de journal.
- **`LOAD EXTENSION` est journalisé et rejoué** (`ExtensionManager::loadExtension`,
  `src/extension/extension_manager.cpp` ; `WALReplayer::replayLoadExtensionRecord`). Un
  second `LOAD` d'une extension déjà chargée n'écrit rien. Un point de reprise vide le
  journal, cet enregistrement compris.
- **Qui écrit un point de reprise sans qu'on le demande** : `CREATE_VECTOR_INDEX`, `COPY`,
  le point de reprise automatique, la fermeture, la fin du rejeu.
- **La garde 1** (`fcd9a7882`) : `NodeTable::isWritableIndex`
  (`src/storage/table/node_table.cpp`). Au rejeu, un index non chargé est *détaché*
  (`IndexHolder::detach`, `src/include/storage/index/index.h`) : il n'est plus écrit par
  `NodeTable::serialize`, `getIndexHolder` et `getIndex` ne le voient plus, `addIndex`
  remplace un détaché du même nom. L'état « entrée au catalogue sans exemplaire dans la
  table » est l'état « à rebâtir », sans changement de format. Côté extension :
  `initHNSWEntries` (`extension/vector/src/main/vector_extension.cpp`) saute une telle
  entrée ; `throwIfBehindItsTable` (`extension/vector/src/index/hnsw_index_utils.cpp`) fait
  refuser la recherche et la création par `HNSWIndexUtils::INDEX_BEHIND_ITS_TABLE`
  (« is behind its table ») ; `DROP_VECTOR_INDEX` fonctionne.
- **Hors rejeu**, c'est le lieur qui refuse d'écrire dans une table dont un index n'est pas
  chargé (« … but its extension is not loaded », « … used in one or more indexes which is
  unloaded ») ; `NodeTable::INDEX_NOT_LOADED_FOR_WRITE` est un second rempart.
- **Le rejeu d'une mise à jour** appelle maintenant `initUpdateState` avant `update`
  (`WALReplayer::replayNodeUpdateRecord`) ; avant, aucun index n'en était informé.
- **Le rejeu d'un `DROP` d'index** retire l'entrée du catalogue *et* l'exemplaire de la
  table (`replayDropCatalogEntryRecord`, `f5acca417`). À l'exécution c'est la fonction de
  l'extension qui fait les deux (`drop_hnsw_index.cpp`).
- **Un doublon de clé primaire validé rend la base impossible à rouvrir** (« Found
  duplicated primary key value » au rejeu), index ou pas. Atteignable seulement sous le
  mode multi-écrivains. Réponse retenue par l'orchestration : A3′ l'empêche de naître ; la
  reprise devra mettre de côté la transaction fautive (fichier à côté, erreur nommée avec la
  table et la clé) au lieu de refuser d'ouvrir.
- **Mes tests de reprise ne tuent pas le processus** sauf
  `vector_index_crash_reopen_test.cpp` : les autres ferment la `Database` après
  `CALL force_checkpoint_on_close=false` (le journal reste et se rejoue), ou forcent un
  point de reprise en échec (`FlakyCheckpointer`). Le banc a désormais la famille « arrêt
  brutal, un seul écrivain » par vrai SIGKILL.
- **Non élucidé** : `e2e_idempotent_registration` rougit environ une fois sur dix,
  « Reading past the end of the file …wal with size 0 at offset 0 », avant comme après mes
  correctifs (2 sur 20 des deux côtés).

**La garde 2, repérage fait, rien de codé** (agent de repérage, non revérifié ligne à
ligne) :
- Un point de reprise supprime le fichier du journal (`WAL::reset`, appelé en fin de
  `Checkpointer::writeCheckpoint`) ; une fermeture propre ne laisse donc aucun journal.
- Le rejeu ne garde que les enregistrements suivis d'un COMMIT : un `LOAD EXTENSION`
  réécrit en tête du journal devrait être enveloppé (BEGIN, LOAD, COMMIT), et
  `loadExtension` exige une transaction active.
- **Forme A, la liste persistée avec la base** : l'en-tête porte une version de stockage
  (39, `storage_version_info.h`) contrôlée par égalité stricte ; la page n'est pas remise à
  zéro, donc un champ ajouté demande soit un marqueur, soit de monter la version (les
  anciens binaires n'ouvrent plus). Le catalogue est une suite fixe de neuf ensembles, sans
  queue optionnelle. À l'ouverture, en-tête et catalogue sont lus avant le rejeu : une
  liste y serait disponible à temps.
- **Forme B, l'enregistrement réécrit en tête du journal** : aucun format ne change, mais
  le journal n'est plus jamais vide — chaque ouverture, lecture seule comprise, fait un
  rejeu complet ; la revérification de l'ouverture en lecture seule compare l'en-tête et
  une somme des premiers octets du journal, qu'un journal recréé à l'identique pourrait
  tromper (inférence) ; une huitaine de tests supposent « pas de journal après un point de
  reprise ».
- **Dans les deux formes** : si le fichier de l'extension manque, l'ouverture doit continuer
  et laisser la garde 1 faire. Aujourd'hui `replayLoadExtensionRecord` relance l'erreur :
  **un journal qui porte un `LOAD EXTENSION` dont le fichier a disparu empêche d'ouvrir la
  base**.
- `ExtensionManager` garde par extension son nom, son chemin résolu et sa source ; les
  extensions liées statiquement sont chargées d'office avant le rejeu
  (`Checkpointer::readCheckpoint`).

- **Une colonne de chaînes se relit de deux façons** (`StringColumn::scanSegment` vers un
  bloc) : le segment entier — le dictionnaire est copié tel quel, les indices décalés — ou
  une partie — seules les chaînes des lignes relues entrent au dictionnaire du bloc et les
  lignes sont renumérotées. La seconde est celle du point de reprise des relations (une
  région à la fois, `CSRNodeGroup::checkpointColumnInRegion`) et des listes
  (`ListColumn::scanSegment` ne relit que la plage de ses listes, même quand le segment de
  la liste est entier). Le point de reprise d'une table de nœuds relit des segments entiers
  (`NodeGroup::checkpoint`) : une colonne `STRING` de nœuds ne passe pas par la seconde.
  Depuis `25b3b45dc` c'est le dictionnaire qui rend la correspondance des indices
  (`DictionaryColumn::scanToChunk`) ; avant, l'ordre d'apparition et l'ordre du disque
  étaient confondus dès que la colonne était très dupliquée (`duplicationFactor <= 0.5`).
- Le dictionnaire d'un bloc exige des décalages croissants, une chaîne par indice : la
  longueur d'une chaîne est l'écart au décalage suivant. On n'y range donc qu'en ajoutant
  à la fin, dans l'ordre où l'on écrit les données.
- Un COPY de relations se journalise au partitionneur, là où les identités sont réservées,
  sous un même verrou (`journalOrderMtx`) : le rejeu réattribue les identités dans l'ordre
  du journal.

- **Le stockage local d'une table de nœuds peut être versé en cours de transaction**
  (`LocalStorage::flushNodeTable`, depuis `f1d8c7190`, au début d'un `COPY`) : c'est
  `NodeTable::commit` appelé plus tôt. Après lui le stockage local est vide et ressert ; il
  repart de la fin de la table à sa prochaine insertion (`restartAt`, dans
  `getOrCreateLocalTable`), et tant qu'il est vide il ne désigne aucun décalage
  (`getMinUncommittedNodeOffset` rend `INVALID_OFFSET`). `NodeGroupCollection::clear` remet
  son compte à zéro — avant, le compte survivait au vidage, sans conséquence parce que rien ne
  suivait le commit.
- Rien n'est journalisé pour une ligne locale avant le commit — ou le versement. Les lignes
  versées sont donc au journal AVANT celles du `COPY`, dans l'ordre des décalages.
- Une transaction en erreur refuse tout jusqu'au `ROLLBACK` (T0) : on ne peut pas montrer ce
  qu'elle « voit encore » après un `COPY` refusé.

- **Une transaction « forcée »** (`Transaction::shouldForceCheckpoint`) est durable par le
  point de reprise que sa validation impose, et depuis `37608cf4b` par lui seul : elle
  n'écrit rien au fichier du journal. `TransactionManager::commit` attend d'abord le départ
  des autres (le délai annule avant toute validation), valide en mémoire, puis fait le point
  de reprise sans condition ; s'il échoue, la base refuse tout jusqu'à sa réouverture. Ses
  appelants : tout `COPY` tant que `force_checkpoint_on_copy` vaut `true` ; le `COPY` de
  nœuds qui écarte des lignes ; les créations d'index (vector, fts, spatial), refusées dans
  une transaction explicite.
- **Le journal porte des propriétés, pas des colonnes** : une ligne y est la liste des
  propriétés du catalogue dans leur ordre (la forme du stockage local). Les colonnes du
  stockage ne sont pas les mêmes après un `ALTER … DROP` (l'emplacement de la colonne retirée
  reste). Trois endroits confondent encore la position de la clé et son numéro de colonne
  (ticket du `DROP` d'une colonne déclarée avant la clé).
- **Une ligne écartée par un `COPY`** (`IGNORE_ERRORS`, clé en double ou nulle) est ajoutée
  puis supprimée : elle laisse un trou à son décalage. Les lignes mal formées, elles, sont
  écartées par le lecteur avant l'opérateur et ne consomment rien.
- **Le journal d'une transaction vit en mémoire non évictable** jusqu'à sa validation
  (`LocalWAL`, pages de 4 Kio prises au gestionnaire de tampon), sans borne.

### Le fichier de données et ses pages (10 octobre 2026)

- **Le fichier n'avait pas d'étendue connue.** L'en-tête (`DatabaseHeader`) ne portait que les
  plages du catalogue et des métadonnées ; l'espace libre (`FreeSpaceManager`) est une liste de
  plages ; le nombre de pages en mémoire est **la taille du fichier** à l'ouverture
  (`FileHandle::constructPersistentFileHandle`). Depuis la version 40 du stockage, l'en-tête
  porte `numDataPages`, l'étendue au point de reprise, et l'ouverture en écriture rend à l'espace
  libre ce qui dépasse (`Checkpointer::returnOwnerlessPages`).
- **Un COPY écrit ses pages avant sa validation par groupe plein de nœuds** (131 072 lignes),
  directement dans le fichier, sans page fantôme (`Column::flushData`). En deçà, tout reste en
  mémoire jusqu'au point de reprise. Un témoin de fuite à 60 000 lignes ne voit rien.
- **`removePageIdxAndTruncateIfNecessary` ne tronque pas le fichier** : il réduit le nombre de
  pages en mémoire. Une queue libérée par un point de reprise (`handleLastPageRange`) reste donc
  dans le fichier et était recomptée à l'ouverture d'après, hors de l'espace libre.
- **Les pages prises à l'espace libre ne fuient jamais** : la liste persistée les dit encore
  libres. Seule la fin du fichier fuit.
- **Le rejeu n'alloue ni n'écrit aucune page** (hors l'application des pages fantômes d'un point
  de reprise marqué) : il réinsère en mémoire ; le point de reprise suivant écrit. Le journal
  est logique : aucune page physique n'y est désignée.
- **Pages jamais rendues à l'annulation dans une session qui continue** (lu dans les
  commentaires, non mesuré) : création d'un index plein texte annulée, `ALTER … ADD` annulé sur
  des groupes déjà sur disque, point de reprise annulé. Hors du lot de l'étendue.
- **Mesurer sans harnais d'abord** : un programme nu lié à `build/moteur/src` (annexes,
  `essai-fuite.cpp`) a donné les chiffres en dix minutes là où un témoin mal dimensionné
  aurait menti.

### Ce que pèse le journal (5 octobre 2026)

- **Mesuré, octets de journal** (un `COPY` journalisé depuis un CSV, taille du `.wal` après la
  validation) : une ligne clé + texte de 200 caractères, 270 ; texte de 2 000, 2 070 — les
  chaînes passent presque au poids du CSV. Une relation sans propriété, 75 ; avec deux chaînes,
  143. Un `FLOAT[768]`, 9 236 avant `1c232f318` (12 octets par flottant), 3 074 depuis.
- **Pourquoi** : `ValueVector::serialize` écrit une `Value` par cellule, et `Value::serialize`
  réécrit à chaque fois le type, la nullité et le nombre d'enfants. Pour un tableau, chaque
  élément était une `Value`. Les tableaux de numériques ont leur forme brute ; les autres
  cellules gardent ce coût fixe (non mesuré par colonne).
- **Le drapeau d'une forme de sérialisation ne se devine pas dans le flux** : il est porté par
  le numéro d'enregistrement (40, 42, 45 pour 30, 32, 35). `WALRecord::deserialize` remet le
  type d'origine : le rejeu ne voit que 30, 32, 35.
- **Le guetteur d'un `.wal` ne mesure pas une transaction** : le fichier grossit de validation
  en validation jusqu'au point de reprise automatique (seuil de 16 Mio, regardé après chaque
  validation). « Plus gros journal » est une somme. La sonde `RAG3DB_PROFILE_JOURNAL` rend le
  poids par transaction.
- **Un `COPY` journalisé ne supprime pas le point de reprise** tant que `auto_checkpoint` est
  actif : un paquet qui écrit plus de 16 Mio de journal en déclenche un à sa validation. Les
  données sont alors écrites deux fois. C'est pourquoi la première mesure ne gagne pas de temps.
- **Une transaction forcée n'écrit rien au journal** (`37608cf4b`) : c'est ce qui permet de
  replier en cours de route un `COPY` trop gros — vider le journal local et ne plus y écrire —
  sans rien perdre : le point de reprise de la validation emporte tout, ou rien.
- **`ASSERT` dans une fonction auxiliaire, `x[2] IS NULL`** : l'analyseur refuse `d.v[2] IS
  NULL` ; écrire `list_extract(d.v, 2) IS NULL`.

### L'index de clé primaire (5 octobre 2026)

- **Deux étages.** Sur disque, `HashIndex` : 256 parts (par les bits hauts du hachage), chacune
  un hachage linéaire à cases de 256 octets — 9 entrées par case pour une clé de chaîne, 14 pour
  un entier —, des cases primaires (`pSlots`) et de débordement (`oSlots`), et pour les chaînes
  longues un fichier de débordement. En mémoire, `InMemHashIndex` : les insertions d'une
  transaction, fusionnées au point de reprise (`HashIndex::checkpoint` → `reserve` →
  `splitSlots`, puis `mergeBulkInserts`). Les insertions ordinaires et les `COPY` passent par
  la même fusion.
- **Une clé retirée de l'étage en mémoire doit reculer son compte** (`deleteKey`,
  `836edfc29`) : l'annulation d'un `COPY` retire ses clés une à une par là, et la réservation
  de l'étage sur disque se dimensionne sur ce compte.
- **`reserve` ne divise que s'il manque des cases** : la soustraction était non signée. C'est
  aussi ce qui laisse ouvrir une base dont l'index a été écrit trop grand.
- **`splitSlots` garde ses cases de débordement neuves de côté et les ajoute à la fin** (les
  itérateurs du tableau sur disque ne peuvent pas tenir deux pages de `oSlots`). Une case
  créée dans l'appel peut être redivisée dans le même appel (le niveau monte quand la part
  fait plus que doubler) : tout pointeur vers ces cases doit rester valide à travers un ajout
  — un `std::deque`, depuis `eb2d78e46`. `getNumElements()` sans argument rend le compte
  VALIDÉ (lecture), pas celui de la transaction en cours.
- **Un défaut de ce genre se voit sous ASan en une passe, et en Release par une clé sur
  cent mille.** Pour le reproduire sans le harnais : relire les clés d'une base par
  `MATCH (n:T) RETURN n.k ORDER BY offset(id(n))`, les recharger dans une table neuve du moteur
  nu, chercher chaque clé (`annexes`, `essai-reel.cpp`). Les clés synthétiques ne perdent rien
  à 2 × 50 000 ; elles perdent à 47 000 puis 200 000.
- **Une assertion dans une fonction auxiliaire d'un test ne l'arrête pas** (`ASSERT_TRUE` dans
  un `ok()` qui rend `void`) : un cas peut être « rouge » pour une instruction refusée plus haut
  et non pour le défaut. Lire la première erreur d'un rouge avant de le compter.
- **Une transaction de 200 000 `CREATE` déborde le tampon de la base de test** (« The buffer
  pool is full ») : par lots de 10 000.

## 2. L'index vectoriel (HNSW) et notre greffe

- **Ce que l'index garde en mémoire, hors du graphe** : `HNSWStorageInfo` — le compte des
  lignes reliées (`numCheckpointedNodes`) et deux points d'entrée. Un `COPY` avance le compte
  à sa fin (`finalize`), une insertion ordinaire au commit (`commitInsert` le pose à
  « décalage + 1 »). Les arêtes, elles, sont des relations écrites par la transaction dans
  ses tables locales : l'annulation les défait seule. Depuis `e1049934e` l'annulation prévient
  les index (`Index::rollbackInsert`) et l'index recule son compte ; avant, il restait en
  avance. Sans point d'entrée, recherche et insertion repartent d'une ligne vivante
  (`findLiveNode`).
- **La lecture groupée des vecteurs** (`OnDiskEmbeddings::getEmbeddings`) rend un vecteur par
  décalage demandé, nul pour une ligne qu'elle ne voit pas ; ses appelants indexent par
  position. Avant `35d09c466` le tableau s'arrêtait à la dernière ligne lue.
- **Joignabilité** : rien ne garantit, à l'insertion, qu'une ligne garde une arête entrante ;
  `keepNodeReachable` ne sert qu'à la mise à jour et à la suppression. Une recherche avec
  `efs` supérieur au nombre de lignes visite tout le joignable : une ligne absente pour son
  propre vecteur est orpheline. Le rejeu du journal rebâtit le graphe par l'insertion
  ordinaire — un graphe relu après un point de reprise et un graphe rejoué ne sont pas le
  même.
- La recherche a deux parts : le graphe, pour les lignes jusqu'au compte ; un balayage direct
  pour les lignes au-delà (`searchFromUnCheckpointed`). Un compte en avance prive donc de
  l'une et de l'autre les lignes qu'il couvre à tort.

- L'insertion dans l'index se fait **au commit** (`needCommitInsert`,
  `NodeTable::commit` → `scanIndexColumns` → `OnDiskHNSWIndex::commitInsert`). La
  suppression et la mise à jour sont **notre greffe** (`98e35566a`) et agissent pendant la
  transaction. Les points d'entrée et `numCheckpointedNodes` vivent hors transaction.
- La recherche lit les vecteurs **dans la table**, pas dans le graphe : un graphe aux
  positions périmées rend encore des distances justes, ce qui masque des défauts.
- **Lecteurs** : `NodeTable::lookup<false>` / `lookupMultiple<false>` (lecture des vecteurs)
  et le balayage des relations en mémoire passent sous `NodeGroup::inMemAppendMtx`
  (partagé), exclusif pendant `NodeGroup::append` (A5 bis). Les informations de version et
  l'index des relations en mémoire ont leurs propres verrous (A5).
- **Corrigé** : `finalizeDelete` qui ne s'exécutait jamais ; les arêtes pontées à la
  suppression ; `keepNodeReachable` (un survivant reste atteignable) ; le `ROLLBACK` d'un
  `DELETE`.
- **Défauts connus, non corrigés** (journal §6) :
  - le `SET` d'un vecteur vers un autre perd des lignes (969 sur 1000 ligne à ligne, 532 par
    lots de 512) ; depuis NULL, rien ;
  - une ligne injoignable dès la construction : à 98 et 100 lignes sur
    `embeddings-8-1k.csv`, et toute ligne très loin des autres (500 lignes, une à
    `[9000, 2, 3, 4]`) ; l'élagage différé des voisins est soupçonné, cause non élucidée ;
  - `QUERY_VECTOR_INDEX … RETURN count(*)` rend toujours `k` ;
  - appeler `createRels` depuis `finalizeDelete` plante dans `shrinkForNode` ;
  - sous plusieurs écrivains : conflits d'écriture entre voisins (H3), état corrompu (H4).
- **La mise à jour** (`OnDiskHNSWIndex::update`) retire les arêtes sortantes de la ligne
  (`deleteFromGraph`), répare les points d'entrée, puis la réinsère (`insertInternal`).
  L'état de mise à jour (`HNSWUpdateState`) est **recréé à chaque ligne** : on ne peut rien
  y accumuler d'une ligne à l'autre d'une même instruction. Il n'existe pas de `finalize`
  pour la mise à jour, contrairement à la suppression.
- **Le tampon des vecteurs lus** (`OnDiskEmbeddingScanState`,
  `extension/vector/src/include/index/hnsw_graph.h`) tient 2048 vecteurs à la fois, gérés
  en pile par des poignées (`EmbeddingHandle`) ; aucune ne fuit d'une ligne à l'autre
  (compté).
- **Les arêtes se lisent par `graph::OnDiskGraph`** (`scanNeighbors`, `shrinkForNode`) :
  d'abord les relations validées, puis celles de la transaction (`LocalRelTable::scan`),
  dans les mêmes vecteurs de sortie. Dans une instruction à plusieurs lignes, la lecture des
  arêtes de la transaction rendait le même voisin répété ; cause première non élucidée
  (rapport de session).
- Ladybug n'a ni mise à jour ni suppression dans son HNSW.
- L'étude : `docs/3-octobre-2026-15h47/02-hnsw-sous-plusieurs-ecrivains.md`.

## 3. Le planificateur

- La recherche par l'index de clé primaire n'existe que pour une clé **constante**
  (littéral, paramètre, `CAST` de l'un des deux : `isConstantExpression`,
  `src/optimizer/filter_push_down_optimizer.cpp`). L'opérateur physique évalue la clé une
  seule fois.
- Un `MATCH` après `UNWIND` : tout prédicat qui dépend de la ligne extérieure est mis de
  côté, le motif est joint par produit cartésien puis filtré (`planRegularMatch`,
  `src/planner/plan/plan_subquery.cpp`). `visitCrossProductReplace` rattrape en jointure de
  hachage **seulement** si chaque côté de l'égalité est une colonne nommée — une variable
  simple oui, `STRUCT_EXTRACT(item, …)` non.
- **La recette** (session de l'arbre principal, `dialect::unwind_par_cle`) : `item` voyage,
  chaque clé est extraite dans le `WITH` juste avant son `MATCH` ou son `MERGE`. Vérifier
  par `EXPLAIN` : `HASH_JOIN`, pas `CROSS_PRODUCT`.
- La jointure de hachage balaie encore la table pour se bâtir (0,3 / 0,8 / 5,8 ms par lot de
  300 à 2 000 / 20 000 / 200 000 nœuds) ; la vraie réponse est une recherche par clé par
  ligne, que Ladybug a faite.
- La base de test en mémoire a un tampon de 73 Mo : un « buffer pool is full » y est un
  plafond de test.

## 4. Les verrous à venir

Note : `docs/3-octobre-2026-15h47/01-note-de-conception-les-verrous.md` (les trois écarts
tranchés en tête du §1). Ce qui est décidé en plus, par message :
- une ressource de verrou est un identifiant **plat** (genre, table, valeur), pour pouvoir
  vivre un jour dans un segment partagé (marche B) ; les détenteurs sont des transactions,
  pas des fils ; l'interface est étroite (prendre, tout relâcher, lister les attentes) ;
- deux genres dès V1 : ligne par clé, **index d'une table** (exclusif pour tout écrivain
  d'une table indexée, de sa première écriture au commit) ; H3 et H4 en sont les témoins ;
- l'annonce d'une clé d'une table indexée prend aussi l'index, sans que l'appelant le sache ;
- sous l'option A (instantané par transaction), un écrivain qui a attendu peut échouer sur
  « could not serialize » : le verrou d'index n'est vraiment utile qu'avec l'annonce en tête
  ou en auto-commit ;
- noms reconnus par le banc : « deadlock », « lock » + « timeout » (`CALL lock_timeout`),
  « could not serialize », « Interrupted », `CALL acquire_locks('Table', [clés])`,
  « duplicated primary key » ;
- la maintenance de l'index au commit vient juste après V2.
Les témoins sont au banc (`test/transaction/concurrence/lock_bench_test.cpp`), rouges.

### A3′, câblée (10 octobre 2026) — ce qu'il faut savoir pour A4′ et V2

- **Où prendre un verrou** : `Transaction::acquireLocks(span<LockRequest>)` /
  `acquireLock(resource, mode)` (`src/transaction/transaction.cpp`) portent toute la politique :
  rien hors du mode multi-écrivains, en reprise ou sans contexte (`usesLocks`) ; première
  tentative sans attendre ; puis attente au plus `lock_timeout`, interruption regardée, sous un
  `TaskScheduler::BlockingWait` ; `TransactionManagerException` avec `LockManager::describe`.
  A4′ n'a qu'à appeler ça. Les verrous sont rendus par `clearTransactionNoLock`, validée ou
  annulée ; une instruction en échec annule la transaction (`rollbackAfterStatementFailure`),
  donc rend.
- **La clé d'une ligne pour le gestionnaire** : `NodeTable::lockKeyOf(pkVector, pos)` = la
  valeur imprimée (`getAsValue(pos)->toString()`), pour que le message soit lisible et que A4′
  (qui tient le décalage, pas la clé) puisse la reconstruire en relisant la colonne de clé.
- **Deux visibilités existent**, et il faut savoir laquelle on veut : l'instantané
  (`isVisible(transaction, offset)`) pour lire ; le dernier état validé
  (`isVisibleToLatestCommit`, `Transaction::LATEST_COMMITTED_TS` = `START_TRANSACTION_ID − 1`
  comme startTS) pour l'unicité. Toute la chaîne (`VersionInfo`, `ChunkedNodeGroup`,
  `NodeGroup`, `NodeTable`) a la forme `(startTS, transactionID, row)`. **La même visibilité
  doit servir à l'insertion et à la validation** (`getUniquenessVisibleFunc`) : sinon la
  validation lève après l'ajout des lignes aux groupes, transaction à moitié validée.
- **Les clés quittent l'index à l'exécution, pas à la validation** : `PrimaryKeyIndex::delete_
  (Transaction*, …)` ne fait rien ; c'est `NodeTable::delete_` qui… non : c'est
  `HashIndexLocalStorage::deleteKey` par `PrimaryKeyIndex::delete_(ValueVector*)`, appelé à la
  suppression — et si la clé était dans `localInsertions` (non encore au point de reprise), elle
  est **retirée tout de suite**, sans visibilité ; sinon elle va dans `localDeletions`. Le
  stockage local de l'index est **partagé par toutes les transactions** (un par index, pas par
  transaction) ; les clés non validées d'une transaction, elles, vivent dans sa
  `LocalNodeTable` jusqu'à sa validation (`commitInsert`).
- **Une attente ne doit pas tenir un fil ouvrier de l'ordonnanceur**, ni surtout le mutex d'une
  tâche : `ProcessorTask::run` exécute `initGlobalState` sous `taskMtx`, et `registerThread`
  (pris sous le mutex de l'ordonnanceur) le prend aussi — une attente là gèle tout. Ce qui doit
  attendre pour tout le plan se prend **avant l'ordonnancement, sur le fil du client**
  (`PhysicalOperator::acquireLocksBeforeExecution`) ; une attente par ligne, dans un opérateur,
  passe par `TaskScheduler::BlockingWait` (fil de remplacement). Même cause, non traitée :
  l'attente d'un point de reprise forcé à la validation (`stopNewTransactionsAndWaitForOthers`)
  tient un fil ouvrier — son délai de 5 s en est le filet.
- **Le banc tolère l'attente** : `writeInTurn` / `commitInOrderByEvents` ont des délais de
  « settle », pas d'échec ; `holderAndWaiter` a maintenant `waiterRollsBack`.
- **A4′ (10 octobre, nuit)** : `NodeTable::lockRowForWrite(transaction, offset, key)` prend la clé
  en exclusif puis `throwIfWrittenByAnotherCommitAfterSnapshot` ; `lockKeyOfRow` relit la clé
  par le décalage ; `RelTable::lockEndpoints(transaction, src, dst, mode)` pour les deux
  extrémités (partagé à la création, exclusif à l'écriture) puis
  `throwIfDeletedByAnotherCommitAfterSnapshot` ; `Transaction::LatestCommittedView` pour lire le
  dernier état validé le temps d'un balayage (DELETE, DETACH DELETE). « Validé après mon
  instantané » = `startTS < version < START_TRANSACTION_ID` dans les chaînes d'`UpdateInfo` et
  les `VersionInfo` (`ChunkedNodeGroup::wasWrittenByCommitAfter`). Le tableau des sémantiques :
  page 07 §0.
- **Fabriquer un état que le moteur ne produit plus** : programme nu contre la bibliothèque
  d'AVANT, gardé au dépôt avec sa base et son journal (`journal_with_duplicate_key/`), comme
  `journal_before_raw_arrays/`. Le faire **avant** de rebâtir.

## 5. Les amonts

- **Kuzu** : notre origine (renommage `c647fbb33`). Défauts d'origine trouvés : le rejeu
  après un point de reprise sans l'extension, le `DROP` d'index au rejeu, le rejeu d'une
  mise à jour qui ignore les index, la recherche par clé limitée aux constantes.
- **Vela** (tags `vela-master-2026-09-03`, `vela-checkpoint-2026-09-03`) : histoire commune
  jusqu'au 10 octobre 2025 ; rien sur le planificateur ni sur ces défauts.
- **Ladybug** (tag `ladybug-main-2026-08-31`) : histoire **séparée** de la nôtre, espace de
  noms `lbug`, même arborescence — on porte, on ne cherry-pick pas. Trouvé ce soir :
  `e92346c97` et sept correctifs (recherche par clé par ligne pour `MATCH`) ; `d2db8acb4`,
  `1ba0cc540` (gardes des index non chargés, reprises dans la garde 1). **Pas regardé** : ce
  qu'il a fait de la mise à jour et de la construction dans son HNSW, et de la reprise.

## 6. Essayé sans succès — à ne pas refaire tel quel

- **A5 bis, coût de la garde** : le verrou exclusif du groupe (double le temps de quatre
  recherches parallèles) ; une garde à bandes, une case par fil lecteur (aucun gain : le
  coût n'est pas la dispute d'une ligne de cache) ; la garde tenue par lot
  (`annexes/a5bis-variante-par-lot.patch`, aucun gain mesurable sous charge). Le poste n'a
  ni `perf` ni `valgrind`.
- **`SET` aligné sur `CREATE`** pour un champ nul : rejeté, des tests de l'amont fixent la
  rigueur de `SET` (`annexes/set-comme-create-et-ses-tests.patch`).
- **Mesurer un coût sous la charge des autres sessions** : une passe d'ingestion faite sous
  une charge de 20 a vu l'embarquement passer de 94 à 144 s.
- **Marquer la table « modifiée » dans `NodeTable::dropIndex`** : retiré, le test ne
  rougissait pas sans.
- **Isoler la croissance du `MERGE` avec la liste en littéral** : le temps est dominé par
  l'analyse de la requête, rien n'en sort.

- **`SET` de vecteur, pistes fermées** : une fuite de poignées de vecteurs (il n'y en a
  pas) ; les doublons de vecteurs (vingt lignes mises sur le vecteur d'une autre restent
  joignables) ; recontrôler après chaque ligne tout ce que l'instruction a touché (l'état
  est recréé à chaque ligne, l'accumulation ne vit pas) ; un témoin du défaut de lecture des
  relations locales par `MATCH` ou par `OnDiskGraph` sur un cas simple (vert avant comme
  après le correctif : il ne prouve rien).

## 7. Les mesures gardées

| Mesure | Résultat | Où |
|---|---|---|
| Coût d'A5 bis, recherche vectorielle seule, dos à dos | dim. 8 : +3 % à un fil, +17 % à quatre ; dim. 256 : +6 %, +21 % | `annexes/vector_search_cost_scratch_test.cpp` |
| Formes de `UNWIND … MATCH` par la clé | table au journal §6 | `annexes/pk_match_forms_scratch_test.cpp` |
| Lots de relations quand la table grossit | plat ; le coût suit le nombre de nœuds | `annexes/rel_insert_growth_scratch_test.cpp` |
| `SET` de vecteur et lignes joignables | table du rapport de session | `annexes/vector_set_reachability_scratch_test.cpp` |
| `e2e_idempotent_registration` en boucle | 18 vertes sur 20, avec ou sans `f5acca417` | `annexes/boucle-idem.sh` |
