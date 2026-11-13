# Une recherche par clé primaire rate des clés validées pendant un point de reprise

- **État** : ouvert (13 novembre 2026, cœur c++)
- **Famille** : moteur
- **Gravité** : lecture fausse (une rangée validée est dite absente)
- **Atteignable en service** : oui. Il suffit d'un lecteur READ ONLY qui cherche par clé pendant
  qu'un point de reprise V2 s'écrit, que ce point de reprise soit automatique ou demandé.
- **Touche rag3weaver** : par le moteur. Un `MATCH (n:T {clé: …})` peut rendre 0 rangée pour une
  clé validée avant le BEGIN.
- **Signalé par** : cœur c++. Le témoin est décrit ci-dessous.
- **Pour** : cœur c++

## Portée

- **Touché** : la recherche par clé primaire (`MATCH (d:Doc {id: k})`, et tout plan qui passe par
  `PrimaryKeyIndex::lookup`) d'une transaction **READ ONLY** admise pendant l'écriture d'un point
  de reprise. Depuis V2-1c, un lecteur entre pendant `checkpointWriting`
  (`transaction_manager.cpp`, `waitWhileCheckpointPendingNoLock`).
- **Pas touché, d'après le code** : un écrivain. `waitWhileCheckpointPendingNoLock` le retient
  tant que `checkpointPending` est vrai, donc un MERGE ou un CREATE ne cherche jamais dans
  l'index pendant la fenêtre. L'insertion vérifie l'unicité par l'index lui-même. Le moteur seul
  ne fabrique donc **pas de doublon** de clé par ce défaut.
- **Doublon possible hors du moteur** : un client qui lit en READ ONLY (« absente »), puis crée
  dans une autre transaction sur la foi de cette lecture. C'est un motif d'appelant, à chercher
  dans rag3weaver. hotfixs sonde les bases de Lucie (sujet Lucivy_Scope « doublons vivants »).
  Mon témoin ne laisse aucune base à doublons et ne peut servir de rouge à cette sonde (réponse
  envoyée à hotfixs le 13 nov.).

## Le témoin

`test/transaction/pk_lookup_under_checkpoint_test.cpp`, branche `cle-primaire-sous-point-de-reprise`
`6d8619f49`.

Le scénario tourne dans un enfant forké :

- **l'écrivain** fait 400 lots de 100 clés, avec un `CHECKPOINT` tous les 5 lots, et annule un lot
  sur deux dans la variante à annulation ;
- **le lecteur** relève `upTo = committed` **avant** son `BEGIN TRANSACTION READ ONLY`, puis
  cherche par clé une clé validée `≤ upTo` ;
- un compte de 0 sort en code 3.

Rouge sur master `ca25b28e8` (base de la branche, sans code moteur en plus), à luciepc :

```
[témoin] clé 36 manquée : compte 0, 500 clés validées
[témoin] clé 500 manquée : compte 0, 1000 clés validées
```

500 et 1000 sont exactement les seuils où le point de reprise passe (5 lots × 100 clés). Les clés
ratées sont celles que ce point de reprise déplace de la mémoire de l'index vers le disque.

## Ce que dit le code

Ordre de `Checkpointer::writeCheckpoint` (`src/storage/checkpointer.cpp`) :

1. `checkpointStorage()` (l. 86), qui appelle `PrimaryKeyIndex::checkpoint`. Celui-ci écrit les
   slots par les pages ombres, écrit les en-têtes, vide les cadres sales, **puis appelle
   `checkpointInMemory()` lui-même** (`hash_index.cpp`, fin de `PrimaryKeyIndex::checkpoint`).
2. `serializeCatalogAndMetadata`, `writeDatabaseHeader`.
3. `logCheckpointAndApplyShadowPages()` (l. 94) : les pages ombres sont recopiées sur les pages
   d'origine.
4. `finalizeCheckpoint()`.

Deux fenêtres en découlent pour un lecteur READ ONLY.

**Fenêtre 1, dans `PrimaryKeyIndex::checkpointInMemory`** (`hash_index.cpp` l. 633) :

- `HashIndex<T>::checkpointInMemory` publie les pips (`pSlots/oSlots->checkpointInMemoryIfNecessary`)
  puis fait `localStorage->clear()`, index par index ;
