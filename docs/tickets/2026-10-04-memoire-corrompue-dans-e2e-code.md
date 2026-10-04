# Une corruption de mémoire tue `e2e_code`, une passe sur trente à soixante

- **État** : ouvert
- **Gravité** : plantage
- **Atteignable en service** : probablement oui — l'écriture fautive est dans la lecture d'une colonne de chaînes longues par un balayage ordinaire ; vu seulement, à ce jour, dans la suite `e2e_code`
- **Touche rag3weaver** : oui (c'est sa suite `e2e_code` qui meurt)
- **Ouvert le** : 4 octobre 2026, session cœur C++
- **Pour** : cœur C++ (stockage, tampon)

## Ce que c'est

Le processus de la suite `e2e_code` meurt par signal, rarement, en fin de suite : quelque chose écrit hors de son tampon, et la victime change d'une fois à l'autre.

## Ce qui a été mesuré

Suite entière (`e2e_code`, 25 tests en parallèle dans un seul processus : une base sur disque, une trentaine en mémoire), poste chargé (12 à 20) :

| Bibliothèque | Passes | Morts par signal | Forme |
|---|---|---|---|
| master `1b9b522c3` | 60 | 1 | SIGABRT, « free(): invalid next size (normal) » |
| master + garde 2 + borne des voisins | 57 | 2 | SIGSEGV dans une copie mémoire ; un autre sans pile |
| le test `a_bulk_load_interrupted_by_a_caught_panic_is_repaired_on_reopen` seul | 150 | 0 | — |

Le défaut est donc antérieur à la garde 2 de la reprise. À quel commit il remonte n'est pas établi : la session de l'arbre principal ne l'avait jamais vu en une quinzaine de passes, ce qui ne tranche pas à cette fréquence.

La pile du SIGSEGV attrapé sous gdb (le fil est celui de la base sur disque, rouverte après un chargement en masse interrompu ; le journal du test s'arrête après « index vectoriel … laissé détruit par un chargement en masse interrompu — reconstruction », c'est-à-dire pendant `DROP_VECTOR_INDEX`, `CREATE_VECTOR_INDEX` ou le `MERGE` de méta qui les suit) :

```
#0  (libc, copie mémoire)
#1  DefaultColumnReadWriter::readCompressedValues — la fonction de lecture d'une page
#2  BufferManager::optimisticRead
#3  DefaultColumnReadWriter::readCompressedValuesToPage
#4  Column::scanSegment
#5  DictionaryColumn::scan
#6  StringColumn::scanUnfiltered
#7  Column::scan → ColumnChunk::scan → ChunkedNodeGroup::scan → NodeGroup::scan
#11 NodeTableScanState::scanNext
#12 ScanNodeTable → Filter → Projection → ResultCollector
```

Cette pile est celle de la victime, pas celle de l'écriture fautive.

## Ce qu'AddressSanitizer a vu (4 octobre, 14 h 25)

Bibliothèque bâtie avec `-DENABLE_ADDRESS_SANITIZER=ON` (dossier `build/asan`), `libasan` préchargée, suite entière : arrêt à la première passe, **heap-buffer-overflow, écriture de 4 096 octets juste après un bloc de 32 768 octets**. Une seconde passe, sous gdb, n'a rien signalé : ce n'est pas déterministe non plus sous ASan (une sur deux à ce jour).

L'écriture fautive — la lecture d'une chaîne longue depuis le disque, dans un balayage de table :

```
memcpy
Uncompressed::decompressFromPage
ReadCompressedValuesFromPage::operator()
DefaultColumnReadWriter::readCompressedValues   src/storage/table/column_reader_writer.cpp:191
BufferManager::optimisticRead                   src/storage/buffer_manager/buffer_manager.cpp:229
Column::scanSegment                             src/storage/table/column.cpp:289
DictionaryColumn::scanValue                     src/storage/table/dictionary_column.cpp:175
DictionaryColumn::scan<ValueVector>             src/storage/table/dictionary_column.cpp:115
StringColumn::scanUnfiltered                    src/storage/table/string_column.cpp:230
Column::scan → ColumnChunk::scan → ChunkedNodeGroup::scan → NodeGroup::scan
ScanNodeTable → Filter → Projection
```

Le bloc dépassé est le tampon des chaînes longues du vecteur de sortie, alloué dans le même balayage, par le même fil, quelques lignes plus bas — la copie d'une chaîne déjà lue vers une position qui porte la même valeur :

```
MemoryManager::mallocBuffer
InMemOverflowBuffer::allocateNewBlock           src/common/in_mem_overflow_buffer.cpp:62
InMemOverflowBuffer::allocateSpace              src/common/in_mem_overflow_buffer.cpp:30
StringVector::addString                         src/common/vector/value_vector.cpp:473
ValueVector::setValue<ku_string_t>
DictionaryColumn::scan<ValueVector>             src/storage/table/dictionary_column.cpp:123
```

Donc : `DictionaryColumn::scanValue` réserve `length` octets dans le vecteur puis fait lire `length` octets par pages de 4 096 ; l'une de ces pages est écrite au-delà de la fin du bloc. Ce n'est ni l'index vectoriel, ni la reprise, ni le tampon de pages : c'est la lecture d'une colonne de chaînes. C'est la même pile que le SIGSEGV attrapé sous gdb. Le rapport complet : `extension/rag3weaver/docs/3-octobre-2026-23h31/coeur-cpp/annexes/asan-e2e_code-4-octobre.txt`.

**Ce qui n'est pas encore su** : pourquoi la longueur écrite dépasse la place réservée. La lecture du code n'a rien montré d'évident (la réservation et la lecture prennent la même longueur ; les blocs sont dimensionnés correctement). Pistes : une longueur calculée à partir de décalages incohérents (`endOffset - startOffset` d'un dictionnaire dont les décalages ne sont pas croissants, ou lus dans un autre segment que les données — la segmentation des colonnes date de septembre 2025, `9f5acab2e`) ; des métadonnées de la colonne de données qui ne sont pas à un octet par valeur ; un vecteur de sortie partagé.

