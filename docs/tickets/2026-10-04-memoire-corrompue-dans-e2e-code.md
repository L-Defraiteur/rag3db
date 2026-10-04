# Une corruption de mémoire tue `e2e_code`, une passe sur trente à soixante

- **État** : ouvert
- **Gravité** : plantage
- **Atteignable en service** : inconnu — vu seulement dans un processus qui tient une trentaine de bases à la fois
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

## Recette minimale

Aucune. Reproduction : la suite entière en boucle, binaire lancé directement —
`extension/rag3weaver/docs/3-octobre-2026-23h31/coeur-cpp/annexes/boucle-nue.sh` (compte les morts par signal) et `boucle-gdb.sh` (s'arrête au premier signal et garde la pile).

## Témoin

Aucun, assumé : un témoin à une chance sur quarante n'en est pas un. Ce qu'il faut pour l'écrire : bâtir la bibliothèque avec AddressSanitizer (`-DENABLE_ADDRESS_SANITIZER=ON`) et jouer la suite une fois — une écriture hors tampon y est arrêtée à la première occurrence, avec la pile de l'écriture.

## Cause

Inconnue. Pistes, non vérifiées : la lecture optimiste d'une page pendant qu'un autre fil la réutilise ; des métadonnées de bloc périmées après le `DROP` puis `CREATE` d'index (qui écrit un point de reprise) ; le chargement de la même extension par une trentaine de bases du même processus.

## Correctif de l'amont (à lire, ne pas copier)

Candidats de la revue des amonts sur le tampon, non reliés à ce défaut : Ladybug `7e4248202`, `eff87c1e1`, `d6adbd2b7` ; Vela `e5e700e73`.

## Ce qu'il faut pour le fermer

La pile d'AddressSanitizer, puis un test C++ qui reproduit l'écriture fautive à coup sûr.
