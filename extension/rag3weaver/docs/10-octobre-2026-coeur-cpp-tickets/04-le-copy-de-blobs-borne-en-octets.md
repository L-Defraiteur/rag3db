# Le COPY de blobs borné en octets — la forme (a), à valider avant le code

Pour la session cœur C++ (le stockage est le sien) et l'orchestration. Seconde session cœur C++,
11 octobre 2026. Ticket : `docs/tickets/2026-10-11-copy-de-blobs-en-point-de-reprise-force-sous-un-petit-tampon.md`.

Ce qui est **mesuré** est marqué comme tel. Le reste vient d'une lecture par agent, à
`fe051eafb` (lignes vérifiables), et **n'est pas encore éprouvé**.

## 1. Le défaut (mesuré)

Sous 256 Mio de tampon, `force_checkpoint_on_copy=true`, blobs d'1 Mio lus d'un CSV :

- sept COPY successifs de 16 blobs, jusqu'à 112 Mio dans la table : verts ;
- un seul COPY de 112 blobs : rouge, « buffer pool is full », en 50 s.

La pile, relevée par gdb : `NodeBatchInsert::appendIncompleteNodeGroup` →
`InMemChunkedNodeGroup::append` → `StringChunkData::appendStringColumnChunk` →
`DictionaryChunk::appendString` → `ColumnChunkData::resize`.

## 2. Pourquoi (lecture)

- Chaque fil du COPY tient un groupe en mémoire de 2^17 lignes de capacité. Il ne s'écrit que
  plein (`isFull()`, `node_batch_insert.cpp:193-196`) : il n'y a **aucun critère d'octets**.
- En fin de flux, les groupes incomplets sont **recopiés** dans un groupe partagé, sous
  `sharedState->mtx` (`appendIncompleteNodeGroup`, `:269-290`). Le dictionnaire grandit par
  `bit_ceil`, et `ColumnChunkData::resize` alloue le nouveau tampon avant de libérer l'ancien.
  Au pic, on a le groupe partagé (64 Mio de capacité), plus le groupe du fil en copie (~56 Mio),
  plus le nouveau tampon (128 Mio) : cela atteint les 256 Mio.
- `resetToEmpty()` **garde les tampons alloués** (`column_chunk_data.cpp:185-194`) : un groupe
  vidé reste à sa capacité de pointe.

## 3. Ce que le stockage permet aujourd'hui (lecture)

- Un groupe de nœuds régulier (`NodeGroup`, pas `CSRNodeGroup`) n'a qu'**un** groupe persistant :
  `chunkedGroups[0]`, s'il est ON_DISK. Les suivants sont en mémoire, par paquets de 2 048 lignes
  (`findChunkedGroupIdxFromRowIdxNoLock`, `node_group.cpp:662-678`). Le point de reprise relit et
  recopie tout ce qui est en mémoire (`checkpointInMemAndOnDisk`, `:509-555`, « The first chunked
  group is the only persistent one »), et `serialize` exige un seul groupe (`:606`).