**Essayé, sans effet** : un test C++ qui écrit puis relit, après un point de reprise, des chaînes de 5 000 à 70 000 caractères avec et sans doublons consécutifs, joué avec `MALLOC_CHECK_=3` — rien (`annexes/long_string_scan_scratch_test.cpp`). Un contrôle de borne provisoire sur le tableau « déjà visités » de la recherche vectorielle (dimensionné d'après une statistique de table) : ne se déclenche pas dans cette suite — écarté ici, mais ce tableau reste sans contrôle de borne.

**La prochaine étape** : rejouer sous ASan avec gdb arrêté sur `__asan::ReportGenericError`, et lire dans `DictionaryColumn::scanValue` et son appelant `length`, `startOffset`, les décalages et les métadonnées du segment (`~/.cache/rag3db-moteur-notes/asan/gdb-asan.cmd`, `annexes/build-asan.sh`). Attention : l'extension vector sort dans `extension/vector/build/`, commun à tous les dossiers de build d'un arbre — rebâtir l'extension ordinaire après une passe ASan.

## Recette minimale

Aucune. Reproduction : la suite entière en boucle, binaire lancé directement —
`extension/rag3weaver/docs/3-octobre-2026-23h31/coeur-cpp/annexes/boucle-nue.sh` (compte les morts par signal) et `boucle-gdb.sh` (s'arrête au premier signal et garde la pile).

## Témoin

Aucun, assumé : un témoin à une chance sur quarante n'en est pas un. Ce qu'il faut pour l'écrire : bâtir la bibliothèque avec AddressSanitizer (`-DENABLE_ADDRESS_SANITIZER=ON`) et jouer la suite une fois — une écriture hors tampon y est arrêtée à la première occurrence, avec la pile de l'écriture.

## Cause

Inconnue. Pistes, non vérifiées : la lecture optimiste d'une page pendant qu'un autre fil la réutilise ; des métadonnées de bloc périmées après le `DROP` puis `CREATE` d'index (qui écrit un point de reprise) ; le chargement de la même extension par une trentaine de bases du même processus.

## Correctif de l'amont (à lire, ne pas copier)

Candidats de la revue des amonts sur le tampon, non reliés à ce défaut : Ladybug `7e4248202`, `eff87c1e1`, `d6adbd2b7` ; Vela `e5e700e73`.

## À relire à sa lumière, une fois la pile obtenue

- L'intermittent d'`e2e_idempotent_registration` : « Reading past the end of the file …wal with size 0 » à la réouverture, 2 fois sur 20, avant comme après les correctifs du rejeu.
- Le ticket « La réouverture d'une base échoue par intermittence, sous charge seulement ».
- Tout rouge intermittent inexpliqué de ces derniers jours.

## Ce qu'il faut pour le fermer

La pile d'AddressSanitizer, puis un test C++ qui reproduit l'écriture fautive à coup sûr.
