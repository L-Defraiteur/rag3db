# A7, la file des points de reprise — et ce que la première forme d'A8 a appris

Pour la session cœur C++ (relecture) et l'orchestration. Seconde session cœur C++, 11 octobre
2026. Définitions : `3-octobre-2026-23h31/coeur-cpp/11-condition-3-les-ecritures-paralleles.md`
§1, ligne A7 du tableau et paragraphe A8. Les marches A7 et A8 sont confiées par la session cœur
C++, qui a relu l'esquisse avant le code.

## A7 — deux validations qui veulent un point de reprise

**Le défaut.** `TransactionManager::stopNewTransactionsAndWaitForOthersNoLock` : « un seul point
de reprise attend à la fois ». La première validation forcée pose `checkpointPending` et attend
que les autres transactions partent. La seconde, encore active, attend que `checkpointPending`
retombe : chacune attend l'autre, et la première expire au bout de 5 s.

**La forme : une file.**

- Une validation qui veut un point de reprise et en trouve un en attente **se range dans la file**
  (`commitInCheckpointQueueNoLock`), **sans rien valider encore**.
- Le point de reprise en attente ne compte pas les validations de la file parmi les transactions à
  attendre. Quand il ne reste qu'elles, il pose `commitQueuedNow`, et elles valident en mémoire.
  Chacune prend son numéro sous le verrou public, la validation faite, puis attend un point de
  reprise **commencé après** ce numéro. Il ne reprend la main qu'une fois la file vide, puis il
  écrit.
- S'il **expire** (une autre transaction n'est pas partie), les validations de la file n'ont rien
  validé : la garantie d'aujourd'hui, « délai expiré → rien de validé », tient. Elles reprennent
  le chemin ordinaire, et la première redevient la tête de la file. S'il **échoue**
  (`checkpointFailed`), elles lèvent la même erreur.
- Si la validation qui portait le point de reprise échoue après l'attente, celles de la file l'ont
  déjà validé en mémoire ; elles écrivent alors le point de reprise elles-mêmes, et, si celui-ci
  ne peut pas s'écrire, le disent : « committed in memory, but the checkpoint that makes it durable
  could not be written ». Il n'y a jamais deux points de reprise à la fois. Une validation forcée
  n'écrit rien au journal : il n'y a pas d'ordre de journal à tenir.

**Le témoin**, au banc : `Copies/TwoCommitsThatWantACheckpoint.DoNotWaitForTheTimeout/Forced`.
Rouge sur master (5 015 ms, délai expiré, la seconde validation perdue), vert après (117 ms), il
sort de `known_red`. `transaction_test`, partie ciblée : 96 sur 96.

## A8 — ce que la première forme a appris (le lot suivant)

- **Attendre dans une validation, borné, le départ des autres** (la première forme) **ne marche
  pas** : un COMMIT s'exécute sur un fil du planificateur de tâches (deux fils au banc). La
  validation qui attend en tient un ; les COMMIT des autres attendent un fil libre, leurs
  transactions restent ouvertes, l'attente expire, la suivante recommence. Avec 8 écrivains, deux
  validations par seconde, et le garde du banc tombe. Pile relevée par gdb.
- **La forme retenue** (relue par le cœur) : la validation qui déclenche pose `checkpointPending`
  et l'heure de la demande, puis rend la main. **Le dernier à partir écrit le point de reprise**
  (`clearTransactionNoLock`, sous le verrou public). Un BEGIN retenu plus d'1 s annule la demande,
  et le report est nommé. Il faut tenir trois choses :
  - le dernier partant n'écrit pas s'il part par la fermeture d'une connexion ou de la base ;
  - si son point de reprise échoue, l'erreur rendue à sa validation dit que la transaction est
    validée au journal et que c'est le point de reprise d'après qui a échoué ;
  - les lecteurs aussi attendent au BEGIN, 1 s au plus : c'est l'écart avec PostgreSQL.
- **Le témoin** : 8 écrivains sans pause, qui enchaînent BEGIN, écriture et COMMIT. Deux jets
  écartés :
  - le premier (4 écrivains avec une pause) laissait des trous où le point de reprise se
    glissait, et il était vert sur master ;
  - le deuxième faisait valider un écrivain seulement si un autre avait sa transaction ouverte. Il
    était insatisfiable par construction, puisque le point calme y est impossible.

  Le jet sans pause montait sur master à 226 Kio pour un seuil de 64 Kio, sous son plafond de
  256 Kio : son plafond se fixera sur plusieurs passes. Il est gardé dans
  `~/.cache/rag3db-tickets-notes/a8-temoin-sans-pause.cpp`.
