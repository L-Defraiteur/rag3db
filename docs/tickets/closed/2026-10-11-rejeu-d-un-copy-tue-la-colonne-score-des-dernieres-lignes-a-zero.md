# Après le rejeu d'un COPY journalisé tué, la colonne DOUBLE des dernières lignes lit zéro

- **État** : corrigé `eae3863ab` (11 octobre 2026, 4 h 10 ; ouvert le même jour à 2 h 40, vu par la
  liste de fusion d'I1 sur luciepc ; bissecté par hotfixs : sans le lecteur affamé ; reproduit sans
  mort par l'arbre principal ; cause trouvée par le cœur C++). Ce n'est ni l'étendue ni le rejeu :
  un défaut de lecture de l'amont, révélé par le COPY journalisé.
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

## La cause, et la fermeture (cœur C++, 11 octobre 2026, 3 h 30 à 4 h 10)

Reproduit sans mort ni rejeu par la sonde de l'arbre principal (`variantes.cpp`, 200 000 lignes,
deux fils, `FLOAT[4]` à côté d'une `DOUBLE`, journalisé) : 3 sur 4 sur luciepc, 69 à 252 scores
faux, stables dans un processus. Preuves : une relecture **par la clé** rend la bonne valeur là où
le balayage lit 0 ; une sonde dans `ColumnChunk::scan` montre que les zéros sortent du **groupe
plein** écrit sur disque pendant le COPY, découpé en **deux segments** (99 840 + 31 232 valeurs),
à partir de la première valeur du second ; la sélection du balayage y est **filtrée** (les
versions de l'insertion du COPY, tant qu'aucun point de reprise ne les a effacées :
`VectorVersionInfo::getSelVectorForScan`, `CHECK_VERSION`) ; et le second segment porte des
**exceptions ALP** (les valeurs que l'encodage ne représente pas, stockées à part) aux positions
0..N-1, N = exactement le nombre de lignes fausses.

Le défaut : `patchFloatExceptions` (`src/storage/table/column_reader_writer.cpp`, code de l'amont,
mai 2025) passait au filtre de sélection un index **absolu** dans le vecteur de sortie
(`startOffsetInResult + position`) là où `DefaultColumnReadWriter::readCompressedValues` lui
donne un index **relatif** au début du balayage : pour tout segment lu après le premier d'un même
bloc, aucune exception n'était rapiécée, et les valeurs restaient à leur encodage ALP, 0.
`logInsertedRowsToWAL` balaye pareil : le journal portait ces zéros, le rejeu les écrivait, le
point de reprise les gardait ; un `CHECKPOINT` **avant** la lecture fusionne les segments et efface
les versions, d'où « juste après checkpoint ». Un fil, `vec` en `STRING`, le point de reprise
forcé : le groupe n'a pas cette forme. Rien vers l'amont (règle de Lucie).

Correctif : l'index relatif au filtre, une ligne. Témoin déterministe :
`JournaledCopyReadBackTest.TwoThreadsAndAnArrayColumnReadEveryScoreBack` (la sonde en gtest, trois
COPY de suite, la configuration de la sonde : tampon de ~73 Mio, deux fils), rouge d'abord ; le
témoin de ce ticket rejoué ×20 vert sur le bâti corrigé.
