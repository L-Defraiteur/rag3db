# Conception : un ALTER ou un DROP attend les lecteurs de sa table (19 novembre 2026)

Ticket : `docs/coeur-cpp/tickets/2026-11-18-un-alter-ou-un-drop-n-attend-pas-les-lecteurs-de-sa-table.md`.
Écrite avant le code, pour accord de c7 (et de Lucie si elle le veut).

## Le problème

Le verrou de schéma d'une table (genre TABLE, lot `101ff50a6`) est pris en partagé par les écrivains,
en exclusif par un ALTER ou un DROP TABLE. Les lecteurs ne le prennent pas. Deux défauts en venaient,
couverts en surface le 19 novembre (pile `9906f44d69`) :
- un ADD COLUMN réallouait le vecteur des colonnes sous un lecteur : un verrou partagé sur le vecteur
  (`d6fb388ad9`) ;
- un DROP COLUMN puis un point de reprise détruisaient une colonne qu'un lecteur plus ancien relit :
  le lecteur est refusé « Schema changed, retry: » (`ac39dd1db4`), son instantané n'est pas honoré.

## La forme proposée (celle de PostgreSQL)

1. **Un lecteur prend le verrou TABLE en partagé**, à la première lecture d'une table dans sa
   transaction, et le garde jusqu'à la fin de la transaction (rendu par `clearTransactionNoLock` →
   `LockManager::releaseAll`, comme pour les écrivains). Point d'accroche : `TableScanState::setToTable`
   (balayages de nœuds et de relations), et les autres entrées de lecture d'une table (recherche par
   clé primaire, recherche HNSW), relevées une à une dans le lot. Un ensemble par transaction évite de
   reprendre le verrou à chaque balayage, comme `tablesLockedForWrite`.
2. **`Transaction::usesLocks()` s'étend aux transactions de lecture**, sous la même condition
   (`enableMultiWrites`). Sans elle, rien ne change.
3. **Après la prise du verrou, le même contrôle que pour un écrivain** :
   `throwIfSchemaChangedAfterSnapshot`. Un lecteur dont la transaction a commencé avant un ALTER validé,
   et qui lit la table pour la première fois après, est refusé « Schema changed, retry: » (la
   disposition des colonnes de son instantané n'existe plus). Le refus de `ac39dd1db4` reste en filet.
4. **L'ALTER et le DROP TABLE ne changent pas** : exclusif, ils attendent. Le CREATE_VECTOR_INDEX garde
   le partagé.

## Ce que ça change

- Un lecteur ouvert voit toujours ses colonnes : un DROP l'attend. Le témoin
  `DroppedColumnUnderOpenReader` change de contrat : le DROP attend la fin du lecteur, et le lecteur
  relit sa colonne (son instantané est honoré), au lieu d'être refusé.
- **Le prix, à accepter ou non** :
  - un ALTER ou un DROP attend la plus longue transaction de lecture ouverte sur la table ;
  - **les lecteurs qui arrivent pendant qu'un ALTER attend attendent derrière lui** (file FIFO du
    gestionnaire de verrous) : une lecture longue plus un ALTER en attente gèlent les lectures de la
    table. C'est le comportement de PostgreSQL ; on le borne par le délai de verrou de l'ALTER
    (`lock_timeout`, dont le refus nomme déjà la transaction attendue) ;
  - un verrou partagé par table lue et par transaction, sur le chemin de toute lecture : à mesurer
    (banc des lectures, `e2e_lectures`) avant le push.
- En service, l'hôte ne fait d'ALTER qu'à l'ouverture ou à une montée de schéma : le gel est rare.
  Un lecteur long (une lecture de rag3weaver qui dure) retarderait la montée de schéma, pas l'inverse.

## Les preuves

- Rouge d'abord : `DroppedColumnUnderOpenReader` réécrit au contrat de l'attente (le DROP dans un fil,
  qui ne doit finir qu'après le COMMIT du lecteur ; le lecteur relit sa colonne) : rouge sur master.
- `AddColumnUnderReaders` vert sous TSan (le verrou des colonnes reste, en ceinture).
- Un témoin du gel borné : un lecteur long, un ALTER sous `lock_timeout`, un lecteur neuf qui attend
  au plus ce délai.
- `transaction_test`, known_red, `e2e_test`, et la mesure du coût sur les lectures.

## À trancher

Le gel des lectures derrière un ALTER en attente : l'accepter (PostgreSQL), ou le refuser et faire
passer les lecteurs neufs devant un ALTER en attente. Dans ce second cas, un ALTER peut attendre sans
fin sous des lectures continues. Ma recommandation : la forme de PostgreSQL, avec un `lock_timeout`
par défaut pour l'ALTER (quelques secondes) et le refus nommé.
