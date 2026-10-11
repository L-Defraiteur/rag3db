# Après le rejeu d'un COPY journalisé tué, la colonne DOUBLE des dernières lignes lit zéro

- **État** : ouvert (11 octobre 2026, 2 h 40, vu par la liste de fusion d'I1 sur luciepc). Bissecté
  par hotfixs : le défaut existe sans le lecteur affamé ; à l'arbre principal, avec l'étendue.
- **Gravité** : perte (une colonne à 0 sur les dernières lignes après rejeu + point de reprise,
  encore après réouverture).
- **Atteignable en service** : oui — c'est le chemin de reprise après un arrêt brutal pendant un
  `COPY` journalisé (condition 4 de la stèle : ne passe pas en probabiliste).
- **Touche rag3weaver** : oui (le `COPY` journalisé est l'écriture en masse du produit).

## Ce que c'est

`OwnerlessPagesTest.TheReplayReadsNothingBeyondTheExtent` (`test/transaction/journaled_copy_test.cpp`,
ligne 1936 ; témoin de `0aed3c4b5`, l'étendue connue du fichier de la base) : une table
`Doc(id INT64, name STRING, score DOUBLE, vec FLOAT[4])`, un `COPY` de 200 000 lignes journalisé
(`copy_journal_threshold` à 1 Gio) dans un enfant qui **finit son `COPY`** (validé au journal, sans
point de reprise : `auto_checkpoint=false`, `force_checkpoint_on_copy=false`) **puis se tue**
(`writeThenDie`, SIGKILL base ouverte), puis la base rouverte (rejeu), `CHECKPOINT`, et comparée à une base
témoin qui ne meurt pas. Une fois, sur luciepc, dans la liste de fusion d'I1
(`verrous-i1-4` = master `5c5feb527` + I1, bâti de 2 h 13) :

```
sum(d.score) : 19943907329.500000   attendu 20000000000.000000
ligne 199999 : name 199999|0.000000|[82.2,16.25,-8,7.5]   attendu name 199999|199999.500000|…
```

`count`, `sum(id)`, `name`, `vec` justes ; seule la colonne `score` (DOUBLE) des dernières lignes
est à zéro (56 092 670,5 manquants : les scores de ~336 lignes à la fin), pareil après
réouverture. Journal : luciepc `~/.cache/rag3db-moteur-notes/i1/liste-fusion/transaction_test.log`.

## Ce qu'on sait

- Seul, ×2 (`--gtest_filter='OwnerlessPagesTest.*'`, 9 cas) : vert ; la suite `transaction_test`
  entière rejouée une fois sur le même bâti (2 h 44, en lourd) : 247 verts. Vert aussi dans la liste
  précédente d'I1 (base `81d5c4823`, qui porte déjà `0aed3c4b5`) et dans la liste complète du
  lecteur affamé d'hotfixs (`f8afe7202`), et une fois dans la liste de son lot suivant. **Probabiliste**,
  mais pas par l'instant de la mort (le fils finit son `COPY` avant de se tuer, remarque d'hotfixs) :
  le hasard vient d'ailleurs — l'ordre d'écriture des groupes par les fils du `COPY`, ou l'étendue
  rendue à l'ouverture.
- **Pas I1** : hors du mode multi-écrivains (`usesLocks` faux ici), I1 ne change rien.
- Deux candidats :
  1. `e4198f862` (hotfixs, le lecteur affamé) : un verrou de fichier `<base>.readers` autour de
     `replayShadowPageRecords`, `removeWALAndShadowFiles`, `truncateWALFile` quand il coupe, et
     au point de reprise autour de `logAndFlushCheckpoint` — « rien ne change dans l'ordre ni dans
     ce qui est écrit » ; sa passe finale ciblée ne jouait pas ce cas ;
  2. `0aed3c4b5` (l'étendue, arbre principal) : un état de rejeu non couvert — une page de la
     colonne DOUBLE du dernier groupe rendue à l'étendue puis écrasée par le point de reprise ?

- Lecture de l'arbre principal (auteur de l'étendue, 3 h, sans reproduire) : une seule colonne fausse
  sur la dernière ligne ressemble à une page de la colonne `score` du dernier groupe **rendue** (au-delà
  de l'étendue) puis réécrite avant que le rejeu ne la relise — ou l'inverse, lue par le rejeu avant que
  le `CHECKPOINT` ne la réécrive. L'étendue est posée à chaque point de reprise ; si `e4198f862` change
  l'ordre entre le rejeu du journal et le premier point de reprise (le lecteur affamé passe par le
  checkpointer), une page allouée pendant le rejeu peut tomber au-delà d'une étendue lue trop tôt :
  **regarder d'abord si l'étendue de l'en-tête est relue après le rejeu ou figée à l'ouverture.**

## Témoin

Le cas lui-même ; en boucle : `transaction_test --gtest_filter='OwnerlessPagesTest.*' --gtest_repeat=10`.
Bissection d'hotfixs sur luciepc (en lourd, chaînée) : le parent `61bacdd05` (sans `e4198f862`) ×30 ;
master `80054b530` ×10 puis ×30 — entre les deux, seul `e4198f862` touche `src/`. Il écrit lui-même
les comptes et l'imputation ici.

## La bissection (hotfixs, 11 octobre, 2 h 46 à 3 h 25, luciepc, Release, en lourd, chaînée)

`--gtest_filter=OwnerlessPagesTest.TheReplayReadsNothingBeyondTheExtent`, un bâti par côté
(`rag3db-bisect` à `61bacdd05`, `rag3db-tickets` à `80054b530`) :

| côté | `e4198f862` | passes | rouges |
|---|---|---|---|
| `61bacdd05`, le parent du lecteur affamé | non | 30 | **1** |
| `80054b530`, master | oui | 10 + 30 | **3** (0 sur les 10, 3 sur les 30) |

- **Imputation : pas le lecteur affamé.** Le rouge existe sans `e4198f862` (1 sur 30 au parent),
  et entre les deux côtés seul `e4198f862` touche `src/`. L'écart 1/30 contre 3/40 n'est pas
  significatif. Le lot n'est pas défait.
- **Le test est né avec `0aed3c4b5`** (`git log -S`) : aucune bissection ne remonte avant lui. Le
  défaut peut être plus ancien que l'étendue, ou venir d'elle. C'est la lecture de l'étendue qui
  reste en tête, à l'arbre principal (orchestration).
- **Les formes du rouge** (journaux luciepc `~/git_workspaces/bisect-parent.log`,
  `bisect-master.log`) : toujours la seule colonne `score`, et toujours sur les dernières lignes ;
  `count`, `sum(id)`, les noms et les vecteurs sont justes. `sum(score)` vaut 19 988 006 066 au
  parent ; 19 974 232 959,5, 19 986 208 068,5 et 19 999 800 033,5 sur master, au lieu de
  20 000 000 000. Le manque varie d'une passe à l'autre : de 1 à environ 130 lignes à 0, au prix d'une ligne de fin (environ 200 000 chacune). Les
  rouges tombent tous à « after the replay and a checkpoint » (ligne 1936), puis à la réouverture
  (ligne 1941), et la ligne 199999 n'est fausse que dans une partie des cas.
- **Le taux** : environ 1 sur 20 sur ce bâti (4 rouges sur 70 passes).
- **Imputé ensuite au COPY journalisé** par l'arbre principal (11 octobre, nuit) : ni l'étendue ni
  le lecteur affamé. Dans un processus vivant, sans mort ni rejeu, un COPY journalisé avec deux
  fils et une colonne tableau met à 0 la colonne DOUBLE des dernières lignes (3 fois sur 3). Le
  chemin vient de `0f4a54b2c`. Voir le ticket de la session cœur C++ (stèle), qui le prend.

## Pour le fermer

Imputer par la bissection ; si c'est le lot du lecteur affamé, corriger ou défaire avant tout
autre push de C++ (orchestration, 11 octobre, 2 h 45) ; si c'est l'étendue, à l'arbre principal.
Puis le cas vert en boucle (×10 au moins) sur le bâti corrigé.