- **seulement après**, `hashIndexHeadersForReadTrx[i] = hashIndexHeadersForWriteTrx[i]` et
  `hashIndexDiskArrays->checkpointInMemory()`.

Entre les deux, `lookupInternal` (`hash_index.h` l. 103) ne trouve plus la clé dans la mémoire
locale, qui vient d'être vidée. Il cherche alors sur disque avec l'**ancien** en-tête de lecture
(niveau et `nextSplitSlotId` anciens), sur des slots déjà fendus. La clé déplacée par une fente
n'est pas dans le slot calculé.

**Fenêtre 2, entre `checkpointInMemory` et `applyShadowPages`** : `DiskArrayInternal::get` d'une
transaction qui n'est pas CHECKPOINT lit la page d'**origine** par les nouveaux pips. Une page
existante réécrite par le point de reprise n'a encore son nouveau contenu que dans le fichier
ombre. Le lecteur lit l'ancien slot, sans les clés fusionnées par `mergeBulkInserts`.

Aucun verrou ne couvre à la fois « mémoire locale puis disque » dans `lookupInternal` et la
publication. La recherche n'en prend aucun ; `DiskArrayInternal::get` prend un verrou partagé
seulement sur la lecture d'un élément.

## Même famille, probablement

`2026-11-13-sous-vingt-ecrivains-un-lecteur-meurt-dans-l-index-de-cle-primaire-sur-disque.md` :
le SIGSEGV de `DdlEveryMinute` meurt dans `DiskArrayInternal::get` sous
`lookupInPersistentIndex`, avec des pips nuls (`rdx = 0`). C'est le même chemin de lecture,
pendant qu'une publication en mémoire remplace les pips sous le lecteur. Le correctif de ce ticket
se rejouera sur ce banc.

## La lecture sale de recherche : une autre famille, d'après le code

Il s'agit du ticket `docs/recherche/tickets/2026-11-13-une-lecture-pendant-un-lot-reindexe-voit-le-lot-a-moitie-sous-la-lib-entiere.md`
(test `backend::tests::a_read_during_a_reindexed_batch_sees_the_batch_before_whole`).

**Le journal ne tranche pas.** Sa L3 au portable
(`~/.cache/rag3weaver-build/archives/lecture-sale-L3-portable-2112/lib.log`, pause du lot l. 2259,
`tenu 3737 ms`) n'a aucune ligne de point de reprise du moteur :

- les 33 lignes qui contiennent « checkpoint » sont des noms de tests Rust ;
- la lib ne trace un point de reprise que sous `RAG3DB_PROFILE_CHECKPOINT`, absent de la passe.

**Le code tranche.** Recherche a établi que le lot de leur test est **une seule transaction
d'écriture tenue ouverte** pendant la pause (BEGIN, deux fichiers, 2 s, COMMIT). Or :

- `TransactionManager::checkpointNoLock` appelle `stopNewTransactionsAndWaitForOthersNoLock(…,
  readersToo = false)` ;
- celle-ci attend que **toutes les transactions d'écriture** actives soient parties, ou rangées
  dans la file des validations (`othersLeftOrQueued`, `transaction_manager.cpp` l. 372-379). Au-delà
  du délai, elle lève « Timeout waiting for active transactions… ».

Aucun point de reprise ne s'écrit donc pendant que le lot est ouvert.

Les symptômes sont d'ailleurs inverses. Leur lecteur voit **trop** : des rangées non validées
(`Scope` de `h`, `Symbol arrivee`, l'arête `MENTIONS`). Ce ticket-ci voit **trop peu** : une clé
validée dite absente. Leur test n'est pas la reproduction de ce défaut. Leur famille est celle de
la lecture sale sous V2 (`RollbackUnderReaders.AReaderSurvivesTheRollbackOfAnotherWriter`, 7 nov.).

## Pourquoi un verrou : cohérent par page, pas par index

C'est cette preuve qui justifie un verrou sur un chemin chaud (c7, 13 nov.).

**Au niveau de la page, V2 est cohérent.**

- `ShadowFile::applyShadowPages` (`shadow_file.cpp` l. 66-80) écrit la page dans le fichier,
  puis appelle `updateFrameIfPageIsInFrame`.
- Celui-ci (`buffer_manager.cpp` l. 484-509) attend qu'une page `LOCKED` (épinglée, ou en
  cours de chargement) soit rendue. Il la verrouille, recopie le cadre et la déverrouille.
