# Un témoin de temps du banc rougit sous charge

- **État** : ouvert (8 novembre 2026, à la demande de l'orchestration)
- **Famille** : moteur
- **Gravité** : test instable — un rouge sans défaut empêche le push d'un lot, ou apprend à
  passer outre
- **Atteignable en service** : non (le banc seulement)
- **Touche rag3weaver** : non

## Ce que c'est

`LockBench.AutoCheckpointIsNotPostponedForeverUnderWritersWithoutAGap`
(`test/transaction/concurrence/lock_bench_test.cpp`) juge qu'un point de reprise automatique passe
sous des écrivains qui ne laissent aucun trou, dans une borne de temps fixe. Ses étiquettes au
rouge : `committed-rows`, `every-commit`, `guard`.

Le 8 novembre, pendant la passe du lot `rejeu-relations-copiees`, il est rouge au banc complet. La
charge de luciepc est alors de 15,8 : l'ASAN du cœur et d'autres lourds tournent en même temps.
Rejoué seul sur le même bâti (2b0fba759), à une charge de 4,2, il est vert 5 fois sur 5. Le lot ne
touche que le rejeu du journal, aucun chemin du test.

Il est inscrit dans `probabilistic.txt` avec ces comptes. Le test n'est pas relâché.

## La forme attendue

Que la borne ne dépende plus du temps du poste :
- une horloge injectée, que le test fait avancer ;
- ou une borne qui tient compte de la charge, mesurée au début du cas, comme la sonde d'estimate
  corrigée le 8 novembre par la session archi.

## Pour le fermer

Le banc entier rejoué une fois sous une charge basse, plus tard dans la journée. S'il reste vert,
il reste à donner la forme ci-dessus au témoin pour l'ôter de `probabilistic.txt`. S'il rougit
encore à charge basse, le défaut n'est pas de charge : il se cherche dans le moteur.
