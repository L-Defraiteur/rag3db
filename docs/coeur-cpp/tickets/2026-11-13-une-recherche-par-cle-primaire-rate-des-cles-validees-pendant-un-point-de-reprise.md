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

## La lecture sale de recherche : non établi

Le ticket `docs/recherche/tickets/2026-11-13-une-lecture-pendant-un-lot-reindexe-voit-le-lot-a-moitie-sous-la-lib-entiere.md`
(test `backend::tests::a_read_during_a_reindexed_batch_sees_the_batch_before_whole`) a son
journal L3 au portable :
`~/.cache/rag3weaver-build/archives/lecture-sale-L3-portable-2112/lib.log`.

Je l'ai lu le 13 nov. La pause du lot y est à la ligne 2259 : `[montre] 2 fichiers
(neuf/__pause_du_lot__.rs, neuf/def.rs) réindexé : verrou attendu 0 ms, tenu 3737 ms`. Le journal
ne dit **ni oui ni non** pour un point de reprise pendant cette pause :

- les 33 lignes qui contiennent « checkpoint » sont des noms de tests Rust (`dataflow::checkpoint`,
  `catalog::tests::checkpoint_*`), aucune n'est le moteur ;
- la lib ne trace un point de reprise que sous `RAG3DB_PROFILE_CHECKPOINT`
  (`checkpoint_profile.h`), absent de cette passe.

Pour trancher, il faut rejouer le test sous `RAG3DB_PROFILE_CHECKPOINT=1`. Si un point de reprise
tombe dans la pause, leur test est la reproduction de ce défaut sous charge ; sinon, c'est une
autre cause.

## Piste de correctif (à valider avec c7 avant le code)

Il ne suffit pas de réordonner la publication. Ma première idée était de publier les en-têtes de
lecture avant de vider la mémoire locale, et de publier après `applyShadowPages` ; cela ne
suffit pas. `Checkpointer::holdReadersLock` (`checkpointer.cpp` l. 130) est un verrou de
**fichier**, pris contre les ouvertures en lecture seule d'autres processus. Les lecteurs du même
processus continuent pendant `applyShadowPages`, qui réécrit **sur place** les pages des slots :
les fentes déplacent des clés existantes. Un lecteur pris pendant l'application lit l'ancien
en-tête sur une page déjà réécrite.

Proposé à c7 le 13 nov. :

- un `shared_mutex` par `PrimaryKeyIndex`, pris **en partagé** par `lookup`, pour la paire
  mémoire locale + disque ;
- pris **en exclusif** par le point de reprise, de l'application des pages ombres jusqu'à la
  publication en mémoire ;
- `checkpointInMemory` de l'index de clé primaire passe **après** `applyShadowPages`, et publie
  les en-têtes de lecture **avant** de vider la mémoire locale.

Seules les recherches par clé attendent, et seulement pendant l'application. Les autres lecteurs
V2 ne sont pas touchés.

Témoins à jouer après le correctif : `PkLookupUnderCheckpoint` (rouge puis vert), puis
`transaction_test` et `concurrence_test` entiers, puis `DdlEveryMinute` en trois tours, sans
toucher à sa ligne known_red sans mesure.
