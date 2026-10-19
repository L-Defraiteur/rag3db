# V2-1 : les lecteurs pendant le point de reprise — ce que dit le code, et ce qui est tranché

Suite de la page 17 (§5, « ce qui reste à trancher avant V2-1 »). Carte lue le 19 octobre 2026
sur master f0e1fcc45, lignes citées à cette révision. Le but du lot V2-1 : un **lecteur** ne
doit ni attendre un point de reprise, ni le faire échouer ; les écrivains attendent encore
(comme une validation forcée aujourd'hui) ; témoin `CheckpointNoStall.ReadDuringACheckpointDoesNotWait`
(`test/transaction/concurrence/checkpoint_no_stall_test.cpp`, rouge connu).

## 1. Ce qui, ligne par ligne, empêche un lecteur de lire pendant l'écriture

Onze obstacles, trouvés dans le code (pas dans la page 17) :

1. **La porte** : `beginTransaction` attend `checkpointPending` avant même de regarder le type
   (`transaction_manager.cpp:58`, `waitWhileCheckpointPendingNoLock` :448-458) ; un BEGIN
   explicite attend par `waitUntilNewTransactionsAreAllowed` (:461, `client_context.cpp:539`).
   Un lecteur nouveau attend donc pendant l'attente des départs **et** pendant toute l'écriture.
2. **Le départ de tous** : `stopNewTransactionsAndWaitForOthersNoLock` (:319-378) attend que
   `activeTransactions` soit vide, lecteurs compris, borné à 5 s puis échec « Timeout waiting for
   active transactions to leave ». Un lecteur long fait échouer le CHECKPOINT ; une demande
   automatique est reportée sous lui (:175-181).
3. **Le verrou public tenu pendant l'écriture** : `checkpointNoLock` appelle `writeCheckpoint`
   sous `mtxForSerializingPublicFunctionCalls` (:570). Un lecteur ne peut ni commencer, ni
   valider, ni annuler, ni rafraîchir son instantané (:91) avant `allowNewTransactionsNoLock`.
4. **L'effondrement des versions** : le point de reprise n'écrit que « le dernier validé »
   (`DUMMY_CHECKPOINT_TRANSACTION`, `node_group.cpp:464`) et efface versions et mises à jour
   (`resetVersionAndUpdateInfo` :566 ; `csr_node_group.cpp:736, :1440`). Un lecteur à instantané
   plus ancien perdrait ce qu'il voyait.
5. **Le remplacement des blocs sans protection** : `chunkedGroups.clear` puis `appendGroup`
   détruit les anciens `ChunkedNodeGroup` (`node_group.cpp:483-484`) ; les `ColumnChunk` sont
   **déplacés** hors de l'ancien bloc (`chunked_node_group.cpp:52-62`, depuis `node_group.cpp:563`) ;
   `persistentChunkGroup` du CSR est un `unique_ptr` réaffecté (`csr_node_group.h:324`,
   `csr_node_group.cpp:740, :777, :784, :1331`). Le `nodeGroupKeepAlive` du lot du rollback sous
   lecteurs (`table.h:39`) tient le **NodeGroup**, pas ses blocs : il ne protège rien ici. Un
   balayage régulier reprend un pointeur brut à chaque appel (`NodeGroup::scan`, :221-264) et
   garde entre deux appels `chunkedGroupIdx`, des `SegmentState` copiés et des `const Column*`
   bruts (`node_table.cpp:266-280`).
6. **La mutation en place de ce qu'un lecteur lit** : `persistentData.setToInMemory()` et
   `resize` sur l'objet persistant (`column.cpp:451-452`) ; min/max et numValues
   (`column.cpp:364-366, :425`) ; le vecteur `data` d'un `ColumnChunk` scindé
   (`column_chunk.cpp:318-327`) ; les vecteurs `NodeTable::columns` (`node_table.cpp:1056-1060`)
   et `RelTableData::columns` (`rel_table_data.cpp:306-311`) ; `dataTypes` et `types`
   (`node_group.cpp:488-496`, `node_group_collection.cpp:220-224`).
7. **La renumérotation du catalogue partagé, sans version** : `vacuumColumnIDs`
   (`property_definition_collection.cpp:46-52`, appelé à `node_table.cpp:1064` et
   `storage_manager.cpp:244`) mute l'entrée que tout le monde lit. Les `columnIDs` d'un plan
   physique ou d'un `TableScanState` liés avant désigneraient d'autres colonnes.
8. **La copie des pages ombres dans des cadres lus** : `applyShadowPages` fait un `memcpy` dans
   le cadre **sans verrou et sans changer la version de page**
   (`shadow_file.cpp:75-77`, `buffer_manager.cpp:466-474`) ; une `optimisticRead` concurrente
   (:225-235) ne le détecterait pas. Le commentaire le dit : « assume this is only called with a
   single thread ».
9. **Les pages de l'ancien bloc réutilisables dès la fin** : `freePageRange` ne rend rien tout de
   suite (`page_manager.cpp:24-32`, liste « non validée »), mais `finalizeCheckpoint`
   (`checkpointer.cpp:96`, `free_space_manager.cpp:194-202`) **évince ces pages du tampon** et
   les rend libres avant la fin de `writeCheckpoint`. Un lecteur qui tiendrait d'anciennes
   métadonnées lirait des pages réallouées par la transaction suivante.
10. **Les verrous internes tenus longtemps** : `NodeGroupCollection::checkpoint` tient le verrou
    de la collection pour tous les groupes (`node_group_collection.cpp:216`) ; `NodeGroup::checkpoint`
    tient `chunkedGroups.lock()` pour tout le groupe (`node_group.cpp:459`), le même que
    `NodeGroup::scan` (:223) et `getNodeGroupShared` (`node_group_collection.h:64`). Même la porte
    ouverte, un lecteur attendrait la table entière.
11. **Entre processus** : `ReadersLock` exclusif de la marque CHECKPOINT à la suppression du
    journal (`checkpointer.cpp:227, :111`) ; inchangé par V2-1 (page 17 §5, troisième point).

## 2. Ce qui est tranché pour V2-1

**Le périmètre.** V2-1 ouvre la porte aux **lecteurs nouveaux pendant l'écriture**. Les lecteurs
déjà ouverts au départ du point de reprise **partent encore** (attente bornée, comme
aujourd'hui) : c'est l'obstacle 4, l'instantané, qui relève de V2-2. Pendant l'écriture aucun
écrivain ne tourne (ils attendent toujours), donc l'état « dernier validé » que le point de
reprise écrit est exactement l'instantané de tout lecteur ouvert pendant lui : **aucune version
à garder en V2-1**. Le témoin `ReadDuringACheckpointDoesNotWait` lance sa lecture 30 ms après
le départ du CHECKPOINT, sans autre transaction : c'est ce cas.

**(a) La porte et le verrou (obstacles 1, 2, 3, 10).**
- `checkpointPending` se dédouble : *retenir les écrivains* (posé comme aujourd'hui, jusqu'à la
  fin) et *retenir tout le monde* (posé seulement pendant l'attente des départs, levé quand
  `activeTransactions` est vide et que l'écriture commence). Un `beginTransaction` READ_ONLY ne
  regarde que le second ; un WRITE regarde les deux. Un lecteur ouvert pendant l'écriture ne
  bloque pas la fin du point de reprise : rien ne l'attend.
- `writeCheckpoint` **ne s'exécute plus sous le verrou public** : `checkpointNoLock` pose l'état,
  relâche `publicLock` le temps de l'écriture (un drapeau `checkpointWriting` garde les autres
  CHECKPOINT et les écrivains dehors), le reprend pour `allowNewTransactionsNoLock`. Un lecteur
  commence, valide (`clearTransactionNoLock`) et annule sous le verrou public sans attendre
  l'écriture. Une demande automatique écrite par « le dernier à partir » suit le même chemin.
- Les verrous internes : `NodeGroupCollection::checkpoint` prend la liste des groupes sous le
  verrou puis **le relâche** et traite chaque groupe sous son propre verrou ; `NodeGroup::checkpoint`
  ne tient `chunkedGroups.lock()` que pour **lire** la liste des blocs au départ et pour
  **l'échanger** à la fin, pas pendant la copie et l'écriture (le gros du temps). Un balayage
  entre les deux voit l'ancienne liste, cohérente.

**(b) Le bloc neuf à la place de la réécriture en place (obstacles 5, 6, 8).** C'est la décision
du second point de la page 17 §5, prise ainsi :
- Le point de reprise d'un groupe **construit un `ChunkedNodeGroup` neuf** (persistant, ON_DISK)
  à partir de l'ancien bloc persistant et des blocs en mémoire, sans toucher aux objets anciens :
  plus de `moveColumnChunk` hors de l'ancien bloc, plus de `setToInMemory`/`resize` sur
  `persistentData`, plus de min/max récrits en place. Un segment inchangé est **repris tel quel**
  (mêmes pages, mêmes métadonnées copiées) ; un segment changé est **réécrit hors place** sur des
  pages neuves (`checkpointColumnChunkOutOfPlace`, qui existe) ; la réécriture en place par
  pages ombres (`checkpointColumnChunkInPlace`, `ShadowUtils::updatePage` sur une page de
  colonne) **disparaît pour les colonnes**. Le fichier ombre ne porte plus que le catalogue, les
  métadonnées et l'en-tête, que personne ne lit par le tampon pendant une transaction : l'obstacle
  8 tombe sans toucher au `memcpy`.
- Coût accepté : un segment qui n'avait que quelques valeurs mises à jour est réécrit en entier
  (hors place) au lieu de quelques pages ombres. C'est le prix d'un lecteur qui ne s'arrête pas ;
  le fichier ne grossit pas durablement (les anciennes pages reviennent, voir (d)).
- L'échange : `NodeGroup::checkpoint` remplace la liste des blocs **en une affectation sous le
  verrou** (`chunkedGroups` : clear + append, comme aujourd'hui, mais sur des `shared_ptr` que
  les balayages tiennent). Les balayages **tiennent le bloc** qu'ils lisent : `NodeGroupScanState`
  garde un `shared_ptr<ChunkedNodeGroup>` du bloc courant (comme le CSR en mémoire le fait déjà,
  `findChunkedGroupSharedFromRowIdx`), pris sous le verrou à l'entrée dans un bloc, rendu à la
  sortie ; les `SegmentState` copiés et les `const Column*` de l'état de balayage restent
  valides parce que les objets qu'ils désignent (`ColumnChunkData` de l'ancien bloc, `Column`
  de la table) ne sont plus mutés ni détruits tant que le bloc vit. Les vecteurs `columns` de
  `NodeTable`/`RelTableData` et `dataTypes` sont remplacés par **échange d'un `shared_ptr` sur un
  vecteur immuable** (un balayage tient celui qu'il a pris à `setToTable`).
- Le CSR : `persistentChunkGroup` devient `shared_ptr` ; `createNewPersistentChunkGroup` construit
  le neuf sans déplacer les colonnes de l'ancien ; l'ancien meurt avec son dernier lecteur. Le
  `reclaimStorage` sur l'ancien passe au destructeur (voir (d)).

**(c) Les trous de colonnes (obstacle 7).** Premier point de la page 17 §5, tranché : **un numéro
de colonne ne change jamais**. `PropertyDefinitionCollection` garde, pour une propriété
supprimée, son `columnID` dans une liste de numéros retirés ; `vacuumColumnIDs` ne renumérote
plus (il devient la compaction, appelée seulement par un point de reprise **sans aucune
transaction ouverte**, lecteurs compris : le chemin d'aujourd'hui, devenu l'exception, et pas en
V2-1 : la compaction attendra l'ADD paresseux, page 16 forme (b), qui lit « un bloc absent vaut
le défaut »). Le vecteur `columns` d'une table garde l'entrée vide à ce numéro (un `unique_ptr`
nul), le `ChunkedNodeGroup` neuf garde un `ColumnChunk` vide à ce numéro, et le prochain ADD
comble le premier trou. Ce que ça change pour l'ADD paresseux : rien à renuméroter, un numéro
retiré n'est jamais rendu avant la compaction.

**(d) Les pages de l'ancien bloc (obstacle 9).** Le dernier point de la page 17 §5, tranché :
`freePageRange` d'un segment remplacé **n'est plus appelé par le point de reprise** mais par le
**destructeur de `ColumnChunkData` persistant** quand il a été remplacé (un drapeau « remplacé,
pages à rendre » posé par le point de reprise, lu par le destructeur) ; `reclaimStorage` d'un
bloc entier suit le même chemin. Les pages vont dans la liste « non validée » du FSM **au moment
de la mort** du bloc, c'est-à-dire au départ de son dernier lecteur, et deviennent libres au
`finalizeCheckpoint` du point de reprise **suivant** ; `finalizeCheckpoint` n'évince du tampon
que des pages dont plus aucun objet vivant ne porte le numéro. Si le bloc meurt pendant le point
de reprise lui-même (aucun lecteur), les pages sont rendues au point de reprise d'après, pas à
celui-ci : un cycle de plus avant réutilisation, aucune page lue par un vivant n'est réallouée.
Au rejeu (arrêt brutal entre deux points de reprise), les pages des anciens blocs sont
« sans propriétaire » : `returnOwnerlessPages` (`checkpointer.cpp:321-335`) les récupère déjà.

**(e) Inchangé en V2-1.** `ReadersLock` entre processus (obstacle 11) ; le point de reprise de
fermeture ; la file des validations (A7) et la demande automatique (A8) ; l'effacement des
versions à l'écriture (obstacle 4 : V2-2) ; les validations forcées (COPY hors journal, index)
qui gardent leur attente.

## 3. L'ordre des pièces du lot, et le témoin de chacune

| pièce | ce qu'elle change | témoin |
|---|---|---|
| V2-1a | (c) : numéros de colonnes stables, trous, `vacuumColumnIDs` devenu compaction jamais appelée en V2-1 ; `renumberColumns` disparaît | `test/transaction/` : DROP d'une colonne du milieu puis CHECKPOINT : les numéros des autres ne bougent pas (catalogue sérialisé relu), un plan lié avant le point de reprise lit encore les bonnes colonnes après ; ADD après DROP comble le trou |
| V2-1b | (b) et (d) : bloc neuf, segments inchangés repris, changés réécrits hors place, plus de pages ombres pour les colonnes ; balayages qui tiennent leur bloc ; pages rendues à la mort du bloc | `test/transaction/` : un balayage ouvert (itérateur `QueryResult` non consommé) pendant un CHECKPOINT qui réécrit son groupe lit jusqu'au bout les anciennes valeurs, sans erreur ; le fichier ne grossit pas au-delà d'un cycle (taille après deux points de reprise) ; ASAN sur ce témoin |
| V2-1c | (a) : la porte à deux niveaux, l'écriture hors du verrou public, les verrous internes relâchés | `CheckpointNoStall.ReadDuringACheckpointDoesNotWait` (banc) passe au vert et sort de `known_red.txt` ; `BeginDuringACheckpointDoesNotWait` reste rouge connu (V2-2) ; un écrivain pendant l'écriture attend toujours (témoin : BEGIN WRITE pendant un CHECKPOINT long rend la main après lui) |

V2-1a d'abord parce que (b) construit des blocs neufs à numéros fixes ; V2-1b avant V2-1c parce
qu'ouvrir la porte sans tenir les blocs est l'accès après libération que le lot du rollback sous
lecteurs a déjà rencontré. Chaque pièce est un lot poussé seul, vert sur la batterie, rouge
connu mis à jour dans le commit.

## 4. Ce que je demande à l'orchestration avant d'écrire V2-1a

- La forme (b) (bloc neuf, plus de réécriture en place des colonnes) a un coût d'écriture sur
  les points de reprise qui ne touchaient que quelques pages d'un gros segment : accepté, ou
  mesurer d'abord sur le banc des vingt écrivains ?
- Les trous de colonnes (c) sont aussi la forme de l'ADD paresseux (page 16) : V2-1a les pose,
  l'ADD paresseux les reprend tels quels. D'accord pour que V2-1a précède l'ADD paresseux dans ma
  file, puisque c'est la même pièce ?
