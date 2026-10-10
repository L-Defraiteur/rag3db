# Un lecteur en lecture seule est refusé tant que les points de reprise d'un autre processus se suivent

- **État** : corrigé `3498ece5d` (11 octobre 2026, seconde session cœur C++) — le verrou des lecteurs ; témoin moteur nu `ReadOnlyReader.SingleOpenIsNotRefusedByBackToBackCheckpoints` (`f85497354`), rouge avant (34 refus sur 80), vert après
- **Gravité** : blocage (une ouverture refusée ; rien n'est perdu ni lu faux)
- **Atteignable en service** : oui, sous un écrivain qui fait beaucoup de points de reprise
- **Touche rag3weaver** : oui (`Rag3dbConnection::read_only`, et le test
  `e2e_prise_atomique::un_lecteur_qui_insiste_pendant_qu_on_ecrit`, rouge 1 à 3 fois sur
  80 selon la session de l'arbre principal)

## Ce que c'est

Une ouverture en lecture seule que traverse un point de reprise d'un autre processus est
refusée par son nom, `WALReplayer::CHECKPOINT_CROSSED_READ_ONLY_OPEN` (d6c1a541a) :

```
Runtime exception: The database was checkpointed by another process while this read-only
open was reading it: what it read may mix the state before and after that checkpoint, so
the open is refused rather than served. Retry the open.
```

Le refus est voulu (il remplace une lecture qui mélangerait l'avant et l'après). Mais
quand les points de reprise se suivent assez vite, chaque nouvelle tentative est
traversée à son tour : le lecteur est affamé, et la patience de l'appelant (250 ms pour
rag3weaver) ne suffit plus.

## Recette minimale

Deux processus. Le premier écrit sans relâche : `CREATE (:Work {id: i, …})`, puis
`CHECKPOINT` toutes les cinq écritures. Le second ouvre la base en lecture seule et
compte `MATCH (t:Work) RETURN count(t)`, quatre-vingts fois, avec la patience de
`read_only_patient` (5, 10, 20, 40, 40… ms, budget de 250 ms).

## Témoin

`test/transaction/concurrence/read_only_reader_test.cpp`,
`ReadOnlyReader.InsistingReaderIsNotStarvedByCheckpoints`, dans `probabilistic.txt`.
Étiquette `reader-not-starved` ; le message relève chaque refus mot pour mot. Le rythme
de l'écrivain se règle par `RO_WRITE_PAUSE_US` (pause entre deux écritures, 200 µs par
défaut).

## Les taux mesurés (référence pour la marche 5)

Release, base sur tmpfs, 4 octobre 2026. Un refus est une ouverture qui reste refusée au
bout des 250 ms.

Master (après 57c8389b4 et df689b522), 5 passes de 80 ouvertures par rythme :

| pause entre écritures | écritures pendant la passe | refus sur 80, par passe |
|---|---|---|
| 1 ms (le rythme du test rag3weaver) | 550 à 835 | 0, 0, 0, 0, 0 |
| 0,5 ms | 1 950 à 3 370 | 1, 1, 1, 0, 0 |
| 0,2 ms | 4 000 à 10 000 | 3, 1, 6, 3, 14 |
| aucune | 24 000 à 32 000 | 30 à 55 |

Master contre l'ancêtre df689b522^ (avant la garde de double ouverture et les remèdes
du point de reprise), 20 passes alternées de chaque côté :

| pause | côté | points de reprise par seconde | refus sur 1 600 | passes avec refus | refus pour 1 000 points de reprise |
|---|---|---|---|---|---|
| 0,5 ms | ancêtre | 276 | 0 | 0 | 0,00 |
| 0,5 ms | master | 278 | 2 | 2 | 0,56 |
| 0,2 ms | ancêtre | 474 | 35 | 17 | 2,14 |
| 0,2 ms | master | 471 | 53 | 17 | 2,72 |

L'écart entre les deux côtés est d'environ deux écarts types : il n'est pas établi, et
ne changerait pas la décision. Le refus existait avant ces commits.

## Cause

Le lecteur s'assure qu'aucun point de reprise n'a traversé son ouverture en comparant
l'état des fichiers avant et après (`ReadOnlyOpenIdentity`). Toute traversée oblige à
recommencer depuis le début ; rien ne lui permet de lire un état stable pendant qu'un
nouveau point de reprise s'écrit.

## Pour le fermer

La marche 5 : une époque écrite dans le fichier, pour qu'un lecteur lise un état stable
sans avoir à recommencer à chaque point de reprise. Elle va avec les écritures
parallèles. Le témoin doit alors passer sous la ligne de 0,2 ms (et idéalement sans
pause) sans refus.

