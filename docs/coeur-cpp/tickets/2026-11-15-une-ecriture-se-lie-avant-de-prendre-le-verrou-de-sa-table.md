# Une écriture se lie avant de prendre le verrou de sa table : un ALTER attendu la fait refuser

- **État** : ouvert (15 novembre 2026, cœur c++)
- **Famille** : moteur
- **Gravité** : gêne (un refus nommé, à relancer ; aucune donnée perdue ni mal écrite)
- **Atteignable en service** : oui, rarement. Il faut un écrivain qui écrit une table pendant
  qu'un ALTER ou un DROP TABLE l'attend, ou qui a commencé sa transaction avant que l'un d'eux
  valide. En service, l'hôte ne fait un ALTER qu'à l'ouverture ou à une montée de schéma.
- **Touche rag3weaver** : par le moteur. Un écrivain reçoit « The schema of table <T> changed
  after this transaction started … ; retry the transaction. » et doit recommencer.
- **Signalé par** : cœur c++. Le lot du verrou de schéma (branche `alter-attend-les-ecrivains`),
  témoins `AlterWaitsForWriters.AWriterCreatingDuringTheAlterWritesNoRowOfTheOldSchema` et
  `AddColumnRollback.TwoAltersOfOneTableDoNotOverlap`.
- **Pour** : cœur c++

## Ce qu'on voit

Le verrou de schéma d'une table (genre TABLE du gestionnaire de verrous) se prend à l'exécution,
à la première écriture de la transaction dans la table : `NodeTable::insert` et ses voisins,
`Alter::executeInternal`, `Drop::dropTable`. La requête est déjà liée et planifiée sur le
catalogue de l'instantané. Un écrivain qui a attendu un ALTER écrirait donc sur le schéma
d'avant. Le même cas existe sans attente, pour une transaction ouverte avant un ALTER validé.

Le lot le ferme par un refus nommé, `Transaction::throwIfSchemaChangedAfterSnapshot`. À la
première prise du verrou d'une table, la transaction est refusée si la tête de l'entrée du
catalogue est une version que son instantané ne voit pas. Elle recommence, et se lie sur le
nouveau schéma. Deux ALTER d'une même table suivent la même règle : le second attend le premier,
est refusé, puis passe quand on le relance.

## Ce que fait PostgreSQL

PostgreSQL prend le verrou de chaque relation à l'analyse (`parserOpenTable`, puis
`AcquireRewriteLocks` et `AcquireExecutorLocks` au plan mis en cache), et il revalide un plan
gardé quand le catalogue change (`RevalidateCachedQuery`). Une requête qui attend un
`ACCESS EXCLUSIVE` se lie donc après lui, sur le nouveau schéma, sans erreur.

## Ce qu'il faudrait

Prendre le verrou de table en partagé au moment de la liaison (`Binder` : la table d'un
`CREATE`, d'un `SET`, d'un `DELETE`, d'un `MERGE` ou d'un `COPY`), dans la transaction de la
requête. Le lier une seconde fois si l'attente a vu le catalogue changer. Le refus nommé
d'aujourd'hui resterait le filet de la transaction ouverte avant l'ALTER : sous l'isolation par
instantané, celle-là ne peut pas voir le nouveau schéma.

## Pour fermer

`AWriterCreatingDuringTheAlterWritesNoRowOfTheOldSchema` passe sur le chemin « OK », sans refus,
pour un écrivain en validation automatique qui arrive pendant l'ALTER. `TwoAlters…` passe sans
relance.
