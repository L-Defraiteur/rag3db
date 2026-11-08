# Une validation et un lecteur HNSW se figent l'un l'autre sur les deux verrous d'un groupe CSR

- **État** : corrigé `e526cc4b6` (7 novembre 2026, lot ordre-des-verrous du cœur c++, sur master en
  `5f0b79466`) : la compaction lit une liste des blocs prise avant l'index et ne prend plus le
  verrou des blocs sous lui ; l'ordre des deux verrous et l'argument de sûreté sont écrits au-dessus
  de `compactDeadRowsNoLock`. Le témoin rejoué 30 fois sans un figement (16,5 s au plus), la suite
  transaction en 6 min 46 s.
- **Gravité** : blocage (le processus entier se fige : la validation tient le gestionnaire de
  transactions, plus rien ne commence).
- **Atteignable en service** : oui — une recherche vectorielle pendant qu'un écrivain valide des
  relations qui ont des lignes mortes à compacter.
- **Touche rag3weaver** : par le moteur seulement (`src/storage/table/csr_node_group.cpp`).

## Ce que c'est

Un interblocage entre deux verrous d'un `CSRNodeGroup` pris dans des ordres contraires :

- **la compaction des lignes mortes** (c44af8031, 12 octobre) tient `csrIndexMtx` en exclusif,
  puis prend le verrou des blocs (`chunkedGroups.lock()` dans `compactDeadRowsNoLock`), dans ses
  deux chemins : `CSRNodeGroup::append` et `compactHintedLists` (à la validation) ;
- **tout le reste** prend le verrou des blocs puis l'index : le point de reprise,
  `finalizeCheckpoint`, et depuis V2-2 (ae4432960) `takeGenerationFor`, appelé par
  `initializeScanState` à chaque balayage d'un groupe.

## Le témoin

`test/transaction/vector_search_during_insert_test.cpp`,
`VectorSearchDuringInsertTest.SearchesWhileOneWriterInsertsVectors`, figé environ une fois sur
cinq (rejoué seul : quatre passages puis le figement). Trois suites transaction de la journée ont
dépassé leur délai sur ce test, lues d'abord comme de la lenteur de luciepc.

Pile prise (eu-stack, `LD_PRELOAD=ptracer-any.so`) :

| fil | où | tient | attend |
|---|---|---|---|
| le principal (validation) | `TransactionManager::commit` → `UndoBuffer::compactDeadRowsAfterCommit` → `compactHintedLists` → `compactDeadRowsNoLock` | le mutex du gestionnaire, `csrIndexMtx` exclusif | le verrou des blocs |
| un fil de tâche (recherche HNSW) | `OnDiskHNSWIndex::searchKNNInLayer` → `OnDiskGraph::scanFwd` → `initializeScanState` → `takeGenerationFor` | le verrou des blocs | `csrIndexMtx` partagé |
| le fil des écrivains du test | `TransactionManager::beginTransaction` | — | le mutex du gestionnaire |

Forme déterministe, non écrite : deux fils et une barrière entre la prise du premier verrou et
la demande du second (un point d'injection dans `compactDeadRowsNoLock` et dans
`takeGenerationFor`), ce que le moteur n'a pas aujourd'hui.

## Le correctif

Les deux chemins de la compaction (`append`, `compactHintedLists`) prennent la liste des blocs sous
le verrou des blocs, le rendent, puis prennent `csrIndexMtx` ; `compactDeadRowsNoLock` lit cette
liste. Une ligne d'un bloc ouvert après la prise de la liste reste vivante et garde sa note.