En attendant, côté rag3weaver : reprendre l'ouverture en lecture seule sur ce refus
nommé. Le seuil « 0 refus » du test `e2e_prise_atomique` ne se relâche pas (décision de
l'orchestration) ; c'est confié à la session de l'arbre principal.

## Deux messages pour le même croisement (5 octobre 2026)

Le moteur refuse une ouverture en lecture seule croisée par un point de reprise sous deux
messages :

- « … was checkpointed by another process while this read-only open was reading » :
  le point de reprise a traversé l'ouverture ;
- « Couldn't replay shadow pages under read-only mode » : l'ouverture arrive pendant
  que l'écrivain est au milieu de son point de reprise.

Le second partait sur la borne de 250 ms des autres erreurs. Le test du lecteur affamé
échouait alors une passe sur quatre, avec un refus non nommé sur une centaine. Côté
rag3weaver, `rag3db_connection::is_checkpoint_crossing` reconnaît les deux messages, et
une seule constante tient chacun d'eux. Les deux sont repris sous la même borne de 2 s.
Le refus final garde le message du moteur et le compte des reprises : un écrivain mort
en plein point de reprise rendrait le second message pour de bon (décision de
l'orchestration).

À faire côté moteur, avec la marche 5 et pas avant : un seul message pour ce croisement.

## Corrigé (`3498ece5d`) : le point de reprise cède aux ouvertures (11 octobre 2026)

Forme choisie par l'orchestration, à la manière de SQLite en mode WAL : le point de reprise
cède au lecteur au lieu de le refuser. Le chemin est un verrou des lecteurs, posé sur un
fichier à côté de la base (`<base>.readers`, `ReadersLock`). Il passe par flock, ou par
LockFileEx sous Windows, jamais par fcntl : un verrou fcntl appartient au processus et ne
séparerait pas deux ouvertures d'un même processus.

- **L'ouverture en lecture seule** le tient partagé, de la capture de l'en-tête et du journal
  jusqu'au rejeu en mémoire. Si un point de reprise est en cours, elle l'attend, **5 s au
  plus**. Au-delà, elle est refusée, et le refus le dit : « This read-only open waited N ms for
  a checkpoint of another process to finish… ».
- **Le point de reprise** le prend exclusif sur la seule fenêtre où le fichier change pour un
  lecteur. Cette fenêtre va de la marque CHECKPOINT à la suppression du journal ; avant elle, il
  n'écrit rien que l'en-tête courant désigne (avis de la session cœur C++). À la reprise d'un
  écrivain, il prend l'exclusif autour du rejeu des pages fantômes et de la troncature du
  journal. Il attend les ouvertures en cours **1 s au plus**, puis passe, et l'écrit sur la
  sortie d'erreur (« … waited N ms for read-only openings of the database and went ahead
  without them »). Il ne peut pas dire combien d'ouvertures il a attendues : flock ne compte pas
  ceux qui tiennent le verrou.
- **Le constat d'après coup** (`CHECKPOINT_CROSSED_READ_ONLY_OPEN`) reste le filet. Il joue pour
  une ouverture plus longue que la borne de l'écrivain (un gros journal à rejouer, un lecteur
  bloqué), pour une base ouverte en lecture seule dans le processus de l'écrivain au-delà
  d'1 s, et pour un dossier où le fichier du verrou ne peut pas exister. Son message dit
  maintenant la borne de 1 s.
- **Ce que le verrou ne couvre pas** : les lecteurs restés ouverts pendant qu'un point de
  reprise passe. Ils restent l'affaire de la marche 5 (une époque écrite dans le fichier), avec
  le message unique pour ce croisement.

Témoins :

- `ReadOnlyReader.SingleOpenIsNotRefusedByBackToBackCheckpoints`, le cas moteur nu : un
  écrivain sans pause, et 80 ouvertures d'un seul essai, sans la patience de l'appelant. Rouge
  sur master avec 34 refus sur 80 (25 « checkpointed by another process », 9 « Couldn't replay
  shadow pages ») ; vert après, 0 sur 80.
- `ReadOnlyReader.InsistingReaderIsNotStarvedByCheckpoints` : 0 refus à 0,2 ms et sans pause.
- Les bornes, dans `ReadOnlyOpenTest` :
  - `ReadOnlyOpenWaitsForACheckpointInProgress` : l'ouverture attend 300 ms, puis s'ouvre ;
  - `ReadOnlyOpenWaitIsBoundedAndNamed` : le refus nommé arrive à 5 s ;
  - `CheckpointWaitsForAReadOnlyOpenThenGoesAhead` : le point de reprise passe à 1 s.

Côté rag3weaver, la reprise des deux messages sous la borne de 2 s devient un filet ; elle peut
rester.
