# Un point de reprise automatique meurt dans le dictionnaire d'une colonne de chaînes

- **État** : ouvert (14 novembre 2026, cœur c++)
- **Famille** : moteur
- **Gravité** : plantage (SIGSEGV du processus entier)
- **Atteignable en service** : à établir. Il faut un point de reprise automatique, demandé par un
  COMMIT, sous vingt écrivains et des DDL.
- **Touche rag3weaver** : par le moteur (l'hôte meurt)
- **Signalé par** : cœur c++ — `TwentyWriters.DdlEveryMinute` ×10 sous profil, sur le correctif de
  l'annulation sous le verrou
- **Pour** : cœur c++

## Ce qu'on voit

`TwentyWriters.DdlEveryMinute`, tour 6 sur 10, joué à luciepc le 14 novembre sur `9b0e73c9c`
(branche `annulation-garde-les-autres-2`, master `6b60d77aa` plus le correctif de l'annulation
sous le verrou). Bâti Release dans `~/git_workspaces/rag3db-colonnes/build/moteur`, sous
`RAG3DB_PROFILE_CHECKPOINT=1`.

Le processus meurt d'un SIGSEGV après 1 min 40, **sans aucun refus du pool** et sans aucune
annulation d'un COMMIT refusé : le chemin neuf du correctif ne s'est pas exécuté. Le core est
gardé par systemd-coredump à luciepc (pid 3980144, 14:17:21).

```
#0 std::_Hash_bytes
#1 std::_Hashtable<DictionaryChunk::DictionaryEntry, …>::find
#2 DictionaryChunk::appendString
#3 StringChunkData::write
#4 Column::checkpointSegment
#5 ColumnChunk::checkpoint
#6 NodeGroup::checkpointInMemAndOnDisk
#7 NodeGroup::checkpoint
#8 NodeGroupCollection::checkpoint
#9 NodeTable::checkpoint
#10 StorageManager::checkpoint
#11 Checkpointer::writeCheckpoint
#12 TransactionManager::checkpointNoLock
#13 TransactionManager::writeRequestedCheckpointIfLastNoLock
#14 TransactionManager::commit
```

Le point de reprise automatique, demandé par un COMMIT (A8), écrit une colonne de chaînes. En
recopiant une chaîne dans le dictionnaire du segment, il en hache une qui n'est plus lisible.

Les autres fils, lus avec le binaire de `9b0e73c9c` encore en place : trace complète à luciepc,
`~/.cache/rag3db-moteur-notes/core-3980144.txt` (`thread apply all bt 8`).

- Un seul fil écrit le point de reprise : le fil 1, celui qui meurt.
- 20 fils attendent à `BEGIN` (`waitWhileCheckpointPendingNoLock`), ce qui est normal : un
  point de reprise est en cours.
- Des lecteurs admis sous V2 balaient pendant ce temps : `ScanRelTable`, `PrimaryKeyScanNodeTable`,
  et le fil 3 dans `Column::scanSegment` → `BufferManager::optimisticRead` → `decompressFromPage`.
  C'est une lecture optimiste ordinaire, qui n'est pas en cause d'après sa pile.
- La chaîne illisible est lue par le point de reprise dans un bloc **en mémoire** de la colonne
  (`StringChunkData::write` à partir des données du bloc). Ce tour n'a eu aucune annulation d'un
  COMMIT refusé. Les échecs ordinaires du scénario (clé dupliquée, accès concurrent) s'annulent
  dans le stockage local de leur transaction.

## Ce qu'on ne sait pas

- **Quelle table.** Note a `texte STRING`, Doc a `titre STRING`. Le dictionnaire peut lire une
  chaîne d'une page rendue, ou d'un bloc réécrit sous lui.
- **Si c'est le SIGSEGV de la nuit sur `f41c0ba5b`** (tour 6, 02:34:46, pid 3562319). Sa pile ne
  se lit plus : le binaire a été rebâti depuis, et ses symboles ne correspondent plus.
- **Si c'est la même famille** que `2026-11-08-un-sigsegv-dans-optimisticread-sous-lecture-
  concurrente-pendant-une-indexation.md`, ou que le lecteur du 13 novembre mort dans l'index de clé
  primaire.

## Ce qu'il faut pour le fermer

1. Lire la pile entière du core 3980144 (fil, colonne, table) avec le binaire de `9b0e73c9c`,
   rebâti à l'identique si celui de l'arbre a changé.
2. Un témoin qui meure de même, puis le correctif.
3. `DdlEveryMinute` ×10 sans SIGSEGV.
