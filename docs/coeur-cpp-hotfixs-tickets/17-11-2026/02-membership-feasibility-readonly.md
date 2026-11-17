# Faisabilité membership : trois préconditions relues, aucune implémentation

Autorisation SOURCE SEULE C7/55c59a3875 ; rattachement au même ticket08nov
publié95e0bc2a61 AVANT cette note. Fetch master figé le17nov :
99fef92ce6ee9a0007eaf99c7c3a6a3789061c57. Diff avec71fbe786fcf0b2314e72b3eb4b8484faf590d78c
VIDE sur src/storage, src/transaction, src/include/storage,
src/include/transaction et extension/vector/src. Les références ci-dessous
s'appliquent à ces deux versions ; pas de résultat natif déduit du diff.
Lecture fichiers uniquement, aucun prototype/test/build/DB/natif/SDK/format.

## Bilan de faisabilité

Les API actuelles apportent des briques utiles, mais les trois préconditions
ne sont pas satisfaites par la seule composition proposée en588/944. **STOP
avant réservation/implémentation** : tableM en NodeTable pendant commit,
acquisition ancienne par OID et rétention après DROP, transition de format sur
WAL non vide exigent des décisions cœur explicites. Aucun « faisable » global.

### 1. Écritures dérivées au commit : ordre attesté et lacune

Transaction::commit (src/transaction/transaction.cpp:234) pose committing,
joue les preCommitActions, puis LocalStorage::commit. Ensuite seulement :
WAL local COMMIT + flush/sync pour le chemin ordinaire, puis undoBuffer::commit
publie les versions. En validation forcée, le WAL local est volontairement omis
et le checkpoint atomique porte la durabilité. rollback:287 annule undoBuffer
AVANT localStorage. TransactionManager::commit:108 encadre ces opérations,
annule un commit refusé et impose le checkpoint forcé ; les preCommitActions
attendent les autres transactions, lecteurs compris (ne pas en ajouter pour
ce diagnostic sous prétexte de lecture).

LocalStorage::commit (local_storage.cpp:64) relève d'abord les nombres de
relations locales préexistantes, puis **fige nodeTableIDs** avant de valider la
première table de nœuds. NodeTable::commit (node_table.cpp:876) appende les
lignes, déclare rowsAreCommitted, produit leur WAL, puis appelle les index avec
les offsets finaux via UncommittedIndexInserter ; la table locale est finalement
vidée. HNSW::commitInsert:775 peut alors créer des tables locales de relations.
LocalStorage ne relève relTableIDs qu'APRÈS tous les commits de nœuds : ces
nouvelles relations sont donc prises en charge, avec remap des seules relations
antérieures au commit. C'est une garantie structurelle pour les relations,
pas pour une nouvelle NodeTable M.

Conséquence précise pour la recommandation588 : une NodeTable M créée/touchée
dans commitInsert après le relevé ne rejoint pas nodeTableIDs. La précréer vide
n'établit pas davantage l'ordre source→M : si M a déjà été validée avant le
callback source, les lignes ajoutées ensuite n'ont pas de second passage prouvé.
Les offsets source finaux ne sont acquis qu'au commit ; préremplir les clés avec
les offsets provisoires n'est pas une solution sans remap explicite des valeurs.
Le même Transaction* ne suffit donc pas à démontrer publication/rollback/WAL.

Mutateurs à couvrir : commitInsert775, finalize813, catchUpCommittedAfter847,
updateInternal1276, delete_/finalizeDelete, insertInternal et réparations de
topologie. La primitive doit assurer présent/absent de façon idempotente sous
la sérialisation d'écriture existante : PK seule n'est pas un upsert, et ignorer
une erreur arbitraire n'est pas l'idempotence. M indique l'incorporation, pas le
nombre de passages par insertInternal. Réparation d'arêtes sans changement de
membre ne doit pas modifier l'ensemble. L'ordre UPDATE index avant colonne
n'est acceptable que si toute l'instruction reste atomique et annulable.

Périmètre minimal À PROPOSER, pas réservé :

- Conserver NodeTable M nécessite une phase explicite de publication des tables
  de nœuds dérivées après les sources et avant les relations/WAL/publication,
  ordonnée/bornée, sans revalider aveuglément toutes les tables. Chemins concernés :
  src/storage/local_storage/local_storage.cpp et son header, NodeTable::commit et
  callbacks HNSW. Ordre, offsets, statistiques, PK, undo et WAL doivent être
  prouvés ensemble ; coût de dépendances et d'un passage supplémentaire.
