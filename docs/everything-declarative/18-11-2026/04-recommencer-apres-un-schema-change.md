# Recommencer une écriture refusée par un ALTER validé : où se recommence-t-elle ?

19 novembre 2026, session ED. Entrée 2.7 de ma page du 16, ticket
`2026-11-15-recommencer-une-ecriture-refusee-par-un-alter-valide.md`. Le blocage par la lib est levé : la lib
épinglée 7c2b506ec porte le lot du cœur 101ff50a6, qui refuse par « Schema changed, retry: »
(`Transaction::SCHEMA_CHANGED_RETRY`). **Avant de coder, une question de conception (n° 1).**

## Ce que le ticket prévoyait, et pourquoi ça ne tient pas tel quel

Le ticket prévoyait que le paquet entier se recommence, sur place, dans la synchronisation d'instantané. Or :

- **un ROLLBACK empoisonne le catalogue** (`code_sync::terminer`, qui appelle `Catalog::poison`) : ses caches
  (identifiants de nœuds, file des liens, index plein texte ouverts, preuve de présence) gardent les écritures
  défaites, et seule une réouverture les remet d'accord avec la base. Rejouer le paquet sur ce catalogue
  risquerait des liens doublés ou perdus, le défaut même que le ticket veut éviter ;
- **le refus ne tombe pas forcément à la première instruction** : le moteur le juge à la première prise de verrou
  de **chaque** table (`node_table.cpp:235`, `rel_table.cpp:59`). Un paquet écrit `File`, `Scope`, les morceaux et
  les relations : le refus peut venir après deux tables déjà écrites dans la transaction.

## Ce que je propose : la reprise qui existe déjà

La reprise d'un index **après un paquet défait** existe et elle est éprouvée (`e2e_tx_par_paquet_arret`, et le
correctif du 18 novembre `first_pass_interrupted`) : le catalogue est rouvert, la session d'instantané reprise, le
paquet refait, et rien n'est doublé.

- **(i) Le grain de la transaction par paquet.** `terminer` reconnaît le refus (`contains(SCHEMA_CHANGED_RETRY)`)
  et rend une erreur nommée, distincte des autres. Le **lanceur de l'index** (`spawn_index_with_stop` et son appelant
  dans l'hôte) rouvre alors le backend, comme après tout paquet défait, puis relance l'index, au plus 3 fois. La fin
  du journal dit « reprise après un schéma changé (n sur 3) ». Après 3 reprises, la fin est un échec nommé, et le
  journal dit la table.
- **(ii) Le grain d'une instruction seule** : hors transaction explicite, l'écrivain des générations et une
  resynchronisation. Chaque instruction est sa propre transaction, annulée par le moteur. La connexion peut alors la
  recommencer elle-même, au plus 3 fois, si elle sait qu'aucune transaction explicite n'est ouverte
  (`Catalog::set_in_transaction` le sait déjà). À vérifier avec le cœur : une instruction refusée n'a-t-elle rien
  laissé, même dans les caches du catalogue (une ingestion à plusieurs COPY est plusieurs instructions) ?

**Le témoin, rouge d'abord** : un `ADD COLUMN` validé sur une autre connexion entre l'ouverture d'un paquet et sa
première écriture. C'est un crochet de test porté par le catalogue, comme les fautes injectées. L'index va au bout,
aucune ligne n'est perdue ni doublée (les comptes et les empreintes d'`e2e_tx_par_paquet_arret`), et le journal dit
une reprise.

## La question (n° 1, pour c7, et le cœur pour (ii))

Retiens-tu (i), la reprise par la réouverture et le lanceur de l'index, au lieu d'un rejeu sur place que
l'empoisonnement interdit ? Et pour (ii), le cœur confirme-t-il qu'une instruction refusée ne laisse rien ?
En attendant la réponse, je passe à l'entrée suivante de ma liste. Le worktree `wt-retry` reste prêt.

## Retenu par c7 : (i). Le relevé du chemin (19 novembre, après la réponse)

**Ce qui existe** :

- `terminer` empoisonne le catalogue sur un ROLLBACK ;
- l'hôte, après **un appel** dont le backend dit `must_reopen`, lâche ce backend : il arrête ses réactions et ses
  indexations (`stop_indexings_of`), puis le prochain appel le rouvre (`daemon_host.rs`, autour de la ligne 732) ;
- un manifeste peut déclarer `catch_up_on_open`, et la réouverture rattrape alors la source.

**Ce qui manque pour (i)** : le fil de l'index ne fait pas d'appel. Un paquet refusé pour schéma changé laisse
donc un catalogue empoisonné que personne ne lâche ni ne rouvre, jusqu'au prochain appel d'outil. Et l'index n'est
relancé que si le manifeste déclare `catch_up_on_open`.

**Le chemin que je prends** :

1. `terminer` reconnaît le refus et rend une erreur qui commence par un préfixe nommé, `SCHEMA_CHANGED_PREFIX`,
   comme `SYNC_SUSPENDED`.
2. La fin de l'index la dit (« reprise demandée : schéma changé sur T »), et l'index s'inscrit **à reprendre** :
   le nombre d'essais est noté en méta, au plus 3.
3. L'hôte, au registre des fils (`catalog_threads`), voit un index fini sur ce préfixe. Il lâche le backend et le
   rouvre comme après un appel, puis relance l'index sur la même source. Au 3e essai, la fin est un échec nommé, et
   l'index n'est plus relancé.

**Le témoin**, rouge d'abord : un `ADD COLUMN` validé sur une autre connexion entre l'ouverture d'un paquet et sa
première écriture, par un crochet du catalogue du test. L'index va au bout, aucune ligne n'est perdue ni doublée,
et le journal dit une reprise. C'est un e2e par l'hôte, puisque la réouverture est à lui.

**Taille** : un lot qui touche l'hôte (le registre des fils et la réouverture), environ une journée. (ii) attend la
réponse du cœur.

## (ii) confirmé par le cœur (19 novembre)

« Une instruction seule en autocommit, refusée par "Schema changed, retry:", ne laisse rien, et la même instruction
rejouée telle quelle va au bout. » Voici pourquoi :

- hors mode MANUAL, `TransactionContext::rollbackAfterStatementFailure` annule toute la transaction automatique
  (`transaction_context.cpp:115`) : le stockage local, le tampon d'annulation et les verrous sont rendus ;
- rien n'atteint le WAL avant la validation ;
- un CREATE sur plusieurs tables ou un COPY s'annule entier ;
- la relecture de l'instruction se lie sur le nouveau schéma.

Le plafond de 3 essais lui paraît juste. Deux réserves :

1. Un COPY au point de reprise forcé, annulé, peut perdre de la place : des pages allouées, pas des lignes
   (`copy_test.cpp:144`).
2. Dans une transaction explicite, la transaction reste ouverte après le refus, avec ses verrous : à défaire par
   ROLLBACK avant de recommencer. C'est la voie (i).

**À venir sur sa branche `drop-waits-for-readers`** : un ADD ou un DROP de colonne attendra aussi les lecteurs
(5 s au plus), et son refus nommera le lecteur ou l'écrivain qui le tient. Un lecteur ne sera plus refusé par un
ADD.

**Donc (ii)** : la connexion (`Rag3dbConnection`) recommence elle-même, au plus 3 fois, une instruction refusée pour
schéma changé **quand aucune transaction explicite n'est ouverte**. Le catalogue ne voit qu'un succès, et ses caches
restent d'accord avec la base. Dans une transaction explicite, l'erreur remonte telle quelle vers (i).
