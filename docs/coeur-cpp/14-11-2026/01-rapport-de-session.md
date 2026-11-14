# Cœur c++, 14 novembre 2026 : rapport de session

## La perte de Note sous un pool plein, correctif (a)

Ticket : `docs/coeur-cpp/tickets/2026-11-14-sous-vingt-ecrivains-et-un-pool-plein-des-note-validees-manquent.md`.

**Les preuves du correctif** (branche `annulation-garde-les-autres-pile-3`, tête `27cfe33a8`, le
`src/` identique à `9b0e73c9c`) :

| Témoin | Sans le correctif | Avec le correctif |
|---|---|---|
| `RefusedCommitRollback.AWriterCommittingWhileARefusedCommitUnwindsKeepsItsRow` | rouge 3/3 : 20 lignes de B perdues sur 20 | vert 5/5 |
| `TwentyWriters.DdlEveryMinute` ×10, `counts-match-commits` | 3/10 (`f41c0ba5b`, et avec le verrou de la clé primaire) | 0/10, sous jusqu'à 1 327 refus du pool par tour |
| `transaction_test` entier | — | vert, 376 |

**L'annulation sous le verrou, mesurée** (`[checkpoint-profile] annulation d'un COMMIT refusé sous
le verrou`) :

- dans le témoin, 110 mesures pour 60 000 lignes chacune : médiane 17,6 ms, max 28,9 ms ;
- dans `DdlEveryMinute`, 2 478 mesures : médiane 0,0 ms, p99 0,3 ms, max 5 875 ms.

Le max est une seule annulation, celle d'un `CREATE_VECTOR_INDEX` dont le COMMIT forcé a été
refusé. Ce n'est pas un coût neuf : `TransactionManager::rollback` tenait déjà ce verrou pendant
toute l'annulation. Le correctif supprime l'entre-deux, pas du temps.

**SingleWriterCrash n'est pas une preuve du correctif (a).**
`SingleWriterCrash.DeathDuringDeletesAndVectorSearchKeepsTheIndexExact` a rougi une fois au juge
`concurrence_test.known_red` du correctif, sur `vector-extension-loaded`. C'est le cas du ticket
`docs/coeur-cpp-hotfixs-tickets/tickets/2026-10-10-ouverture-sans-une-extension-dont-une-table-depend.md`,
une manche tuée à 633 ms avec 2 000 documents, rouverte sans l'extension.

La série seule ne départageait que ce rouge d'extension :

- 0 rouge sur 36 essais avec le correctif ;
- 0 sur 26 sans le correctif. La série a été arrêtée là : les 10 restants n'apportaient rien
  (c7) ;
- 0 sur 36 le 7 novembre.

Ce cas ne sort que sous la charge du banc complet. Le même 0 des deux côtés le dit indépendant
du correctif, qui n'est pas sur son chemin : c'est une mort par SIGKILL, ni COMMIT refusé ni
troncature. Les preuves du correctif sont celles du tableau.

**Le juge** est rejoué sur la tête de pile `27cfe33a8`, celle qui sera poussée. Elle porte aussi
les trois cas du témoin du dictionnaire de hotfixs dans `concurrence_test`.

**Le journal `temoins.log` de la garde du push** nomme la lib courante `0c41f9fe6`. Ce lot est du
C++ du moteur : sa passe s'est bâtie et jouée sur son propre arbre (`rag3db-colonnes/build/moteur`)
et n'a lié aucune lib du magasin. La garde exige pourtant un journal qui nomme une lib du magasin ;
la seule du moment est `0c41f9fe6` (c7, 14 nov.).

## Ouverts par ce lot ou à côté

- Le SIGSEGV du dictionnaire d'une colonne de chaînes, au tour 6 de `DdlEveryMinute` (core
  3980144) : `docs/coeur-cpp/tickets/2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire-d-une-colonne-de-chaines.md`.
  Le témoin réduit de hotfixs est vert sous ASAN ; c'est mon lot n° 1 ensuite.
- L'ADD COLUMN qui passe sous une transaction d'écriture ouverte (ticket de hotfixs, Pour : cœur).
- Le planificateur qui balaie toute la table des arêtes sous un prédicat d'arête (ticket d'ED).