- Le COPY sait écrire un groupe de nœuds **entier** directement sur le disque
  (`directFlushWhenAppend`, `node_group_collection.cpp:133-154`, hors du verrou, parce que
  personne d'autre ne peut réserver dans un groupe plein). Il ne sait pas écrire une **partie**
  d'un groupe de nœuds, puis la suite.
- Conséquence : écrire tôt la première partie d'un groupe, puis continuer en mémoire, **ne borne
  pas** la mémoire. Les lignes suivantes restent en mémoire jusqu'au point de reprise, qui les
  recopie. Pour 112 blobs d'1 Mio, qui tiennent dans un seul groupe de nœuds, le premier vidage
  prend B octets, puis les ~80 Mio restants sont recopiés deux fois.

## 4. La forme proposée

**A. Mesurer.** Ajouter `InMemChunkedNodeGroup::getEstimatedMemoryUsage()`, la somme des
`ColumnChunkData::getEstimatedMemoryUsage()` de ses colonnes, en O(1) par colonne. C'est la
capacité allouée, ce qui compte pour le tampon. Le budget B se déduit de la limite du tampon et
du nombre de fils, environ `limite / (3 · (fils + 1))`, puisque chaque groupe peut tripler au pic.

**B. Dans l'opérateur (NodeBatchInsert).**

- Écrire un groupe local dès qu'il est plein **ou** au-delà de B (`copyToNodeGroup`).
- Dans `appendIncompleteNodeGroup`, écrire directement un groupe local qui dépasse déjà B, au lieu
  de le copier dans le groupe partagé : c'est cette copie qui échoue. Appliquer le même test au
  groupe partagé, et dans `finalize`.
- Après un vidage décidé par le budget, **remplacer** le groupe par un neuf au lieu de
  `resetToEmpty()`, en gardant l'ordre : écrire, puis alimenter l'index de clé (il relit la clé
  dans le groupe du COPY), puis remplacer.

**C. Dans le stockage : un vidage partiel** (`appendToLastNodeGroupAndFlushWhenFull`).

- (i) Le groupe de nœuds cible est vide : `flush` puis `merge`, **sous `nodeGroups.lock()`**.
  C'est le chemin du vidage direct, sans la condition « plein ». Hors du verrou, un autre fil
  réserverait la suite avant le `merge`, avec un `startRow` d'annulation faux.
- (ii) **Le cœur de la proposition** : le groupe de nœuds n'a qu'un groupe, ON_DISK, et **ce COPY
  l'a créé** (son premier décalage ≥ `numRowsBeforeCopy`). On vide les nouvelles lignes en
  segments, et on les **ajoute aux segments de la tête** :
  - `ColumnChunk::appendSegments` ;
  - `ChunkedNodeGroup::appendFlushed`, qui met à jour le nombre de lignes et la capacité, et qui
    fait `versionInfo->append(txn, ancien, n)` ;
  - `NodeGroup::numRows += n`.

  Le tout se fait sous le verrou de `chunkedGroups` et `inMemAppendMtx` en exclusif. La tête
  reste « le seul groupe persistant » : les invariants du §3 tiennent, elle a seulement plus de
  segments.
- (iii) Sinon (une tête issue d'un point de reprise antérieur), le chemin actuel, en mémoire.
  C'est une **limite à écrire** : un très gros COPY de blobs dans un groupe déjà commencé peut
  encore manquer de tampon.

**D. Facultatif, peu coûteux.** Dans `InMemChunkedNodeGroup::flushInternal`, vider chaque segment
de `split(true)` dès sa création au lieu de les matérialiser tous avant, ce qui divise le pic du
vidage par environ deux.

## 5. Les risques, et ce qui les tient

| risque | ce qui le tient |
|---|---|
| annulation d'un COPY dont la tête a reçu des segments | (ii) est limité aux groupes créés par ce COPY : l'annulation, parcourue en ordre inverse, finit à `startRow` 0, donc `truncate(0)` puis retrait du groupe. Les pages viennent de l'allocateur optimiste, qui les rend toutes. À ajouter quand même : tronquer aussi les segments sur disque à `startRow > 0`. |
| ordre entre fils | les vidages partiels se font sous le verrou. Ce verrou sérialise les écritures du COPY sur cette table, donc il coûte du parallélisme, à mesurer. Une autre voie : un ordre par ticket. |
| lecteurs concurrents | les lignes d'un groupe créé par ce COPY sont invisibles aux autres transactions : leur sélection est vide, et la colonne n'est pas lue. |
| COPY journalisé (`logInsertedRowsToWAL`) | il relit par `NodeGroup::scan`, qui sait lire une tête à plusieurs segments. Les témoins `JournaledCopyReadBackTest` et `OwnerlessPages*` doivent rester verts. |
| IGNORE_ERRORS, clés en double | la suppression du doublon se fait par version et marche sur une tête ON_DISK. Avec des lignes écartées, le journal est abandonné pour un point de reprise forcé, comme aujourd'hui. |
| index de clé primaire | ne pas libérer le groupe avant `indexBuilder->insert`. |

## 6. Les témoins

- Rouges aujourd'hui : `BlobCopyCheckpointTest.OneForcedCopyBeyondAQuarterOfTheBufferPool`, et
  une variante journalisée (`force_checkpoint_on_copy=false`).
- À ajouter : la mémoire du tampon mesurée sous un plafond (le pic du COPY sous B fois une
  constante) ; un COPY annulé (ROLLBACK) après des vidages partiels, sans pages orphelines ni
  lignes restantes ; un COPY à plusieurs fils dont les décalages et la clé primaire restent justes.
- La confirmation de l'arbre principal, rejouée en blobs.

## 7. Ce que je demande

1. **Au cœur** : (ii), allonger les segments d'une tête ON_DISK créée par le même COPY, est-il
   acceptable ? Sinon, quelle autre forme, si un vidage partiel suivi de lignes en mémoire ne
   borne rien ?
2. **À l'orchestration** : (iii), un COPY dans un groupe déjà commencé, reste une limite écrite,
   d'accord ? Le filet du produit (des COPY de blobs bornés en octets, chez A) le couvre en
   attendant.
