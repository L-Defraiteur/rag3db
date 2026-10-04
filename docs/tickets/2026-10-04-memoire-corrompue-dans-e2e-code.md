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

**Six passes de plus sous ASan (4 octobre, 14 h 50 à 15 h 15), bibliothèque instrumentée** — une ligne imprimée dès que les décalages lus du dictionnaire sont décroissants ou dépassent la taille des données : six passes vertes, aucun rapport, aucune incohérence imprimée. À ce jour ASan a donc attrapé le défaut **une passe sur huit**. Une différence entre la passe qui l'a attrapé et les sept autres : elle chargeait l'extension vector bâtie elle aussi avec ASan, les autres l'extension ordinaire ; non exploré. La charge du poste a varié de 6 à 40 entre ces passes. L'hypothèse d'une course (une longueur tirée d'une lecture optimiste d'une page en cours d'éviction) n'est ni confirmée ni écartée : les nombres n'ont pas pu être lus.

**La prochaine étape** : rejouer sous ASan avec gdb arrêté sur `__asan::ReportGenericError`, et lire dans `DictionaryColumn::scanValue` et son appelant `length`, `startOffset`, les décalages et les métadonnées du segment (`~/.cache/rag3db-moteur-notes/asan/gdb-asan.cmd`, `annexes/build-asan.sh`). Attention : l'extension vector sort dans `extension/vector/build/`, commun à tous les dossiers de build d'un arbre — rebâtir l'extension ordinaire après une passe ASan.

**Reprise du 4 octobre, 16 h — l'extension vector bâtie elle aussi sous ASan.** Première passe sans rapport (elle portait un détecteur provisoire d'usage concurrent du tampon des chaînes, qui n'a donné que des faux positifs : le fil « autre » était au repos dans la pile de tous les fils). Deuxième passe : **un autre débordement, dans l'extension**, même test (`a_bulk_load_interrupted_by_a_caught_panic_is_repaired_on_reopen`, la base sur disque) :

```
READ of size 1, 44 octets après un bloc de 211 octets
VisitedState::contains            extension/vector/src/include/index/hnsw_index.h:58
OnDiskHNSWIndex::oneHopSearch     extension/vector/src/index/hnsw_index.cpp:1298
OnDiskHNSWIndex::searchKNNInLayer extension/vector/src/index/hnsw_index.cpp:1231
searchFromCheckpointed → search → QUERY_VECTOR_INDEX (query_hnsw_index.cpp:256)
bloc alloué par VisitedState::VisitedState (hnsw_index.h:50), HNSWSearchState, query_hnsw_index.cpp:320
```

Le tableau des « déjà visités » est dimensionné par `nodeTable->getStats(transaction).getTableCard()` (`query_hnsw_index.cpp:293`, de même `vector_search_function.cpp:165`) — la cardinalité **estimée** de la table (« not always up-to-date », `table_stats.h:63`) — et indexé par le décalage de ligne d'un voisin : ici 211 lignes comptées, un voisin au décalage 255. `contains` lit hors du tableau ; `add` (`visited[offset] = 1`, même fichier) y **écrit un octet** : une corruption de tas, invisible à ASan tant que l'extension n'est pas bâtie avec lui (c'est pourquoi sept passes n'avaient rien vu). Le contrôle de borne provisoire essayé plus haut ne s'était pas déclenché : il était juste, la condition est rare.

**Quatre passes de plus (16 h 16 à 16 h 29), avec un contrôle « cardinalité < nombre de lignes de la table »** posé au commit d'une table de nœuds, à la fin d'un COPY, à la relecture de la table à l'ouverture et à la création de l'état de recherche : quatre passes vertes, aucun rapport ASan, **et le contrôle n'a jamais parlé**. Sur ces quatre passes la cardinalité n'a donc jamais été en retard sur la table. Deux lectures restent, non départagées : (a) la cardinalité n'est en retard que dans la passe fautive ; (b) elle est juste (211 lignes) et c'est **le voisin rendu par le graphe de l'index qui est faux** (décalage 255 dans une table de 211 lignes). La lecture (b) rapprocherait les deux rapports : dans les deux, une colonne lue sur disque rend une valeur incohérente (un décalage de dictionnaire, un voisin), dans la même base sur disque, juste après `DROP_VECTOR_INDEX` puis `CREATE_VECTOR_INDEX` (qui libère puis réalloue des pages et écrit un point de reprise). Non vérifié.

