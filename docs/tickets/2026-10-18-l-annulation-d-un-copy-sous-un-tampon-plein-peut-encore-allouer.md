# L'annulation d'un COPY sous un tampon plein peut encore allouer (à confirmer)

- **État** : ouvert, à confirmer (18 octobre 2026, seconde session cœur C++, sur la relecture du
  cœur C++). Ce défaut reste après la correction du ticket fermé
  `closed/2026-10-18-un-rollback-sous-un-tampon-plein-bloque-la-base-et-la-fermeture-tue-le-processus.md`.
- **Gravité** : blocage possible. Le ROLLBACK serait refusé, comme dans ce ticket, mais sans plus
  bloquer la base. La transaction est retirée et l'erreur nommée. Si l'annulation était déjà à
  moitié faite, la base est à rouvrir.
- **Atteignable en service** : oui en principe, par un COPY annulé (ou refusé) sous un tampon plein
  de pages validées non reportées.
- **Touche rag3weaver** : oui, par ses COPY.

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

## Pour le fermer

1. Écrire le témoin et le jouer, pour confirmer ou infirmer.
2. S'il est rouge : que `rollbackPKIndexInsert` ne lise que les lignes du COPY, qui sont en
   mémoire puisque non reportées, sans épingler de page sur disque, et que la suppression d'une
   clé n'alloue pas, au besoin par une réserve prise avant l'annulation.
