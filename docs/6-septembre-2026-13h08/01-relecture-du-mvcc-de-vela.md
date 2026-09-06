# Relecture du MVCC de Vela — ce que l'arbre contient vraiment

**6 septembre 2026, 13 h 08.** La relecture que personne n'avait faite, et que
[`5-septembre-2026-16h13/01`](../../extension/rag3weaver/docs/5-septembre-2026-16h13/01-les-objectifs-et-leur-ordre.md)
posait en préalable de la vraie concurrence. Cartographie faite par un agent
de lecture sur `HEAD = 80697d3c4`, relue et mise en forme ; les références
`fichier:ligne` sont celles de cet arbre. Rien n'a été modifié.

Elle complète [`3-septembre-2026-14h42/06-reperage-vela.md`](../3-septembre-2026-14h42/06-reperage-vela.md)
et [`07-lecteurs-concurrents.md`](../3-septembre-2026-14h42/07-lecteurs-concurrents.md),
qu'elle ne contredit sur aucun point.

## L'avertissement qui change la lecture de tout le reste

**Le port du travail de Vela n'est pas dans cet arbre.** Trois lignes en ont
été reprises (le verrou de fichier, §1). Le commit `05d6f0f2a` « port Vela
non-blocking checkpoint + concurrent writes » existe dans le dépôt d'objets
(`git log --all`) mais n'est ancêtre d'aucune branche ni de `HEAD`. Ce qu'on
appelait « le MVCC de Vela » est donc, ici, **le MVCC amont de Kuzu**. Le
travail transactionnel de Vela reste dehors ; c'est lui qu'il faudra relire
le jour où on le prend. Le remote `vela` est configuré, deux tags épinglent
leurs commits (`vela-master-2026-09-03`, `vela-checkpoint-2026-09-03`).

## 1. Le verrou de fichier

`src/common/file_system/local_file_system.cpp:135-147` : `fcntl(fd, F_SETLK,
…)`, `F_WRLCK` pour un écrivain, **non bloquant** — le second processus reçoit
une `IOException "Could not set lock on file"` immédiate, il ne s'endort pas.

Le drapeau est décidé à **un seul endroit**, `src/storage/storage_manager.cpp:45-56` :
un écrivain prend `O_LOCKED_PERSISTENT_FILE`, une ouverture en lecture seule
**ne prend aucun verrou**. C'est le report de trois lignes du correctif Vela
`87bf0bef9` (`4b6f9baa2`, 3 septembre). Le chemin `READ_LOCK` de
`file_handle.cpp:36` est donc du code mort.

| ouverture | verrou | face à un écrivain en place |
|---|---|---|
| écriture | `F_WRLCK` sur le fichier de données | refus immédiat |
| lecture seule | aucun | succès |

