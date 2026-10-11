# Après le rejeu d'un COPY journalisé tué, la colonne DOUBLE des dernières lignes lit zéro

- **État** : ouvert (11 octobre 2026, 2 h 40, vu par la liste de fusion d'I1 sur luciepc ; hotfixs
  bissecte cette nuit).
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

## Témoin

Le cas lui-même ; en boucle : `transaction_test --gtest_filter='OwnerlessPagesTest.*' --gtest_repeat=10`.
Bissection d'hotfixs sur luciepc (en lourd, chaînée) : le parent `61bacdd05` (sans `e4198f862`) ×30 ;
master `80054b530` ×10 puis ×30 — entre les deux, seul `e4198f862` touche `src/`. Il écrit lui-même
les comptes et l'imputation ici.

## Pour le fermer

Imputer par la bissection ; si c'est le lot du lecteur affamé, corriger ou défaire avant tout
autre push de C++ (orchestration, 11 octobre, 2 h 45) ; si c'est l'étendue, à l'arbre principal.
Puis le cas vert en boucle (×10 au moins) sur le bâti corrigé.
