# Un CREATE REL TABLE retient parfois un écrivain au-delà de sa durée

- **État** : ouvert (16 novembre 2026, cœur c++)
- **Famille** : moteur
- **Gravité** : lenteur (un écrivain attend quelques centaines de millisecondes de plus que le DDL)
- **Atteignable en service** : oui, pour un CREATE REL TABLE posé sous des écrivains
- **Touche rag3weaver** : par le moteur
- **Signalé par** : cœur c++. Les ×10 de `TwentyWriters.DdlEveryMinute` sur le correctif du pool
  (`ec7a0bf30`), et les tours du 15 novembre.
- **Pour** : cœur c++

## Ce qu'on voit

Le contrôle `create-rel-table-holds-writers-at-most-its-duration` du scénario des vingt écrivains
borne l'attente d'un écrivain qui chevauche le CREATE REL TABLE de la minute 3 : au plus la durée
du DDL, plus 50 ms, au-delà de son p99 hors DDL. Il rougit dans 5 des 50 journaux de
`DdlEveryMinute` du 15 novembre, sur master comme dans les lots, à luciepc
(`~/.cache/rag3db-moteur-notes/*/ddl-*.log`) :

| journal | DDL | l'écrivain le plus long | p99 hors DDL |
|---|---|---|---|
| alter7/ddl-1 | 48 ms | 325 ms | 220 ms |
| ddl10-fix/ddl-2 | 33 ms | 255 ms | 167 ms |
| ddl10-fix/ddl-9 | 40 ms | 254 ms | 154 ms |
| sonde-diag/ddl-3 | 609 ms | 1 861 ms | 514 ms |
| apres/ddl-7 (pool corrigé) | 636 ms | 838 ms | 34 ms |

Les trois premiers dépassent de quelques dizaines de millisecondes, sous un pool saturé et un p99
élevé. Les deux derniers dépassent de 120 ms à 700 ms, dont un sur le correctif du pool, où le reste
du scénario est vert. Les écrivains de Note, d'une table que le DDL ne touche pas, attendent autant
que ceux de Doc (830 ms, apres/ddl-7).

## Ce qui reste à faire

Lire où attend l'écrivain : un CREATE REL TABLE ne prend pas le verrou de schéma des tables qu'il
relie. Un écrivain de Note qui attend autant suggère un verrou global : le mutex des appels publics
du gestionnaire de transactions, ou le point de reprise que le DDL déclenche. À mesurer par eu-stack
échantillonné sur un tour qui rougit.

## Pour fermer

La cause lue, le correctif, et `DdlEveryMinute` ×10 sans ce contrôle rouge.