Le seul autre site qui prend un verrou est `src/storage/shadow_file.cpp:103` :
le rejeu des pages fantômes rouvre le fichier en `WRITE_LOCK`. C'est **le**
point de collision lecteur/écrivain, gardé en amont (`:92-95`, refus
transitoire « Couldn't replay shadow pages under read-only mode »). Le WAL et
le fichier fantôme ne sont jamais verrouillés.

Contrat perdu, à dire : avant, un lecteur tenait un `F_RDLCK` qui empêchait un
écrivain de s'installer. Ce n'est plus vrai.

## 2. Le gestionnaire de transactions

`src/transaction/transaction_manager.cpp`. Deux `std::mutex` — **intra-
processus** : `mtxForSerializingPublicFunctionCalls`, `mtxForStartingNewTransactions`.

**Un seul écrivain**, `:35-39` : `if (!enableMultiWrites && hasActiveWriteTransactionNoLock()) throw …`.
Le second écrivain reçoit une **exception, pas une attente**. Aucune file,
aucune condition variable, aucun réessai.

**`debug_enable_multi_writes`** (`settings.h:99-104`, `db_config.cpp:30`) :
son effet est **exactement d'une ligne** — sauter ce `throw`. Il n'active
aucune détection de conflit, aucun verrouillage de ligne. Le commentaire
l'assume : *« temporary solution to make tests of multiple write transactions
easier »*. Ce qui reste derrière est opportuniste et a posteriori :
`VectorVersionInfo::delete_` lève sur suppression concurrente
(`version_info.cpp:104,116`) ; `checkWWConflict` pour le catalogue
(`catalog_set.cpp:26-32`). **Rien pour les insertions ni les mises à jour.**
Quiconque l'active en croyant activer un mode multi-écrivains obtient des
corruptions silencieuses.

**Horloge** : `lastTransactionID` et `lastTimestamp`, `transaction_t` **non
atomiques**, portée processus (`transaction_manager.h:68-69`). `startTS` = le
`lastTimestamp` courant à l'ouverture ; `commitTS` = `++lastTimestamp` au
commit. `START_TRANSACTION_ID = 1 << 63` sépare identifiants en vol et
estampilles validées.

## 3. Le MVCC lui-même

| structure | granularité | ce qu'elle porte |
|---|---|---|
| `VersionInfo` (`version_info.h`) | un par `ChunkedNodeGroup` | insertions / suppressions |
| `VectorVersionInfo` (`version_info.cpp:14-71`) | un par vecteur de 2 048 lignes | une estampille par ligne, avec raccourci « même version pour tout le vecteur » |
| `UpdateInfo` / `VectorUpdateInfo` (`update_info.h`) | par colonne × vecteur | **chaîne de versions** `prev`/`next`, un `shared_mutex` par nœud |

**La granularité effective est la ligne**, l'allocation est par vecteur. Les
mises à jour sont la seule partie déjà pensée pour plusieurs fils.

**Visibilité** (`version_info.cpp:216-240`) : `insertion == transactionID ||
insertion <= startTS`, et symétriquement pour la suppression. Isolation
obtenue : **snapshot**, qui repose entièrement sur l'ordre total de
`lastTimestamp` — **local au processus**. Le passage validé écrase les
estampilles en place (`setInsertCommitTS`, `:123-146`) sans atomique : correct
sous un seul écrivain, course de données dès qu'on relâche.

**Undo** : un `UndoBuffer` par transaction, annoté non thread-safe
(`undo_buffer.h:66`) ; `VersionRecordHandler` aussi (`version_record_handler.h:15`).

**Ramassage des versions : le checkpoint, et lui seul.**
`ChunkedNodeGroup::resetVersionAndUpdateInfo()` (`chunked_node_group.cpp:125-132`)
n'est appelé que depuis le chemin de checkpoint. **Aucun GC incrémental, aucun
horizon de transaction, aucune notion de plus vieux lecteur vivant.**

## 4. Le WAL et le checkpoint

Deux WAL : `LocalWAL` par transaction en mémoire, déversé au commit dans le
`WAL` partagé (un `std::mutex`, un fichier). `clear()` tronque, `reset()`
supprime. **Il n'y a pas de rotation.**

**Déclenchement** (`transaction_manager.cpp:67-72`) : `CHECKPOINT` explicite,
`forceCheckpointOnClose`, ou `localWAL + wal > checkpointThreshold` (16 Mo).

**Il bloque, les deux.** `checkpointNoLock` (`:159-178`) appelle
`stopNewTransactionsAndWaitUntilAllTransactionsLeave()` (`:116-135`) : attente
active par pas de 500 µs que `activeTransactions` soit **vide, lecteurs
compris**, exception au-delà de 5 s. **Un lecteur de longue durée fait échouer
le checkpoint.** Pendant le checkpoint, le mutex de sérialisation est tenu :
toute nouvelle transaction attend, lecture comprise.

Subtilité de portée (`:168-172`) : le verrou `lockForStartingTransaction` est
détruit à l'accolade du `try`, donc relâché **avant** `writeCheckpoint()`. Pas
un bug aujourd'hui — l'autre mutex suffit — mais la barrière ne tient pas la
durée qu'on croit en la lisant.

`writeCheckpoint` (`checkpointer.cpp:65-96`) : storage → catalogue → en-tête →
`logCheckpointAndApplyShadowPages` (point de non-retour) → finalisation →
`wal.reset()`, `shadowFile.reset()`.

**Les lecteurs d'un autre processus** pendant un checkpoint ne sont drainés par
rien : c'est la fenêtre mesurée dans le doc 07 — 6 à 8 refus sur 60, tous
résolus en ~3 nouvelles tentatives, 0 lecture incohérente.

## 5. Vela

**Dans cet arbre : trois lignes**, avec leur commentaire honnête
(`storage_manager.cpp:47-56`) : *« this buys concurrent READERS only. Two
writer processes are still refused, and the reader gets no isolation from the
writer »*.

**Hors de l'arbre, `05d6f0f2a`** apporte l'inverse point par point de ce que
décrivent les §2-4 :

| ce que Vela apporte | ce qu'il remplace ici |
|---|---|
| rotation du WAL vers `.wal.checkpoint` gelé | `WAL::reset()`, pas de rotation |
| `stopNewWriteTransactionsAndWaitUntilAllWriteTransactionsLeave` | drainage de **toutes** les transactions |
| instantanés MVCC du catalogue, `snapshotTS` épinglé | `Catalog::version` non atomique |
| suivi par époque | `version != 0` |
| `updateFrameIfPageIsInFrame` avec boucle CAS | application directe des pages fantômes |
| `shared_mutex schemaMtx` sur `NodeTable` | aucun verrou sur `NodeTable` |
| `StorageManager` en `shared_mutex` | `std::mutex` |

Deux conclusions du doc 06, confirmées : la fusion est peu coûteuse (55/58
fichiers seuls, trois conflits, une demi-journée) ; et **leur concurrence est
intra-processus** — `std::mutex mtxForCheckpoint`, `condition_variable`,
`atomic activeTransactionCount`. Leur écrivain prend toujours le `F_WRLCK`.
Plusieurs processus écrivains : **non**. Leur branche
`storage/concurrent-checkpoint-recovery` porte encore six commits de juillet
non fusionnés chez eux (« recover partial checkpoint installs », « fence
failed rotated checkpoints »).

## 6. Points d'appui pour un arbitre inter-processus

**Ce qui a déjà un verrou en mémoire** : `CatalogSet` (`shared_mutex`),
`StorageManager` (`mutex`), `FileHandle` (`shared_mutex`), `UpdateInfo`
(`shared_mutex` ×2), `DiskArray`, `HashIndex`, `NodeGroupCollection` /
`NodeGroup` (`UniqLock`), `RelTable::relOffsetMtx`, `WAL`, `ChunkedNodeGroup::spillToDiskMutex`.
**Ce qui n'en a pas** : `NodeTable`, `RelTable` (hors offsets), `Table`,
`TableCatalogEntry`. Aucun verrou par table.

**Où un verrou fin pourrait se poser** : à la table, rien à greffer — la
porte est `StorageManager::getTable` ou l'entrée de catalogue ; au **node
group**, la granularité la mieux préparée (collection déjà verrouillée, un
verrou hors processus par `(tableID, nodeGroupIdx)` s'aligne sur une structure
existante) ; à la ligne, l'estampille existe mais la détection est incomplète
et faite après la mutation.

**Ce qui suppose un écrivain unique en dur** — la liste qui décide :

1. **L'horloge** : `lastTimestamp` non atomique, portée processus. L'ordre
   total dont dépend tout le MVCC n'existe pas hors processus.
2. **L'offset de nœud est dérivé, pas réservé** : `startNodeOffset =
   nodeGroups->getNumTotalRows()` **au commit** (`node_table.cpp:608`). Il n'y
   a rien à verrouiller parce qu'il n'y a pas d'allocateur. Deux commits
   concurrents écrivent au même endroit ; et `Transaction::getUncommittedOffset`
   mappe les lignes locales dans cet espace **avant** le commit.
3. **`RelTable::reserveRelOffsets`** : bien verrouillé, par un mutex processus.
4. **`LocalStorage`** : un seul fil d'insertion par transaction, annoté.
5. **`UndoBuffer`, `VersionRecordHandler`** : annotés non thread-safe.
6. **`BufferManager`** : file d'éviction en `std::atomic`, versions de page —
   correct entre fils, **aveugle entre processus**. Deux processus qui mappent
   le même fichier ont deux caches incohérents ; la garantie tient aujourd'hui
   par le `F_WRLCK`.
7. Compteurs de version du catalogue et du gestionnaire de pages : `uint64_t` nus.
8. Le fichier fantôme : état partagé sur disque, appliqué sous verrou
   exclusif, sans protocole — juste un refus.

## Ce que ça décide

**Bâtir un arbitre inter-processus par-dessus le MVCC existant — verrou hors
processus + MVCC en place — n'est pas plausible en l'état.** La couche
transactionnelle est bien celle qu'il faut ouvrir, et l'ouvrir veut dire
sortir du processus trois choses qui y sont enfermées par construction :
**l'horloge, l'allocation d'offsets, le cache de pages**.

Précisément :

- Un arbitre externe qui **sérialise les écrivains** — un seul à la fois, mais
  *choisi* hors processus, avec **file d'attente** au lieu du refus sec de
  `transaction_manager.cpp:36` — est **plausible et peu coûteux**. Il ne touche
  pas au MVCC : il déplace le `F_WRLCK` de « premier arrivé, les autres
  échouent » vers « premier arrivé, les autres attendent ». C'est un gain de
  confort réel, pas un gain de concurrence.
- Un arbitre qui laisse **deux écrivains en parallèle sur des ressources
  disjointes** demande de remplacer, dans l'ordre : l'horloge (partagée hors
  processus), l'allocation d'offsets (réservation hors processus, pas
  `getNumTotalRows()` au commit), l'invalidation du `BufferManager` (protocole
  de cohérence de cache entre processus). Aucun des trois ne se contourne par
  un verrou.
- **Prendre le travail de Vela reste la bonne première étape, pour ce qu'il
  est** : la concurrence *intra*-processus correcte (rotation du WAL,
  checkpoint non bloquant pour les lecteurs, instantanés de catalogue). Il ne
  fait pas un pas vers l'inter-processus, et il faut cesser de l'espérer de
  lui. Le doc 06 §8 le disait : *« Si on la veut, c'est notre travail. »*

**Les trois risques concrets :**

1. **L'horloge est le point dur, et elle est invisible.** Deux processus avec
   deux horloges produisent des lectures **silencieusement fausses**, pas des
   erreurs. Un arbitre qui verrouille les ressources sans partager l'horloge
   donne une base corrompue qui a l'air de marcher.
2. **L'offset de nœud n'a pas d'allocateur.** Le créer remonte jusqu'au
   mapping des relations locales. Ce n'est pas une greffe locale.
3. **Le `BufferManager` n'a aucune notion d'invalidation externe.** Dès deux
   écrivains, un processus sert des pages périmées depuis son cache sans s'en
   apercevoir. Le seul filet est le refus du rejeu des pages fantômes — un
   refus, pas une invalidation, et seulement pendant la fenêtre du checkpoint.

Et une dette de nom : `debug_enable_multi_writes` promet plus qu'il ne fait.
À renommer, ou à faire lever un avertissement, avant tout chantier de
concurrence.

## Ce que ça change pour l'ordre côté Rust

C1 à C4 (`extension/rag3weaver`, réconciliation du 6 septembre) restent
justes : la fermeture de ressource, la marque par niveau, le catalogue en
lecture, la réclamation sur la dette — tout cela vit au-dessus de la base et
ne suppose rien d'elle. Ce qu'ils ne peuvent pas faire seuls, c'est deux
écrivains rag3db : là, l'ordre est **fusion de Vela** (une demi-journée,
intra-processus propre) → **arbitre externe qui met les écrivains en file**
(confort, pas de concurrence) → et seulement ensuite le monument : horloge,
allocateur d'offsets, cohérence de cache. Le doc 06 §8 avait la bonne phrase
et cette relecture en donne la mesure.
