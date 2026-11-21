# Le point de reprise vide les pages libres non reprises sans verrou, pendant qu'un lecteur y en rend

- **État** : ouvert (21 novembre 2026, hotfixs) ; correctif et témoin en cours, même jour
- **Famille** : moteur
- **Gravité** : corruption mémoire possible (le tas de malloc), ou des pages perdues (une fuite). Une course de
  données, que ni ASan ni le Release ne voient.
- **Atteignable en service** : oui. Il faut un point de reprise, automatique ou demandé, pendant qu'un fil de requête
  lâche le dernier `ChunkedNodeGroup` qu'un point de reprise précédent a retiré : un lecteur sous le point de reprise
  V2, ou une transaction qui finit.
- **Touche rag3weaver** : par le moteur (l'hôte)
- **Signalé par** : hotfixs, `TwentyWriters.DdlEveryMinute` sous TSan (tour 5 sur 10, master 4fcbe09fc3), au mot de
  c7, pour C1
- **Pour** : hotfixs
- **Lien** : peut-être la cause de C1 (`docs/coeur-cpp/tickets/2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire-d-une-colonne-de-chaines.md`) :
  des champs d'un morceau du point de reprise écrasés, qu'aucun chemin du code ne produit. Pas prouvé : le cœur
  rejouera C1 sur la branche corrigée.

## Le constat (TSan)

- **T1**, le point de reprise automatique : `TransactionManager::commit` → `writeRequestedCheckpointIfLastNoLock` →
  `Checkpointer::writeCheckpoint` → `StorageManager::finalizeCheckpoint` → `PageManager::finalizeCheckpoint` →
  `FreeSpaceManager::finalizeCheckpoint`. Il écrit le vecteur `uncheckpointedFreePageRanges` : il en évince les pages,
  le déplace (`std::move`) dans `mergePageRanges`, puis le vide. **Il ne tient aucun verrou** :
  `PageManager::finalizeCheckpoint` (`page_manager.cpp:87-89`) ne prend pas `mtx`.
- **T5**, un fil de requête : le dernier `shared_ptr` d'un `ChunkedNodeGroup` lâché → `~StringChunkData` →
  `~DictionaryChunk` → `~ColumnChunkData` → `PageManager::freePageRange` →
  `FreeSpaceManager::addUncheckpointedFreePages`, un `push_back` sur le même vecteur, **sous `mtx`**.

Les deux accès ne sont pas ordonnés. Selon l'entrelacement :
- une plage poussée entre le déplacement et le `clear` est perdue : ses pages ne redeviennent jamais libres ;
- un `push_back` qui réalloue pendant que T1 parcourt ou déplace le vecteur écrit dans un bloc que `malloc` a peut-être
  déjà redonné à un autre objet.

`rollbackCheckpoint` vide le même vecteur, lui aussi sans `mtx`.

## Le correctif prévu

Prendre `mtx` dans `PageManager::finalizeCheckpoint` et dans `PageManager::rollbackCheckpoint`. L'ordre des verrous
est déjà celui de `freeImmediatelyRewritablePageRange`, qui tient `mtx` pendant l'éviction des pages : `mtx`, puis le
gestionnaire de tampons. Sous `mtx`, aucun chemin ne prend un verrou du point de reprise ou du gestionnaire de
transactions.

## Les témoins

1. Un stress de l'allocateur : des fils rendent des pages réelles, allouées d'abord, pendant que le fil principal
   appelle `finalizeCheckpoint` en boucle. À la fin, toutes les pages rendues doivent être libres ; aucune ne doit
   manquer.
2. Le même test sous TSan : la course y est nommée avant le correctif, et muette après.
Les deux sont à voir rouges avant le correctif, puis verts.
