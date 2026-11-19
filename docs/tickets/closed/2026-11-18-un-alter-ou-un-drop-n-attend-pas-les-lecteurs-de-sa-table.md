# Un ALTER ou un DROP n'attend pas les lecteurs de sa table

- **État** : corrigé sur master par la pile C++ du 19 novembre 2026 (tête testée `4ab4ab8150`, cœur c++)
- **Famille** : moteur
- **Gravité** : la cause commune de deux défauts couverts par des correctifs de surface
- **Atteignable en service** : oui, par un lecteur long pendant une montée de schéma
- **Touche rag3weaver** : par le moteur (les lecteurs refusés « Schema changed, retry: »)
- **Signalé par** : cœur c++
- **Pour** : cœur c++

## Ce qui se passe

Le verrou de schéma d'une table (genre TABLE du gestionnaire de verrous, lot `101ff50a6`) est pris en
partagé par les écrivains, et en exclusif par un ALTER ou un DROP TABLE. Les **lecteurs ne le prennent
pas**. Deux défauts en viennent :
- un ADD COLUMN réalloue le vecteur des colonnes pendant qu'un lecteur y lit (ticket
  `2026-11-18-un-add-column-realloue-les-colonnes-d-une-table-sous-un-lecteur.md`) ; corrigé en surface
  par un verrou partagé sur ce vecteur (branche `add-column-under-readers`) ;
- un DROP COLUMN suivi d'un point de reprise détruit une colonne qu'un lecteur plus ancien voit encore
  et relit : SIGSEGV (ticket
  `2026-11-18-le-point-de-reprise-detruit-une-colonne-retiree-qu-un-lecteur-plus-ancien-tient-encore.md`) ;
  corrigé en surface par un refus nommé du lecteur (branche `dropped-column-under-old-reader`).

## La forme de fond

Comme PostgreSQL (ACCESS SHARE pris par toute lecture, ACCESS EXCLUSIVE par l'ALTER) : un lecteur prend
le verrou de table en partagé pour la durée de sa transaction, et un ALTER ou un DROP l'attend. Le
lecteur ne voit alors jamais une colonne naître ou disparaître sous lui, et le refus nommé devient un
filet rarement servi.

Ce que ça coûte, à mesurer avant de choisir : un verrou partagé par table lue et par transaction (le
gestionnaire de verrous sur le chemin de toute lecture), et un ALTER qui attend la plus longue
transaction de lecture ouverte. Sous V2-2, un lecteur ouvert ne retient plus le point de reprise ; il
retiendrait l'ALTER.

## Pour fermer

La conception écrite et accordée par l'orchestration ; le verrou des lecteurs ; les deux témoins
(`AddColumnUnderReaders`, `DroppedColumnUnderOpenReader`) verts sur le chemin de l'attente (le lecteur
lit sa colonne, le DROP attend) ; la mesure du coût sur les lectures.

## Fermé (19 novembre 2026)

La forme (c), accordée par c7 : une transaction qui lit une table tient le verrou de ses lecteurs
(`LockResourceKind::TABLE_READ`) en partagé, de sa première lecture (`TableScanState::setToTable`) à sa fin. Un ADD
ou un DROP de colonne et un DROP TABLE le prennent en exclusif avant le verrou de schéma, et attendent ; un COMMENT,
un RENAME, un CREATE ou DROP d'index ne le prennent pas (le DROP du graphe d'un index HNSW garde la rétention des
lecteurs d'avant). L'attente est bornée par `alter_lock_timeout` (5 s par défaut, jamais plus que `lock_timeout`) ;
les lecteurs neufs attendent derrière un ALTER en attente (file FIFO, comme PostgreSQL). Le refus nomme les
transactions attendues : « reader N (read-only, open for X ms) », « writer N (open for X ms) ». Pas de contrôle de
schéma pour un lecteur : le refus nommé de `ac39dd1db4` reste le filet sans les écritures multiples.

Témoins (`drop_waits_for_readers_test.cpp`) : rouges 3/3 sur `c802e8c5b0`, verts 3/3 sur la tête. La première forme
(`d7a617c9eb`, le verrou de table lui-même) cassait cinq tests (rétention HNSW, COMMENT, ADD sous lecteurs, délai et
message de l'ALTER) : reprise en `6c5f6d5218`. Passe de la tête `4ab4ab8150` : `transaction_test` 407, known_red,
`.test` de vector 74, `e2e_test` 1 867 avec `DropNodeTableReclaim` admis. Le coût du verrou de lecture sur le
chemin des lectures n'est pas mesuré : à faire au banc des lectures.
