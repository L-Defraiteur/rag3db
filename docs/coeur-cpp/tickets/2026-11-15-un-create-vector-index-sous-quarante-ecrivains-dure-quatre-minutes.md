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

## La bissection (15 novembre)

Sur les 67 commits du moteur (`src/`, `extension/vector/`, `test/transaction/concurrence/`) entre
`8ed386ca2` et `a2f9c9b65`, un tour du témoin par pas, à luciepc sous `poste lourd`, charge 6.
Seuil : bon sous 150 s. Le bout ancien, `8ed386ca2` lui-même, fait aujourd'hui **64 s** (et non les
13 à 15 s de sa ligne known_red) : une part de l'écart vient de l'environnement du jour.

| commit | date | CREATE | instruction la plus longue |
|---|---|---|---|
| `8ed386ca2` | 19 oct. | 64 s | 58 s |
| `c4e213ca5` | 8 nov. | 216 s | 210 s |
| `34538a328` | 25 oct. | 212 s | 206 s |
| `e526cc4b6` | 7 nov. | 174 s | 167 s |
| `1636f6607` | 25 oct. | 151 s | 142 s |
| `36542684f` | 25 oct. | **36 s** | 29 s |
| `f42d90d13` | 25 oct. | **165 s** | 158 s |

(L'ordre des commits est celui de `git rev-list --reverse` sur ces chemins, pas celui des dates.)

**Le premier commit lent : `f42d90d13`**, « le CREATE_VECTOR_INDEX en deux transactions — T_A pose
l'entrée et un holder NOTED, l'attente des écrivains d'avant T_A, T_B bâtit puis passe l'index à
READY ». Son parent du moteur, `36542684f`, fait 36 s ; lui et tous les commits d'après dépassent
150 s. `1636f6607`, à 151 s, est au bord du seuil mais d'après `f42d90d13`.

## La piste (à mesurer)

Dès la validation de T_A, le holder NOTED est visible et chargé : `NodeTable::lockIndexForWrite`
fait prendre le verrou INDEX de la table **en exclusif** à chaque écrivain (le convoi E11), pendant
tout le bâti de T_B, alors qu'aucun d'eux n'écrit dans l'index (`receivesWrites()` faux tant qu'il
est NOTED). Avant `f42d90d13`, l'index n'était visible qu'à la validation : pas de convoi pendant le
bâti. Ce qui reste à mesurer : où l'écrivain le plus long attend (le verrou INDEX, le mutex de
validation pendant le rattrapage de T_B, ou l'attente de T_B). La forme probable du correctif : un
holder qui ne reçoit pas d'écritures ne fait pas prendre le verrou INDEX.

## Pour fermer

Le témoin revient au niveau de sa ligne known_red (un CREATE de l'ordre de 15 s), ou plus bas.