Le compte de la reprise : huit passes lancées, la limite fixée. Une avec faux positifs de mon détecteur (verte pour ASan), une avec le rapport ci-dessus, deux mortes en moins d'une minute par une faute de mon propre contrôle (elles ne comptent pas comme observations), quatre vertes. Avec l'extension sous ASan : un rapport sur six passes réelles ; le rapport de 14 h 25 (`scanValue`) n'est pas revenu. **La condition n'est pas établie.** L'hypothèse d'une course sur la lecture optimiste n'est ni confirmée ni écartée : les nombres de `scanValue` n'ont pas pu être lus (le défaut n'y est pas revenu) ; aucun usage du tampon du vecteur par deux fils n'a été vu.

Pour reprendre : `annexes/instrumentation-traque-4-octobre.patch` (à appliquer sur `110a65f15` ; imprime au rapport ASan les longueurs, décalages et métadonnées de `scanValue`, et les piles de tous les fils par gdb), `annexes/build-traque.sh`, `annexes/boucle-traque.sh`, le rapport complet `annexes/asan-extension-vector-4-octobre.txt`. À ajouter avant de relancer : dans `oneHopSearch`, imprimer le décalage du voisin, la cardinalité et le nombre de lignes quand le voisin dépasse le tableau — cela départage (a) et (b) en un rapport. Et un essai déterministe à écrire, non tenté : base sur disque, table à vecteurs et chaînes longues, index, `DROP_VECTOR_INDEX`, fermeture, réouverture, `CREATE_VECTOR_INDEX`, recherche et balayage.

**Les gardes sont posées** (`1ea49837f`, 4 octobre, sur décision de l'orchestration : une garde qui change une corruption de tas en erreur nommée se justifie seule et sert de sonde). Trois refus nommés, actifs en Release, chacun avec ses nombres dans le message :

| Refus | Où | Ce qu'il arrête |
|---|---|---|
| « is beyond the visited set » (`HNSWIndexUtils::OFFSET_BEYOND_VISITED_SET`) | `VisitedState::add` et `contains` | un décalage de ligne hors du tableau des visités |
| « is beyond the in-memory graph » (`HNSWIndexUtils::OFFSET_BEYOND_IN_MEM_GRAPH`) | `InMemHNSWIndex::insert` | un décalage hors du graphe en mémoire à la construction |
| « Dictionary offsets out of order » (`DictionaryColumn::DICTIONARY_OFFSETS_OUT_OF_ORDER`) | `DictionaryColumn::scan` | une fin de chaîne avant son début, ou au-delà des données |

Et un correctif de fond dans le même commit : l'index vectoriel se dimensionne par `NodeTable::getNumTotalRows` (le nombre de lignes) et plus par la cardinalité estimée, à la recherche comme à la construction — où cette cardinalité bornait aussi le parcours des lignes : en retard, des lignes restaient hors de l'index.

**Ces gardes n'ont pas de test neuf**, et c'est assumé : aucune des trois conditions ne se fabrique par l'interface. Un décalage hors du tableau des visités ne peut plus venir que d'un graphe d'index faux, depuis que le tableau a la taille exacte de la table ; des décalages de dictionnaire décroissants demandent un fichier abîmé ou la cause inconnue de ce ticket. Les fabriquer, c'est trouver la condition — l'essai déterministe est confié à la session du banc. La prochaine occurrence, en service ou en test, dira laquelle a parlé et avec quels nombres : **quiconque voit l'un de ces trois textes l'ajoute ici.**

Piste de témoin sans ASan (orchestration) : si la cardinalité est parfois en retard, un index bâti avant `1ea49837f` avait un rappel incomplet — des lignes absentes de l'index, mesurable par un compte.

## Un scénario déterministe voisin, et une hypothèse (session du banc, 4 octobre au soir)

**Trouvé** : sans rag3weaver, en Release et sans ASan, le processus meurt par SIGSEGV, trois
passes sur trois, sur un moteur d'avant `1ea49837f` (`b1b4df161`). La recette :
- base sur disque, table à vecteurs et chaînes longues, index, `CHECKPOINT` ;
- un `COPY` refusé (clé en double) ;
- `DROP_VECTOR_INDEX`, réouverture, `CREATE_VECTOR_INDEX`.

Le mécanisme, lu par la session cœur C++ :
- le `COPY` annulé laisse la cardinalité gonflée (401 pour 200 lignes : ticket
  `2026-10-04-copy-refuse-gonfle-la-cardinalite.md`) ;
- `CREATE_VECTOR_INDEX` bornait son parcours par cette cardinalité, alors que son graphe en
  mémoire est dimensionné par les vraies lignes : il écrivait hors de ses tableaux.

Depuis `1ea49837f`, le même scénario est vert : la création aboutit et les 200 lignes sont
joignables. Témoin : `VectorIndexUpdate.CreateIndexAfterARefusedCopy`.

Cinq autres variations n'ont rien donné, ni avant ni après ce commit :
- le scénario de base (index, `DROP`, réouverture, `CREATE`) ;
- des suppressions et réinsertions avant le `DROP` (décalages au-delà des lignes vivantes) ;
- un point de reprise entre `DROP` et `CREATE` ;
- des lignes non validées pendant la recherche ;
- des insertions annulées par `ROLLBACK`.

La cardinalité n'y a jamais été en retard sur les lignes, et les chaînes longues ont
toujours été relues justes.

**Hypothèse, non prouvée** : les deux rapports d'ASan seraient ce même défaut. Le test
fautif, `a_bulk_load_interrupted_by_a_caught_panic_is_repaired_on_reopen`, interrompt un
chargement en masse, puis fait `DROP` et `CREATE` de l'index à la réouverture. Si
l'interruption gonfle la cardinalité, la création écrit hors des tableaux de son graphe :
- c'est une corruption du tas, dont `DictionaryColumn::scanValue` ne serait qu'une victime
  plus loin ;
- le voisin au décalage 255 dans une table comptée à 211 serait un nœud inséré au-delà des
  lignes.

La fréquence s'expliquerait par l'instant de l'interruption : avant ou après la fusion des
statistiques.

**Ce qui la prouverait** : `e2e_code` rejoué en nombre sur le moteur d'avant `1ea49837f`,
puis sur celui d'après. Le défaut doit disparaître après.
- **En Release**, à une passe sur soixante : pour voir au moins une mort avant avec 95 % de
  chances, puis lire un zéro après comme un taux inférieur à 1/60, il faut environ 180
  passes de chaque côté (règle de trois). C'est 6 à 12 heures par côté, à 2 à 4 minutes la
  passe.
- **Sous ASan, l'extension vector comprise**, le défaut a été attrapé une passe sur six :
  environ 18 passes de chaque côté suffisent pour la même confiance. C'est 1 h 15 à 1 h 30
  par côté, à 4 minutes la passe.
- Plus court encore : vérifier d'abord que l'interruption du test gonfle la cardinalité, en
  lisant `STATS_INFO` contre `count(*)` à la réouverture. Une seule passe.

**Un candidat sérieux pour la cause (4 octobre, 18 h)** : en faisant un test du moteur de l'essai déterministe du banc, un défaut d'origine est sorti — après un ajout annulé (un `COPY` refusé) dans un groupe de nœuds encore en mémoire, le point de reprise suivant écrit hors de son bloc (ticket « Le point de reprise écrit hors de son bloc après un ajout annulé », corrigé par `05788a868`). Le test fautif d'`e2e_code` interrompt un chargement puis rouvre et recrée l'index, ce qui écrit un point de reprise. **Non vérifié** : rien ne dit encore que c'est cette écriture-là qui tuait la suite. Ce ticket se ferme si les passes du banc sous AddressSanitizer, sur un moteur à jour, ne rendent plus de rapport — ou reste ouvert avec le texte du refus nommé qui aura parlé.

## La passe courte : l'hypothèse du COPY refusé tombe pour ce test (4 octobre, fin d'après-midi)

La recette du test fautif, rejouée par une sonde (non commitée) sur master avec
`1ea49837f`. Elle relève, à la réouverture, avant la réparation (`DROP` puis `CREATE` de
l'index), la cardinalité (`STATS_INFO`), `count(*)` et le plus grand décalage de chaque
table de nœuds :

| table | cardinalité | lignes | plus grand décalage |
|---|---|---|---|
| `Scope_Chunk` (la table de l'index vectoriel) | 211 | 211 | 210 |
| `Scope` | 144 | 144 | 143 |
| `Symbol` | 382 | 382 | 381 |
| `_index_blobs` | 670 | 670 | 669 |
| `_DataflowNodeState` | 27 | 19 | 26 |
| les autres | justes | | |

- **Aucune table indexée n'a de cardinalité gonflée** : rien n'indique de `COPY` refusé
  dans ce test. L'hypothèse du `COPY` refusé ne vaut donc pas pour lui.
- **`_DataflowNodeState` (27 pour 19)** : l'écart vient de suppressions, que la
  cardinalité ne décompte jamais. Il est sans danger ici : le plus grand décalage (26) reste
  sous 27.
- **Les 211 du rapport d'ASan sont exactement les lignes de `Scope_Chunk`.** Le tableau
  des déjà-visités était bien dimensionné ; le voisin au décalage 255 n'existe pas dans
  cette table (plus grand décalage 210). C'est la lecture (b) : **le graphe de l'index
  rend un voisin faux**. La piste qui reste : d'où vient un décalage 255 dans le graphe
  d'un index rebâti sur 211 lignes ? Ce peut être un décalage d'une autre table, ou celui
  d'une ligne déjà supprimée, resté dans une arête.
- Les défauts trouvés en chemin restent réels et ont leurs témoins : la cardinalité
  gonflée par un `COPY` refusé, la clé d'origine perdue, le point de reprise qui suit un
  `COPY` refusé. Mais ils ne sont pas, à ce jour, la cause des rapports d'`e2e_code`.
- Les 18 passes sous ASan ne sont pas lancées, sur consigne de l'orchestration.

**La garde a parlé (4 octobre, 18 h 50)** — moteur `5c8507577`, `e2e_code` sous AddressSanitizer (bibliothèque et extension), deuxième passe sur deux. Plus de rapport ASan, plus de corruption : le test `a_bulk_load_interrupted_by_a_caught_panic_is_repaired_on_reopen` échoue sur le refus nommé,

```
Dictionary offsets out of order: string 22 runs from byte 2185 to byte 1095
in a dictionary of 6197 bytes and 17 strings.
```

à `tests/e2e_code.rs:758` (`catalog.initialize()` de la réouverture), juste après « index vectoriel 'Scope_Chunk_vec__hashembedder' laissé détruit par un chargement en masse interrompu — reconstruction ». Ce que les nombres disent : **l'indice de la chaîne (22) dépasse le nombre de chaînes du dictionnaire (17)**. Les décalages lus au-delà du dictionnaire sont donc n'importe quoi (2185 puis 1095), et c'est ce qui faisait reboucler la longueur dans le premier rapport. Ce ne sont pas les décalages qui sont faux : c'est que l'indice et le dictionnaire ne vont pas ensemble — une colonne d'indices lue dans un état, son dictionnaire dans un autre. Le candidat « point de reprise après un ajout annulé » est donc écarté pour ce rapport-ci (il est corrigé, et le défaut revient).

Deux lectures, non départagées : (a) les métadonnées du dictionnaire sont en retard sur la colonne d'indices après une écriture en place au point de reprise (`StringColumn::writeSegment`, `DictionaryColumn::append`) ; (b) une page relue est périmée — le cadre du tampon d'une page libérée puis réallouée (le `DROP` puis `CREATE` de l'index libère et réalloue des pages, et écrit un point de reprise). La lecture (b) expliquerait aussi le voisin 255 rendu par le graphe de l'index pour une table de 211 lignes.

**La seconde garde a parlé aussi, et la base est abîmée sur disque (4 octobre, 19 h 12 à 19 h 40).** Passe suivante, même test : « HNSW index: node offset 255 is beyond the visited set of 211 nodes » — les nombres du rapport ASan de l'après-midi, avec un tableau cette fois à la taille exacte de la table : le voisin 255 est bien faux.

La base du test fautif reste sur disque (`/var/tmp/rag3weaver-bulk-<pid>`, copiée dans `~/.cache/rag3db-moteur-notes/bases-fautives/`). Rouverte dans un processus neuf, elle est **abîmée de façon durable** : `MATCH (n:_index_blobs) RETURN n` rend « Dictionary offsets out of order: string 0 runs from byte 2097151 to byte 2062335 in a dictionary of 1880926 bytes and 312 strings ». `storage_info('_index_blobs')` place `_data_index` à la page 5102 et `_data_offset` à la page 5103 (bitpacking sur 21 bits). Le contenu brut de la page 5103 :

```
ffff ffff ee13 0000 ffff ffff ffff ffff …   (4 092 octets à ff)
```

C'est une table de pages de `DiskArray` — « page suivante » invalide, première page de données `0x13ee` = 5102, le reste invalide —, c'est-à-dire une page de l'index de clé primaire (ou de son fichier de débordement). **Deux structures possèdent les mêmes pages** : une colonne de chaînes et un tableau sur disque de l'index. 2 097 151 et 255 sont des « tout à un » lus sur 21 et sur 8 bits. Aucun recouvrement entre colonnes (445 plages relevées par `storage_info` sur toutes les tables) : le second propriétaire n'est pas une colonne.

L'autre base fautive (celle du refus du dictionnaire, 18 h 50) se relit sans erreur dans un processus neuf : là, la page fausse n'était pas (ou plus) sur disque.

Ce qui reste à établir : par quel chemin une page est rendue puis réattribuée alors que quelqu'un s'en sert encore. Deux pistes, non vérifiées : une page d'ombre en attente appliquée par-dessus une page réattribuée entre-temps ; le stockage d'une colonne vivante rendu au point de reprise (`NodeGroup::checkpointInMemAndOnDisk` rend ce qui « doit avoir été supprimé »). rag3weaver émet `ALTER TABLE _index_blobs ADD _deleted_gen` à chaque ouverture de son magasin de blobs. En cours : un journal provisoire des allocations, libérations, écritures directes et pages d'ombre de la base du test, pour lire l'histoire de la page à la prochaine occurrence. Outils : `annexes/sonde-base-fautive/` (rouvrir une base, relire toutes ses tables, relever les plages de pages).

## Recette minimale

Aucune. Reproduction : la suite entière en boucle, binaire lancé directement —
`extension/rag3weaver/docs/3-octobre-2026-23h31/coeur-cpp/annexes/boucle-nue.sh` (compte les morts par signal) et `boucle-gdb.sh` (s'arrête au premier signal et garde la pile).

## Témoin

Aucun, assumé : un témoin à une chance sur quarante n'en est pas un. Ce qu'il faut pour l'écrire : bâtir la bibliothèque avec AddressSanitizer (`-DENABLE_ADDRESS_SANITIZER=ON`) et jouer la suite une fois — une écriture hors tampon y est arrêtée à la première occurrence, avec la pile de l'écriture.

## Cause

Inconnue. Pistes, non vérifiées : la lecture optimiste d'une page pendant qu'un autre fil la réutilise ; des métadonnées de bloc périmées après le `DROP` puis `CREATE` d'index (qui écrit un point de reprise) ; le chargement de la même extension par une trentaine de bases du même processus.

## Correctif de l'amont (à lire, ne pas copier)

Candidats de la revue des amonts sur le tampon, non reliés à ce défaut : Ladybug `7e4248202`, `eff87c1e1`, `d6adbd2b7` ; Vela `e5e700e73`.

La revue des amonts de la session du banc n'a **aucun correctif sur le site exact** (la variante `ValueVector` de `DictionaryColumn::scanValue`, appelée par `StringColumn::scanUnfiltered`). Deux voisins, non reproduits : Ladybug `254a7444d` (la variante `StringChunkData` de `scanValue`, lecture partielle d'un segment vers un bloc) et Ladybug `a10cecbc7` (`string_column.cpp`, la borne d'une lecture filtrée sur plusieurs segments — une lecture périmée, pas une écriture).

Trois hypothèses de lecture de la session du banc, non vérifiées : puisque la réservation et la copie prennent la même longueur, la destination n'est plus dans le bloc réservé au moment de la copie — (1) le tampon auxiliaire du vecteur est remis à zéro ou libéré entre la réservation et la copie ; (2) deux fils écrivent dans le même vecteur ; (3) dans la boucle de `DictionaryColumn::scan`, la référence `scannedString` est copiée par `setValue` après une réallocation.

## À relire à sa lumière, une fois la pile obtenue

- L'intermittent d'`e2e_idempotent_registration` : « Reading past the end of the file …wal with size 0 » à la réouverture, 2 fois sur 20, avant comme après les correctifs du rejeu.
- Le ticket « La réouverture d'une base échoue par intermittence, sous charge seulement ».
- Tout rouge intermittent inexpliqué de ces derniers jours.

## Ce qu'il faut pour le fermer

La pile d'AddressSanitizer, puis un test C++ qui reproduit l'écriture fautive à coup sûr.
