# Un SIGSEGV dans optimisticRead sous lecture concurrente pendant une indexation

- **État** : ouvert (8 novembre 2026), en cours (cœur c++) : un mécanisme est prouvé (l'étendue, plus bas), le premier événement non ; ce qui
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

## Ce que dit la lib corrigée en service (8 novembre, midi)

Le correctif est sur master en `f7df88003` (moteur `cbc06c3e1`), dans la lib rebâtie avec
`a312f823e` et le rejeu de hotfixs. L'hôte de luciepc tourne dessus depuis 12 h 07 : aucun
plantage en une heure (NRestarts 0). **Ce n'est pas une preuve** : la réindexation forcée de
code-lourd n'a duré que 4 min 30 s, 137 741 vecteurs sur 137 943 réutilisés, sans la charge
concurrente de 8 h 50 (montre, fusions lucivy, équipier). Elle a exercé le chemin du correctif (un
COPY dans une table indexée, la finalisation de l'index), pas la course.

La preuve est un banc « rouge d'abord » (G, hôte d'essai 7991 à luciepc, base neuve copiée du
manifeste de code-lourd, réembarquement complet, search_code dense et plein texte et read_file en
boucle pendant l'index) : sous dadbf0041 d'abord, puis sous cbc06c3e1 au moins le double du temps
qu'il aura fallu pour faire planter dadbf0041. S'il ne fait pas planter dadbf0041 en une heure, la
course demande autre chose que ce que nous savons reproduire, et la preuve reste l'usage en service
sous surveillance. Le portable de Lucie passe sur la lib corrigée en parallèle (décision de c7 :
le mécanisme est prouvé par le témoin ASAN, le repli est automatique).

## L'étendue de code-essais-c7 (8 novembre, 16 h 50 à 17 h 30, cœur c++)

c7 a trouvé un fichier creux : `code-essais-c7.rag3db` mesure 318 Gio apparents pour 1,8 Gio réels.
Les lectures ci-dessous portent sur une copie reflink de 16 h 52, en lecture seule, et sur une
copie jetable.

| Ce qui est lu | Valeur |
|---|---|
| Étendue de l'en-tête (numDataPages, écrit par cbc06c3e1) | 455 380 pages |
| Pages du fichier | 83 108 352 |
| Données écrites (SEEK_DATA) | de 0 à 455 380, puis 256 pages à partir de 83 108 096 |
| Segments au-delà de l'étendue | deux, `Scope_Chunk` `embedding__granite_278m_data`, 96 pages chacun, à 83 108 096 et 83 108 192 |
| Entrée libre au-delà de l'étendue | 83 108 288, sur 64 pages |

Les segments sont dans les métadonnées du point de reprise, pas dans le journal : la base relue sans
son `.wal` les montre aussi. Aucun truncate du moteur n'allonge le fichier de données. Le trou vient
d'écritures posées au-delà de numPages.

**Le mécanisme, reproduit sur la copie jetable**
- À l'ouverture en écriture, `Checkpointer::returnOwnerlessPages` (lot du 10 octobre, 0aed3c4b5)
  rend libre tout ce qui va de l'étendue à la fin du fichier, soit 455 380 sur 82 652 972 pages.
  Cette plage couvre les 192 pages des deux segments vivants.
- Le point de reprise suivant écrit une liste libre qui contient ces pages vivantes. Une allocation
  future les réécrira.
- `FreeSpaceManager::handleLastPageRange` baissait numPages, et avec lui les états de page, au début
  de la dernière plage libre qui touche la fin du fichier. Quand cette plage couvre des segments
  vivants, numPages passe sous eux et l'étendue écrite aussi. C'est l'état de la copie de 16 h 52.
  Le lecteur suivant de ces segments sort de la chaîne des blocs de `ConcurrentVector` : un
  `nextIndex` nul, la signature des deux cores, toujours au même bloc.
- La base d'origine, fermée par c7 à 17 h 14 après une ouverture en écriture, déclare 83 108 352 pages
  et porte les 192 pages vivantes dans une entrée libre. c7 l'a mise de côté entière dans
  `~/.cache/rag3db-moteur-notes/essais-c7/etat-ferme-17h14/`.

**Ce qui n'est pas expliqué** : le premier événement, entre 0 h 08 et 8 h 50, sous 17a094d12 ou
dadbf0041, qui a posé une entrée libre ou des pages vers 83 millions dans une base de 455 000 pages.
Le cycle ouverture puis point de reprise entretient l'écart et le déplace, mais ne le crée pas. Le
bloc des cores, 80 724, vise la page 82 661 836, dans une autre base (code-rag3db-depot, 448 430
pages, saine) : le numéro faux y était en mémoire.

**Relevé des bases, 17 h 20** : toutes les bases de plus de 100 Mio de ce poste sont saines, sauf
code-essais-c7. À luciepc, les 22 bases lisibles sur 34 sont saines : aucune n'est creuse, aucune
n'a de fichier plus long que son étendue. La sonde croisée de hotfixs (fsm_info × storage_info)
donne zéro page à la fois libre et tenue sur la copie de 12 h 06 et sur la base de son index tracé.

**Le lot `etendue-gardes`** : témoins `2411e3089`, correctif `8a1f0bd44` et `aab77eaf7`.

Sur master, le témoin ASegmentBeyondTheExtentIsNotGivenBack est rouge. Il rend libres 18 pages tenues
du dernier segment, relit des lignes fausses après une réallocation, puis meurt de SIGSEGV.

**Sur master à 9bae60786 (8 novembre, 18 h 20).** Le lot y est arrivé rebasé, sans changement du
code moteur depuis la version testée, 62af3ae42. Entre les deux, master n'a apporté que du Rust et
des scripts de rag3weaver, ainsi que le pointeur de codeparsers.

Ce qui a été joué à luciepc, sur 62af3ae42, en ionice idle :
- les quatre témoins de l'étendue, verts ;
- 108 cas voisins de transaction_test, verts : pages sans propriétaire, COPY journalisé, points
  de reprise et V2, coupures, annulations, vecteurs à la réouverture, formats, lecture seule ;
- les neuf binaires de test/storage, verts ;
- trois piles imprimées, toutes par les trois témoins de refus, aucune ailleurs.

La copie de 16 h 52 a aussi été jouée sous la lib du lot. L'ouverture ne rend rien de tenu. Après
un point de reprise, la sonde croisée trouve 0 page libre et tenue. Les 114 855 vecteurs se relisent
identiques à l'origine.

**Ce qui n'a pas été joué : `transaction_test` entier.** Il passait 10 cas en 30 minutes sous la
pression disque de luciepc. Il sera rejoué sur disque calme avant tout rebâti de lib qui prend ce
lot. D'ici là, aucune lib du magasin ne contient le lot.
- numPages ne baisse plus : la dernière plage libre reste une entrée.
- returnOwnerlessPages ne rend que ce qui suit la fin de ce qui est tenu. Le relevé passe par
  `reclaimStorage` de chaque table vers un allocateur qui ne fait que noter, puis ajoute le
  catalogue, les métadonnées et la liste libre.
- Le PageManager refuse une plage à libérer ou à servir qui dépasse le fichier.
- `pin` et `optimisticRead` lèvent une erreur pour une page au-delà de numPages.
- `writePagesToFile` refuse d'écrire au-delà de numPages.

Témoins dans `test/transaction/data_file_extent_test.cpp` :
- ASegmentBeyondTheExtentIsNotGivenBack ;
- ARangeBeyondTheFileIsNotFreed ;
- AWriteBeyondTheFileIsRefused ;
- APageBeyondTheFileIsAnErrorNotACrash.

Les sondes en lecture seule (`croise.cpp`, `entete.py`) sont dans
`~/.cache/rag3db-moteur-notes/essais-c7/`.

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
