# Core lu : pile du checkpoint dictionnaire conservée

**Analyse du core acceptée, aucune qualification C++ verte ni cause établie.** GO exact C7-CORE-RO-37F-16nov120, pilote37f2b52aa834f3359292bd0e58f2c82be4689b32/recette314d3f603501fa62a40a89c05456f088a58f40668031f7334f299d13dee1cc4e. Un seul lancement admis de02 ; le refus CLI01 reste conservé, aucun journal réécrit.

## Exécution et fin

Vrai poste4G/swap0, quota500000/100000, POSTE_FILS5/admitted_threads5. Parent run-p224659-i151168702.scope, départ monotonic1195185,005814978. Limites effectives enfant mémoire4294967296/swap0/cpu500000/100000 ; namespace sans route, scratch32G privé. Entrées/outils épinglés, auto-load/auto-solib/debuginfod désactivés avant lecture du core, sysroot vide, aucun inférieur ou service lancé.

Décompression sparse code0 en1,366267s, taille17548656640 et SHA15a21f6deeacb0c8c1400acd4196f230189971b7bd325703c0091d6f14d34e2f conformes avant GDB. GDB code0 en0,214423s, stdout/stderr séparés préservés. Fin entrée5,511915s, avant120s : unité inactive/dead/Result=success/MainPID0/ControlGroup vide, quiescent=true, aucun PID attribué restant, aucun premier refus. Cinq entrées/outils rehashés après quiescence. Poste rendu, GO clos, aucun retry ou GDB supplémentaire.

## Ce que montre la pile, et ses limites

L'ELF archivé888 symbolise le thread crashé4141127 :

```
#0  0x00007fa3882ba880 in ??
#1  Hashtable<DictionaryEntry>::_M_find_tr<string_view>
#2  DictionaryChunk::appendString
#3  StringChunkData::write
#4  Column::checkpointSegment
#5  ColumnChunk::checkpoint
#6  NodeGroup::checkpointInMemAndOnDisk
#7  NodeGroup::checkpoint
#8  NodeGroupCollection::checkpoint
#9  NodeTable::checkpoint
#10 StorageManager::checkpoint
#11 Checkpointer::writeCheckpoint
```

Le journal coredump déjà conservé nommait std::_Hash_bytes à la même adresse #0 ; GDB conserve ?? car les bibliothèques partagées du processus ne sont pas chargées. **Pas de nouveau frame StringChunkData::finalize dans les12frames collectés** : le présent trajet arrive par write. L'autre thread4141129 est dans BufferManager::optimisticRead → Column::scanSegment → DictionaryColumn::scan → StringColumn::scanUnfiltered → NodeGroup::scan. Les deux autres piles ont largement des symboles inconnus ; ne pas les interpréter comme des piles fiables de corruption.

Le chemin dictionnaire/checkpoint rejoint la famille du parent C1 ; la même cause, une chaîne précise fautive, une libération, un index invalide ou une responsabilité du correctif append **ne sont pas démontrés**. Le core vient du cas isolé sur sourceed0/moteur888, pas d'un nouveau TwentyWriters. Son drapeau de diagnostic chaînes était effacé par le pilote50e2 : aucune valeur détaillée de cette sonde n'est acquise.

L'ELF possède symtab mais pas DWARF. Variables/arguments locaux non acquis, pas de ligne source promise, bibliothèques partagées non résolues volontairement, backtraces limitées12frames. RIP/RSP et info files/sharedlibrary conservés. Aucun dump supplémentaire, désassemblage, appel de fonction ou recompilation debug implicite.

## Preuves et tickets

[Lecture/empreinte](preuves/core-ro-02/lecture.json), archive recus-et-backtraces.tar.gz : recette, limites, commandes, env, hash décompressé, stdout/stderr complets et reçus code/timeout/wait/quiescence/rehash/inventaire. Paquet complet aux deux postes ;20 fichiers bruts et5 copies d'entrées rehashés localement conformes.

Le ticket de forme CLIzstd est fermé uniquement parce que la commande corrigée est effectivement qualifiée sous cette admission. Ticket du SIGSEGV, C1, ajout refusé, wrongRows228, plan lecteur et drapeau de sonde restent ouverts. Aucun correctif de production, lib posée ou master poussé. Suite : lecture source du trajet write/checkpoint et des durées de vie ; toute analyse native supplémentaire ou reprise du cas demandera une recette et un GO distincts.