- Le changement de version fait recommencer une lecture optimiste en cours. Une page `EVICTED`
  se recharge depuis le fichier, déjà écrit.
- `DiskArrayInternal::get` lit par `optimisticReadPage` (`disk_array.cpp` l. 79).

Un lecteur voit donc une page entière, ancienne ou neuve, jamais à moitié recopiée.

**Au niveau de l'index, ce n'est pas cohérent.**

- Les pages de slots sont réécrites **sur place**. Une fente déplace des clés du slot `s` vers
  `s + 2^niveau`, et la page de `s` passe par une page ombre.
- Si l'on publiait seulement après `applyShadowPages`, un lecteur admis pendant la recopie
  garderait l'ancien en-tête (niveau, `nextSplitSlotId`) et les anciens pips. Il calculerait `s`,
  puis lirait la page de `s` déjà neuve, d'où la clé est partie. Une clé déjà sur disque avant ce
  point de reprise n'est pas non plus dans la mémoire locale : elle est ratée.
- Le même mélange vaut entre un slot primaire et sa chaîne de débordement (`oSlots`), lus sur
  deux pages distinctes.

Aucune version épinglée ni copie ne couvre plusieurs pages. Réordonner déplace la fenêtre sans
la fermer : c'est le verrou qui la ferme. `holdReadersLock` (`checkpointer.cpp` l. 130) ne le
fait pas, car c'est un verrou de **fichier**, pris contre les ouvertures en lecture seule d'autres
processus. Les lecteurs du même processus lisent pendant la recopie.

## Correctif retenu (« à toi » de c7, 13 nov.)

- Un `shared_mutex` par `PrimaryKeyIndex` :
  - **partagé** dans `lookup`, sur la paire mémoire locale + disque ;
  - **exclusif** pris par le point de reprise, de `applyShadowPages` jusqu'à la publication.
- `checkpointInMemory` de l'index de clé primaire passe **après** `applyShadowPages`, et publie
  les en-têtes de lecture **avant** de vider la mémoire locale.

Deux bornes, à tenir et à dire :

1. **Le partagé ne couvre que la recherche.** Le lecteur ne le prend pas en tenant une page
   épinglée. Sous lui, il ne prend que le verrou partagé du tableau disque et les verrous des
   groupes de nœuds (visibilité), jamais l'inverse. Le point de reprise, qui tient l'exclusif, ne
   tient aucun de ces verrous en l'attendant : il a fini l'écriture des tables dans
   `checkpointStorage`, avant la recopie. L'ordre index de clé primaire → tableau disque est le
   même des deux côtés.
2. **L'exclusif est borné** par la recopie des pages ombres et la publication. Son temps se
   mesure en ms aux profils du point de reprise, dans `PkLookupUnderCheckpoint` vert.

Témoins à jouer après le correctif : `PkLookupUnderCheckpoint` (rouge puis vert), puis
`transaction_test` et `concurrence_test` entiers, puis `DdlEveryMinute` en trois tours, sans
toucher à sa ligne known_red sans mesure.

## Les bases de Lucie : aucun doublon (hotfixs, 13 novembre 2026)

Sonde en lecture seule sur des copies des instantanés `avant-835d5298f` (luciepc à 20 h 06, portable à
20 h 07), lib 835d5298f. Dans code-lourd, code-principal, code-rag3db-depot et memoire-longue : **0 clé
primaire en double et 0 clé logique en double**. Les clés logiques regroupent Scope par `key`, File par
`path`, Symbol par `_project` et `name`, Library par `_project` et `import_path`. Il n'y a donc aucun
morceau ni vecteur rattaché à un doublon.

La sonde a été vue rouge sur une base fabriquée qui porte deux Scope de même `key` sous deux uuid. Le
détail, le programme et le témoin sont dans
`docs/coeur-cpp-hotfixs-tickets/13-11-2026/02-sonde-des-doublons.md`. Pour memoire-longue, seules les
clés primaires sont couvertes.
