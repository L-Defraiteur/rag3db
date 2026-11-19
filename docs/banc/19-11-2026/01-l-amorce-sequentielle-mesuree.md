# L'amorce séquentielle du HNSW, mesurée (19 novembre 2026)

La demande vient de c7, pour hotfixs : la preuve (b) du correctif du point isolé. Avant la phase parallèle du bâti,
les 1 000 premiers points sont insérés par un seul fil (`seed_points`, 1 000 par défaut, 0 = le bâti d'avant). La
question est simple : est-ce que l'amorce coûte en rappel ou en temps de bâti ? Le correctif est porté par la page
`docs/coeur-cpp-hotfixs-tickets/19-11-2026/02-le-point-isole-trois-variantes.md`.

## La mesure

- **Le test** : `VectorIndexSeedMeasure.RecallAndBuildTimeWithAndWithoutTheSeed`
  (`test/transaction/concurrence/vector_index_update_test.cpp`), sur la branche `isolated-point-found-first-measure`
  (230614bea4). Cette branche est posée sur celle de hotfixs (3536e55a66), avec un seul commit, le test. Le test est
  sauté sans `CONCURRENCE_MEASURE=1`.
- **La base du 8 novembre** : 120 000 vecteurs de dimension 768, tirés par `mt19937` à partir de `id + 1` et écrits
  avec trois chiffres significatifs. La force brute compare les mêmes valeurs arrondies que celles que l'index a reçues.
- **Les réglages** : `seed_points` (`BANC_SEED_POINTS`) vaut 0 ou 1 000, et le nombre de fils (`BANC_THREADS`) 8 ou 24.
  Les fils sont posés par `maxNumThreads` avant l'ouverture et par `CALL threads=N` : la configuration des tests vaut
  2 par défaut, et à 2 fils le défaut ne se montre pas.
- **Ce qui est mesuré** :
  - le temps du `CREATE_VECTOR_INDEX` ;
  - le rappel à k = 10 sur 200 requêtes tirées hors de la table, contre une recherche exhaustive ;
  - les 200 points les plus isolés d'un échantillon de 2 000, cherchés par leur propre vecteur.
- **Où** : à luciepc, binaire 4470b4009d8ca45c (Release), sous `poste mesure`, de 13 h 06 à 13 h 12. La charge
  au départ était de 5,3. Les journaux sont dans `~/.cache/rag3db-banc-notes/amorce/run-seed*-th*.log`, et le lanceur
  dans `~/.cache/rag3weaver-build/amorce.sh`.

## Les chiffres

| `seed_points` | fils | CREATE | rappel@10 | points isolés : premier rang | dans le top 10 |
|---|---|---|---|---|---|
| 0 (avant) | 8 | 45,7 s | 0,7515 | 0 / 200 | 3 / 200 |
| 1 000 | 8 | 47,0 s | 0,7505 | 0 / 200 | 4 / 200 |
| 0 (avant) | 24 | 30,3 s | 0,7445 | 0 / 200 | 2 / 200 |
| 1 000 | 24 | 31,8 s | 0,7500 | 0 / 200 | 1 / 200 |

## Ce que ça dit

- **L'amorce ne coûte pas de rappel** : les écarts sont au millième, dans les deux sens (−0,001 à 8 fils, +0,0055 à 24).
- **Elle coûte peu au bâti** : +3 % à 8 fils, +5 % à 24 fils. C'est le même ordre que le +6,5 % mesuré par hotfixs sur
  20 000 lignes.
- **Le volet des points isolés ne prouve rien.** Sur des gaussiennes de dimension 768, les points les plus isolés
  sont des anti-hubs que le graphe atteint à peine. Ils ne sont presque jamais retrouvés par leur propre vecteur, avec
  comme sans amorce. Ce n'est pas la forme du défaut, un point lointain que rejoint un nuage. Comme hotfixs l'avait
  dit, son « avant » parfait par `transaction_test` ne prouvait pas l'absence du défaut ; cet « avant » sans succès ne
  prouve pas non plus sa présence. La preuve (a) du correctif reste son témoin.
- **Une limite** : la même recherche par leur propre vecteur n'a pas été faite sur des points ordinaires. Elle
  aurait dit si 0/200 est propre aux anti-hubs, ou au réglage de recherche par défaut.
