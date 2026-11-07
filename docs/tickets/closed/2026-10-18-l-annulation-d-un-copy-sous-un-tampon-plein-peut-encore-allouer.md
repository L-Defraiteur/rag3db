# L'annulation d'un COPY sous un tampon plein peut encore allouer (à confirmer)

- **État** : non reproduit le 7 novembre 2026 (cœur c++) : le témoin écrit sur la recette du ticket
  (`CopyRollbackUnderAFullBuffer.RollsBackAndKeepsTheDatabaseOpen`,
  `test/transaction/concurrence/copy_rollback_full_buffer_test.cpp`) est vert sur master ; il reste
  au banc en garde.
- **Gravité** : blocage possible. Le ROLLBACK serait refusé, comme dans ce ticket, mais sans plus
  bloquer la base. La transaction est retirée et l'erreur nommée. Si l'annulation était déjà à
  moitié faite, la base est à rouvrir.
- **Atteignable en service** : oui en principe, par un COPY annulé (ou refusé) sous un tampon plein
  de pages validées non reportées.
- **Touche rag3weaver** : oui, par ses COPY.
- **Famille** : moteur

## Ce que c'est

Le ticket fermé a retiré l'allocation sûre du ROLLBACK, l'index de hachage local rebâti à vide.
Celle-ci ne venait que des lignes locales : `LocalStorage::rollback` relâche désormais ses
tables. Un COPY, lui, écrit ses lignes directement dans les groupes validés, et son annulation
passe par les enregistrements INSERT_INFO :
`NodeTableVersionRecordHandler::rollbackInsert` → `NodeTable::rollbackPKIndexInsert`.

Ce chemin relit les clés des lignes annulées par `scanIndexColumns`, sur les groupes validés. Ce
balayage peut épingler des pages d'un groupe sur disque (`BufferManager::optimisticRead` →
`pin`), ce qui demande un cadre au tampon. La suppression des clés dans l'index de clé primaire
peut, elle aussi, allouer (division de blocs, selon le cœur).

Sous un tampon plein, l'annulation peut donc lever au milieu du stockage partagé. Depuis le
ticket fermé, ce cas est rattrapé : la transaction est retirée, et la base est condamnée, à
rouvrir. Ce n'est pas un ROLLBACK qui réussit.

## Recette, à écrire

Celle de `FullBufferRollback.RollsBackAndClosesCleanly`
(`test/transaction/concurrence/full_buffer_rollback_test.cpp`), avec un `COPY` de la connexion A
à la place de ses `CREATE`. Ensuite, sous le tampon rempli par B, `ROLLBACK`. Le témoin attend
« ROLLBACK passed », et une base qui reste ouverte.

## Le 7 novembre 2026 : non reproduit

Le témoin suit la recette : la connexion A ouvre une transaction et fait un COPY de 20 000 lignes
dans Doc ; B remplit le tampon des tests (64 Mo, point de reprise automatique coupé) par des lots
validés qui se divisent par deux jusqu'à vingt refus, dans une autre table (le COPY de A tient Doc
jusqu'à sa fin : une première forme du témoin, B écrivant dans Doc, ne remplissait rien et passait
à tort ; le plan exige désormais des lignes de B) ; puis A fait ROLLBACK. Sur master, B valide
151 250 lignes avant ses refus, le ROLLBACK de A passe du premier coup, un BEGIN de A passe
ensuite, Doc est vide, et le processus se ferme sans signal.

Le ROLLBACK d'un COPY relit ses clés et les retire de l'index de clé primaire sans qu'un cadre
lui manque : les pages du COPY, non reportées, sont en mémoire et tenues par le tampon. Le cas
d'une suppression de clé qui diviserait un bloc de l'index sous un tampon plein n'est pas couvert
par ce témoin ; il se rouvrira avec sa recette si un banc le montre.

## Pour le fermer

1. Écrire le témoin et le jouer, pour confirmer ou infirmer.
2. S'il est rouge : que `rollbackPKIndexInsert` ne lise que les lignes du COPY, qui sont en
   mémoire puisque non reportées, sans épingler de page sur disque, et que la suppression d'une
   clé n'alloue pas, au besoin par une réserve prise avant l'annulation.
