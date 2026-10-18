# Sous un tampon plein, un ROLLBACK est refusé et bloque la base, la fermeture d'une connexion tue le processus, et un CHECKPOINT refusé empoisonne la base

- **État** : ouvert
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

## Trois défauts

1. **Un ROLLBACK a besoin de mémoire.** Le cœur l'a lu : l'annulation rejoue le tampon
   d'annulation, et `rollbackPKIndexInsert` relit les colonnes pour retirer les clés de l'index,
   ce qui alloue. Si cette relecture lève, `clearTransaction` n'est pas atteint, et le contexte
   garde sa transaction à demi annulée. Chez PostgreSQL, l'abandon réussit toujours.
2. **Un destructeur lève.** `ClientContext::~ClientContext` annule la transaction ouverte, et
   l'exception du tampon en sort : `std::terminate`. Un destructeur ne doit jamais lever.
3. **Un CHECKPOINT refusé faute de mémoire empoisonne la base.** Le refus arrive avant toute
   écriture. Il doit rester une erreur nommée et laisser la base ouverte. L'état « à rouvrir »
   n'a de sens qu'après une écriture commencée.

Le fond est le même que celui du ticket du point de reprise : le tampon ne peut pas rendre les
pages validées pas encore reportées.

## Témoins

`test/transaction/concurrence/full_buffer_rollback_test.cpp`, dans `known_red.txt`. Chaque cas se
joue dans un processus fils.
- `FullBufferRollback.RollsBackAndClosesCleanly` (premier pas 5 000) :
  - `rollback-under-a-full-buffer-succeeds` : l'un des deux ROLLBACK passe ;
  - `closing-a-connection-does-not-abort` : le fils sort sans signal en fermant ses connexions.
- `FullBufferRollback.RefusedCheckpointLeavesTheDatabaseOpen` (premier pas 20 000) :
  - `refused-checkpoint-leaves-the-database-open` : après le CHECKPOINT refusé, un BEGIN passe.
