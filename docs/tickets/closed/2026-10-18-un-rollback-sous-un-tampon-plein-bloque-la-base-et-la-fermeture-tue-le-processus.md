# Sous un tampon plein, un ROLLBACK est refusé et bloque la base, et la fermeture d'une connexion tue le processus

- **État** : corrigé par le commit qui déplace ce ticket (18 octobre 2026, seconde session cœur
  C++, forme relue par le cœur C++) ; voir « Ce qui a été fait », à la fin.
- **Gravité** : plantage du processus (SIGABRT à la fermeture d'une connexion). La base est
  bloquée sans autre issue que la fermeture, ou condamnée à être rouverte, sans qu'aucune donnée
  validée soit perdue.
- **Atteignable en service** : oui, dès que le tampon se remplit de pages validées et pas encore
  reportées. Le scénario des vingt écrivains y arrive avec un tampon de 2 Gio en moins de deux
  minutes, avec le point de reprise automatique coupé ou un seuil relevé à 64 Mio (voir
  `2026-10-18-un-point-de-reprise-fige-toutes-les-transactions-le-temps-de-s-ecrire`, section
  « La première voie, mesurée »).
- **Touche rag3weaver** : oui. L'hôte garde ses connexions : la fermeture de l'une d'elles sous
  un tampon plein le tue.
- **Ouvert le** : 18 octobre 2026, banc de concurrence, à la demande de l'orchestration.
- **Pour** : hotfixs, après le CREATE concurrent ; relecture par le cœur C++.

## Ce qu'on voit

Le scénario des vingt écrivains, avec le point de reprise automatique coupé et un tampon de 2 Gio
plein, a rendu 39 fois « Connection already has an active transaction » au BEGIN d'un bloc.
Après chaque échec dans un bloc, le harnais envoie ROLLBACK. C'est ce ROLLBACK que le tampon
plein a refusé, et la transaction est restée ouverte.

Une sonde à deux connexions sur un fil le reproduit de façon déterministe, avec le tampon des
tests (64 Mo) et le point de reprise automatique coupé. Elle a été jouée deux fois sur deux, à
62083d8be, sur luciepc :

1. A : BEGIN, puis 20 000 lignes de `Doc(id INT64 PRIMARY KEY, titre STRING, vec FLOAT[64])`,
   toutes acceptées.
2. B, en automatique : des lots validés, par pas qui se divisent par deux à partir de 5 000,
   jusqu'à vingt refus. Le tampon est alors plein au plus juste de pages validées que rien ne
   peut rendre (166 250 lignes).
3. A : ROLLBACK, refusé (« Buffer manager exception: Unable to allocate memory! The buffer pool is
   full and no memory could be freed! »). ROLLBACK de nouveau, refusé. Il ne passera jamais,
   puisque rien ne rend de mémoire.
4. BEGIN de A, CHECKPOINT de B et `MATCH` de A sont tous refusés de même : la base est bloquée.
5. À la sortie de la portée, les connexions se ferment, et le processus meurt :

```
terminate called after throwing an instance of 'rag3db::common::BufferManagerException'
  what():  Buffer manager exception: Unable to allocate memory! The buffer pool is full and no memory could be freed!
#9  TransactionManager::rollback(ClientContext&, Transaction*) [clone .cold]
#10 ClientContext::~ClientContext()
#11 Connection::~Connection()
```

Avec un premier pas de 20 000, le tampon est presque plein, mais pas tout à fait. Le ROLLBACK de A
passe. Le CHECKPOINT de B est alors refusé par le tampon (« Runtime exception: Buffer manager
exception: Unable to allocate memory! … »), et dès lors tout est refusé pour tout le monde :
« A checkpoint of this database failed, so it must be closed and reopened before it is used
again ». Le point de reprise n'avait pourtant rien écrit.

## Deux défauts, et un troisième rangé ailleurs

1. **Un ROLLBACK a besoin de mémoire.** Le cœur l'a lu : l'annulation rejoue le tampon
   d'annulation, et `rollbackPKIndexInsert` relit les colonnes pour retirer les clés de l'index,
   ce qui alloue. Si cette relecture lève, `clearTransaction` n'est pas atteint, et le contexte
   garde sa transaction à demi annulée. Chez PostgreSQL, l'abandon réussit toujours.
2. **Un destructeur lève.** `ClientContext::~ClientContext` annule la transaction ouverte, et
   l'exception du tampon en sort : `std::terminate`. Un destructeur ne doit jamais lever.
3. **Un CHECKPOINT refusé faute de mémoire condamne la base**, alors que le refus est arrivé
   avant toute écriture. Ce point n'est pas pour ce ticket. Le cœur l'a lu comme la règle
   actuelle, et l'orchestration l'a suivi : `writeCheckpoint` réécrit en place avant l'en-tête, et
   le code ne distingue pas « refusé avant la première écriture » de « cassé au milieu ».
   Condamner la base est donc la règle prudente d'aujourd'hui. Le témoin
   `FullBufferRollback.RefusedCheckpointLeavesTheDatabaseOpen` est rangé sous le ticket du point
   de reprise (`2026-10-18-un-point-de-reprise-fige-toutes-les-transactions-le-temps-de-s-ecrire`),
   voie 2 : c'est elle qui doit permettre cette distinction.

Le fond est le même que celui du ticket du point de reprise : le tampon ne peut pas rendre les
pages validées pas encore reportées.

## Témoins

`test/transaction/concurrence/full_buffer_rollback_test.cpp`, dans `known_red.txt`. Chaque cas se
joue dans un processus fils.
- `FullBufferRollback.RollsBackAndClosesCleanly` (premier pas 5 000) :
  - `rollback-under-a-full-buffer-succeeds` : l'un des deux ROLLBACK passe ;
  - `closing-a-connection-does-not-abort` : le fils sort sans signal en fermant ses connexions.
- `FullBufferRollback.RefusedCheckpointLeavesTheDatabaseOpen` relève du ticket du point de reprise
  (voir le point 3).

Fermé par hotfixs, avec la relecture du cœur, quand les deux contrôles ci-dessus sont verts.

## Ce qui a été fait (18 octobre 2026)

**1. Un ROLLBACK n'alloue rien qui puisse manquer.** `LocalStorage::rollback` relâche ses tables
locales au lieu de les vider. `LocalNodeTable::clear` rebâtissait un index de hachage, soit une
page demandée au tampon, et c'était l'allocation refusée. Le chemin du commit, qui vide encore,
libère l'ancien index et les groupes avant de rebâtir.

**2. La transaction quitte toujours le système.** `TransactionManager::rollback` la retire
(`clearTransactionNoLock`, verrous rendus) même si son annulation lève, puis lève une erreur
nommée (« The rollback could not complete (…). The transaction is closed and its locks are
released »). Le refus ne condamne la base que si l'annulation a commencé de défaire le stockage
partagé ou le catalogue sans finir (`UndoBuffer::rollback` dit s'il a commencé,
`Transaction::hasHalfDoneRollback`). Elle est alors à rouvrir (`REOPEN_AFTER_FAILED_ROLLBACK`).
Un échec du seul stockage local, après un undo complet (le cas de ce ticket), ne condamne rien.
- `TransactionContext::rollback` oublie sa transaction dans tous les cas.
- `Transaction::failAfterStatement` marque la transaction échouée avant l'annulation : le
  ROLLBACK de clôture ne la rejoue pas (le retrait des lignes n'est pas idempotent).

**3. Un destructeur ne lève pas.** `~ClientContext` rattrape l'erreur de l'annulation et l'écrit
(« [rag3db] closing a connection: the rollback of its open transaction failed: … »).

**4. Un CHECKPOINT refusé avant d'avoir rien changé reste une erreur nommée.**
`StorageManager::markCheckpointChangedMemory` est posé au premier pas qui ne se défait pas :
- l'échange de groupes de `NodeGroup::checkpoint`, ou son effacement d'un groupe tout supprimé ;
- après le point de reprise des groupes d'une table de nœuds ;
- au début d'une table de relations modifiée ;
- avant une table retirée ;
- après tout le stockage.

`NodeTable::checkpoint` ne déplace plus ses colonnes qu'après le succès de ses groupes (relu
avec le cœur : déplacées d'abord, un échec laissait les numéros du catalogue faux). Avant ce
drapeau, `checkpointNoLock` rend les pages (`Checkpointer::rollback`), rouvre les BEGIN et lève
« … the checkpoint was refused before it changed anything. Every committed transaction is in the
journal, and the database stays open. ».

Il condamne quand même la base si ce point de reprise porte une validation forcée
(`checkpointCarriesForcedCommit` : la validation forcée elle-même, ou une validation de la file
A7). Faite en mémoire, elle n'est durable que par lui.

**Témoins.** Ils sont rouges sur `2def4bed0` et verts avec le lot, trois passes chacun, sur
luciepc :
- `FullBufferRollback.RollsBackAndClosesCleanly` et `FullBufferRollback.RefusedCheckpointLeavesTheDatabaseOpen`,
  sortis de known_red ;
- `FlakyCheckpointerTest.ARefusedCheckpointThatChangedNothingLeavesTheDatabaseOpen`, témoin
  unitaire neuf, rouge sur master (« must be closed and reopened »).

Les tests de la condamnation (`RefusedBegin`, `SingleWriterCrash.FailedCheckpointThenDeath…`)
gardent leur intention : leur faux Checkpointer pose le drapeau avant de lever.

**Ce qui reste.** L'annulation d'un COPY sous un tampon plein passe par
`rollbackPKIndexInsert`, qui peut épingler des pages et allouer. Ce cas a son ticket, à
confirmer : `2026-10-18-l-annulation-d-un-copy-sous-un-tampon-plein-peut-encore-allouer`.

