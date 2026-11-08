# Un SIGSEGV dans optimisticRead sous lecture concurrente pendant une indexation

- **État** : ouvert (8 novembre 2026), en cours (cœur c++) : la cause n'est pas prouvée ; ce qui
  suit est lu sur le core et le désassemblage.
- **Famille** : moteur
- **Gravité** : plantage (l'hôte de Lucie, relancé par systemd 46 s plus tard).
- **Atteignable en service** : oui — vu en service le 8 novembre à 8 h 50 m 11 s.
- **Touche rag3weaver** : par le moteur (l'hôte rag3weaver-backend meurt).
- **Pour** : cœur c++.

## Ce que c'est

L'hôte du portable (rag3weaver-backend, binaire posé à 8 h 45, lib
`rag3db-build-dadbf0041…`) meurt de SIGSEGV (SEGV_MAPERR) sur le fil `demon-5`, dans
`rag3db::storage::BufferManager::optimisticRead(FileHandle&, unsigned, std::function<void(unsigned char*)> const&)`
(rip `0x7f1a74f12c73` = `librag3db.so + 0xd12c73`). Au même instant : la montre réindexait des
fichiers de `code-rag3db-depot` et de l'arbre d'essais de c7 (un verrou de 4,7 s sur
`backend.rs`), deux fils lucivy fusionnaient des segments, l'équipier lisait des fichiers.

## Ce que disent le core et le désassemblage

Le core (32 Gio, tronqué) ne rend pas la pile ; gdb rend les registres du fil fautif :
`rdx = 0`, `rax = 2`, `rcx = 0x13b54`, `rsi = 0x1cc`.

L'instruction fautive (`d12c73: mov (%rdx),%rdx`) est la boucle de
`ConcurrentVector::operator[]` (`src/include/common/concurrent_vector.h`), en ligne dans
`optimisticRead`, qui suit la chaîne des index de blocs (`index = index->nextIndex` tant que
`indexNum > 0`) : elle a trouvé un `nextIndex` nul. Le décalage dans le bloc vaut `0x1cc`
(460), le numéro de bloc `0x13b54` (80 724, blocs de 1 024 états de page) : la page demandée est
vers 8,3 × 10⁷, soit près de 330 Gio de fichier, très au-delà de toute base de Lucie.

**Lecture** : ce n'est pas la course d'agrandissement de `ConcurrentVector::resize` (« Not
thread-safe ») — une page fraîchement ajoutée n'est qu'à un bloc au-delà du dernier publié, pas à
quatre-vingt mille. Le lecteur portait un numéro de page faux : très probablement lu dans les
métadonnées d'un segment déjà libéré (une lecture après libération), ou dans une structure
réécrite sous lui. Les candidats, depuis les lots V2 et ceux de la semaine :
- la durée de vie des blocs et des segments tenus par un balayage (V2-1b, V2-1c, V2-2 : un lecteur
  qui garde un pointeur nu sur un segment ou un état de colonne que l'échange du point de reprise
  ou le retrait des générations fait mourir) ;
- la copie-échange du COPY borné en octets (`NodeGroup::extendFlushedHead`, G2b) ;
- les listes CSR et leur compaction (`compactDeadRowsNoLock`, la liste des blocs prise avant
  l'index), lues par la recherche vectorielle (`OnDiskGraph::scanFwd`).

## Le second plantage (9 h 05 m 49 s, pid 102525)

Même lib, même instruction (`…12c70`, la même boucle `nextIndex`), et les registres :
`rdx = 0`, `rax = 2`, **`rcx = 0x13b54` (80 724), le même numéro de bloc**, `rsi = 0x124` (292).
Deux plantages à quinze minutes d'écart demandent donc le **même** bloc d'états de page, à des
décalages voisins : la page fausse n'est pas un octet de hasard dans une mémoire écrasée, c'est
une valeur calculée ou lue, reproductible. Aucun plantage en dix heures sous 17a094d12 : le défaut
est entré entre 17a094d12 et dadbf0041 (les huit commits moteur du 7 novembre ; les deux têtes
pour la lecture d'une page sont la copie-échange du COPY borné en octets et le retrait des
générations).

Le refus du rejeu du .wal de code-rag3db-depot est un défaut distinct (le rejeu d'une mise à jour
d'une relation insérée plus tôt dans la même transaction, chez hotfixs) : rejoué sous un shell
ASAN de dadbf0041, il ne montre aucune erreur mémoire.

## L'ASAN du 8 novembre (cœur c++, à luciepc, build ASAN de dadbf0041)

- **Propres, aucun rapport** : les cas V2 (CheckpointUnderReaders, CheckpointNewSegment,
  CheckpointOpenReader, RollbackUnderReaders ×3), VectorSearchDuringInsertTest,
  ReadersDuringDeleteTest, HalfAppliedCommit ; au banc C7_RandomMix et les cinq
  LateRelTableRewrites (la compaction CSR).
- **Un débordement dans le CREATE_VECTOR_INDEX concurrent** (TwentyWriters.DdlEveryFifteenSeconds :
  l'insertion HNSW lit une ligne au-delà de la colonne mise en cache par `_CACHE_COLUMN`) : un
  défaut à part, ticket de hotfixs.
- **Un heap-use-after-free dans `OnDiskHNSWIndex::shrinkForNode`**, trouvé par le témoin du mélange
  de hotfixs (`VectorCopyUnderASmallPoolTest.TheIndexReadsItsVectorsWhileTheHeadIsExchanged`,
  `test/transaction/blob_checkpoint_test.cpp`) : à la finalisation de l'index par un COPY sur une
  table indexée, le vecteur du nœud est pris par pointeur dans le vecteur de données de l'état de
  lecture, puis `getEmbeddings` des voisins, sur le même état, réalloue ce vecteur
  (`ListAuxiliaryBuffer::resize`) : `metricFunc` lit 3 072 octets d'un tampon libéré. C'est le
  chemin de la montre (réindexation, COPY dans une table d'embarquements indexée). Correctif : la
  branche `vecteur-du-noeud-copie` (le vecteur du nœud copié avant les voisins) ; le témoin passe
  sous ASAN avec lui, sans aucun rapport.

**Ce qui n'est pas prouvé** : que cette lecture d'un tampon libéré produise le numéro de page de
330 Gio des deux cores. En Release elle lit des flottants périmés et n'écrit rien. La preuve sera
l'hôte qui tient la même charge sans plantage après le rebâti ; le ticket reste ouvert jusque-là.

## Témoin

Aucun encore. La voie : rejouer sous ASAN les cas qui lisent pendant qu'un point de reprise et des
écritures tournent (`CheckpointUnderReaders.*`, `CheckpointOpenReader.*`,
`RollbackUnderReaders.*`, `VectorSearchDuringInsertTest.*`, le scénario des vingt clients du
banc), puis un témoin qui reproduise le mélange vu : lectures plein texte et vectorielles, une
indexation qui écrit et valide, des points de reprise automatiques. Un heap-use-after-free dans
`ColumnChunkData` ou un `ChunkState` désignerait la pièce.

## Pour le fermer

La cause prouvée par ASAN ou par un témoin rouge, le correctif, le témoin vert, et l'hôte qui tient
la même charge.