- Alternative à comparer AVANT tout nouveau choix : une table REL interne de
  membership indépendante des deux graphes, un marqueur par offset source.
  Les relations créées au commit sont déjà collectées après les nœuds. Cela
  éviterait cette nouvelle phase NodeTable, mais exige idempotence/unicité,
  offsets finaux et DELETE/NULL/recovery qualifiés. Des marqueurs dans une
  troisième table ne sont PAS des self-arêtes ajoutées au graphe ANN. C'est une
  option documentaire, pas une recommandation remplacée sans arbitrage ; elle
  ne résout ni rétention DROP ni compatibilité de l'ancien écrivain.

Témoins à prépublier : plusieurs sources et index, offsets sparse déplacés par
un autre commit, bâti+catch-up+UPDATE répété même membre, rollback après M avant
fin des relations et échec WAL. Vérifier source/E/M/arêtes/PK et absence de
publication partielle. Aucun de ces témoins n'est écrit ou joué ici.

### 2. Vieux lecteur : catalogue, holder, table et pages

Briques attestées : CatalogSet::traverseVersionChainsForTransactionNoLock
(catalog_set.cpp:156) choisit une version par transaction ; IndexHolder porte
createdTS/droppedTS et isVisibleTo (index.h:260–283). TransactionManager calcule
oldestReaderTS depuis snapshotFloorTS de tous les actifs (transaction_manager.cpp:653)
et Checkpointer::checkpointStorage:152 le transmet au StorageManager. Les
NodeGroup ont un mécanisme de générations retenues/retireGenerationsBefore.
Ce sont des garanties de versions de données conservées, pas la preuve de
rétention de toutes les identités retirées.

Lacunes dans les chemins effectivement relus :

- NodeTable::reclaimDroppedIndexes (node_table.cpp:1586) efface tout holder
  isDropCommitted, sans comparer droppedTS au plancher des lecteurs ; appelé
  par checkpoint:1012. Une première capture APRÈS ce checkpoint ne peut pas
  s'appuyer sur un shared_ptr que le lecteur n'a jamais acquis.
- StorageManager::reclaimDroppedTables (storage_manager.cpp:177), appelé à la
  fin du checkpoint:275, teste la présence dans DUMMY_CHECKPOINT_TRANSACTION,
  appelle reclaimStorage et tables.erase. Le conteneur possède des unique_ptr
  (storage_manager.h:127), getTable:82 rend un pointeur brut. Aucune rétention
  de table/pages retirées par ce seul accès n'est attestée. NodeTable::reclaimStorage:1085
  réclame groupes et PK ; partager uniquement l'Index ne retient pas ces objets.
- CatalogSet::getEntryOfOID (catalog_set.cpp:267) compare l'OID demandé à celui
  de la TÊTE AVANT de parcourir ses versions. Après reCREATE homonyme avec une
  nouvelle OID, la recherche d'une ancienne OID ne traverse pas cette chaîne.
  L'accès par OID nécessaire ne peut donc pas être déclaré acquis sans examiner
  et préciser cette résolution historique. Ce constat source n'est pas une
  reproduction native de toutes les formes DROP/CREATE.
- CheckpointNoLock attend les écrivains, pas tous les anciens READ ONLY
  (transaction_manager.cpp:635) : leur absence ne peut pas justifier ces retraits.

Périmètre minimal À PROPOSER : conservation des holders et tables/pages selon
un horizon MVCC incluant le plus vieux snapshot, résolution historique par OID
sans dépendance au nom courant, puis acquisition possédée/lease de scan jusqu'à
la fin de l'observation. Chemins cœur : catalog_set.cpp/header, index.h et
NodeTable::reclaimDroppedIndexes, StorageManager.{h,cpp} ; PageAllocator/gestion
pages seulement si le report de reclaimStorage n'est pas suffisant. Ne pas
transformer unique_ptr en shared_ptr seul : le lecteur ancien sans acquisition
préalable exige une rétention globale indexée par horizon, et les pages doivent
rester non réutilisables. Coût : mémoire/disque et versions retenus par les
lecteurs longs ; collecte seulement après leur départ. Garantir aussi que le
checkpoint suivant ne sérialise pas comme vivantes des tables déjà supprimées.

Témoins distincts à prépublier : vieux snapshot sans capture M puis DROP/reCREATE,
checkpoint+GC et PREMIÈRE lecture ; scan déjà acquis suspendu entre lots puis
même interleaving. Vérifier E/M/OIDs anciens, nouvelle instance indépendante,
aucun accès nom et aucun UAF/pages réutilisées. Si la capacité annoncée ne peut
plus être honorée, erreur réelle, pas NULL silencieux ni writerlock de secours.
Aucune modification de rétention générale autorisée par cette note.

