# Un CREATE_VECTOR_INDEX sous quarante écrivains dure quatre minutes, et un écrivain l'attend tout du long

- **État** : ouvert (15 novembre 2026, cœur c++)
- **Famille** : moteur
- **Gravité** : régression de performance (un ordre de grandeur), aucune donnée perdue
- **Atteignable en service** : oui, pour un premier index vectoriel posé pendant que des écrivains
  écrivent la table
- **Touche rag3weaver** : par le moteur (un écrivain bloqué plusieurs minutes)
- **Signalé par** : cœur c++. Départage du lot du verrou de schéma (`alter-attend-les-ecrivains`).
- **Pour** : cœur c++

## Ce qu'on voit

`VectorIndexCreateUnderManyWriters.NoWriterWaitsLongDuringTheCreate` (concurrence_test). Sa ligne
known_red dit, au 19 octobre : un CREATE de 13 à 15 s sous quarante écrivains, une instruction qui
attend 9 à 11 s (borne 5 s). Joué à luciepc le 15 novembre (bâti Release, build/moteur de
rag3db-colonnes, sous `poste lourd`), un tour par tête :

| tête | ce qu'elle porte | CREATE | l'instruction la plus longue |
|---|---|---|---|
| `a2f9c9b65` | master, avant le lot | 237 s | 231 s |
| `08f350e8e` | + verrou de table des écrivains et de l'ALTER | 177 s | 171 s |
| `3645ece32` | + verrou partagé du CREATE vectoriel | 233 s | 226 s |
| `ed2b6ffe6` | + refus du schéma changé (×3) | 229 à 260 s | 223 à 253 s |

Le témoin reste rouge pour la même raison sur master et dans le lot, mais à un niveau seize fois
plus haut que sa ligne. Le lot ne l'aggrave pas ; il n'en est pas la cause.

Dans `TwentyWriters.DdlEveryMinute`, quand le CREATE de la minute 4 passe (il est refusé pour pool
plein deux fois sur trois), il dure 270 s, et une instruction d'écrivain de Doc en dure 261.

## Ce qui reste à faire

Trouver le commit de master entre le 19 octobre et `a2f9c9b65` où le CREATE passe de 15 s à
quatre minutes, par bissection, une tête par tour de ce témoin (environ 5 min par tour).

## Pour fermer

Le témoin revient au niveau de sa ligne known_red (un CREATE de l'ordre de 15 s), ou plus bas.