### 3. Ancienne base avec WAL non vide : ce que le bump garantit ou non

DatabaseHeader::readStorageVersion (database_header.cpp:22) refuse une version
plus récente, lecture et écriture. WALReplayer::dryReplay (wal_replayer.cpp:427)
refuse un FormatRecord plus récent avant rejeu. WAL::logCommittedWAL
(wal.cpp:30) synchronise avant publication, et tronque/synchronise au point
précédent si l'écriture échoue. Ce sont des barrières réutilisables lorsqu'elles
ont effectivement été écrites et rendues durables.

Mais WAL::initWriter:105 écrit l'en-tête/FormatRecord seulement si taille0, puis
append à la taille existante. Recovery WALReplayer::replay:218 : branche dernier
CHECKPOINT rejoue shadow et retire le WAL ; branche dernier COMMIT:345 rejoue
les enregistrements, puis **tronque au dernier COMMIT sans supprimer le WAL**.
Database::init:215 appelle recover sans imposer ensuite un checkpoint d'upgrade.
Il existe donc dans le code une branche de réouverture avec ancien WAL non vide
conservé ; le nouvel initWriter n'y pose pas automatiquement un nouveau marqueur.
Cette lecture démontre l'absence de garantie par LE SIMPLE BUMP, pas un crash
observé ni un défaut d'une implémentation membership qui n'existe pas.

Option minimale recommandée à étudier : transition d'upgrade explicite, avant
capacité/écriture M, sous les garanties de checkpoint/recovery existantes :
rejouer l'ancien WAL, checkpoint durable du format nouveau, fermeture/rotation
du WAL existant et nouvel en-tête de version, puis seulement autoriser le
nouveau descripteur. Une ouverture READ ONLY ne migre pas. Une base en mémoire
n'a pas cette transition disque. Coût d'un checkpoint et indisponibilité
contrôlée à l'upgrade, à annoncer ; ne pas le déclencher dans SHOW_INDEXES.

Alternative : nouveau FormatRecord durable avant les premières écritures M
sur WAL append, après validation des règles de dryReplay et de la frontière
transactionnelle. Il faut prouver que l'ancien moteur voit TOUJOURS le marqueur
avant un payload inconnu, que les crashes/rollback ne le retirent pas en laissant
M durable et que le header checkpointé reste cohérent. Ne pas choisir cette
solution par une ligne ajoutée dans initWriter : une telle preuve n'est pas faite.

Périmètre minimal à coordonner : storage_version_info.h, WAL::{initWriter,
logCommittedWAL,reset/clear}, WALReplayer::replay/dryReplay, point d'admission
upgrade de Database/StorageManager et Checkpointer si nécessaire. Pas tous
forcément à modifier : choisir la transition puis réduire la surface d'après la
preuve. Aucun changement de format/numéro ni essai de crash autorisé ici.

Témoins à prépublier : base ancien format avec WAL non vide au dernier COMMIT,
recovery→transition→premier M, crashes avant/après chaque borne durable et avant
premier checkpoint de M ; vieux reader ET writer refusent le nouveau header ou
WAL sans tenter l'index. Crash T_A/T_B/READY ; ancien index toujoursNULL malgré
upgrade global ; trailer tronqué/inconnu erreur, jamais table manquante masquée.
Inclure WAL vide, dernierCHECKPOINT, sync/truncate refusés. Aucun résultat acquis.

## Décision nécessaire avant code

La proposition de table interne demeure une orientation, pas une primitive déjà
composable sans modification du cœur. Les trois surfaces sont explicites ; ne
pas démarrer un format bump, un prototype ou une instrumentation pour les
« vérifier ». C7/cœur doit arbitrer : phase dérivée NodeTable ou autre
représentation, politique de rétention/résolution historique, transition durable
de format. Chaque choix doit revenir avec réservation ciblée et témoins
prépubliés, puis recette/RO/GO distincts pour toute validation native.

Le budget partagé de lecture doit inclure M/allocations/PK/versions/E et vecteurs,
plusieurs index et prepared statements réutilisés. Limite de budget=>NULL
sans partiel ; annulation/corruption/I/O restent distinctes. Les coûts d'écriture,
de rétention et d'upgrade s'ajoutent ; aucune mesure ou latence inventée. Les
quatre fixtures historiques restent inchangées et le ticket reste ouvert.
