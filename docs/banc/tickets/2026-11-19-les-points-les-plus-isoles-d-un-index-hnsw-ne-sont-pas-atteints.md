# Les points les plus isolés d'un index HNSW ne sont pas atteints

- **État** : ouvert ; constat sur une base synthétique, à chiffrer sur des vecteurs réels avant de juger.
- **Famille** : moteur
- **Gravité** : à établir. Sur la base synthétique, une ligne isolée n'est presque jamais rendue à sa propre requête.
  Si c'est vrai sur des vecteurs réels, c'est une réponse fausse : une ligne existe, et une recherche exacte de son
  vecteur ne la rend pas.
- **Atteignable en service** : à établir, par la mesure sur un vrai corpus.
- **Touche rag3weaver** : non, l'extension vector (le HNSW).
- **Pour** : hotfixs (au mot de c7, 19 novembre).
- **Signalé par** : banc, la mesure de l'amorce séquentielle (`docs/banc/19-11-2026/01-l-amorce-sequentielle-mesuree.md`).
- **Ouvert le** : 19 novembre 2026

## Le constat

La base du 8 novembre compte 120 000 vecteurs gaussiens de dimension 768, en l2, avec l'index HNSW par défaut. Le
test `VectorIndexSeedMeasure.RecallAndBuildTimeWithAndWithoutTheSeed` (branche `isolated-point-found-first-measure`,
230614bea4) prend un échantillon de 2 000 lignes et en garde les 200 les plus isolées (la plus grande distance à leur
plus proche voisin). Chacune est cherchée par son propre vecteur, à k = 10 :

| `seed_points` | fils | rendue au premier rang | rendue dans le top 10 |
|---|---|---|---|
| 0 | 8 | 0 / 200 | 3 / 200 |
| 1 000 | 8 | 0 / 200 | 4 / 200 |
| 0 | 24 | 0 / 200 | 2 / 200 |
| 1 000 | 24 | 0 / 200 | 1 / 200 |

Dans le même passage, le rappel global à k = 10 sur 200 requêtes vaut 0,75.

- **Ce n'est pas le défaut corrigé par l'amorce** : celui-là est un point lointain que rejoint un nuage, et l'amorce
  ne change rien ici.
- Ces lignes sont des **anti-hubs** : en grande dimension, certains points ne sont le voisin de presque personne, et
  le graphe n'y mène pas.

## Ce qui manque avant de juger

- **Des vecteurs réels** : la même mesure sur les vecteurs bge-m3 (ou granite-278m) d'un vrai corpus, par exemple les
  `ScopeContent_Chunk` d'une copie de B′ (`banc-parite/migration-bprime-20261116`). Les gaussiennes sont le pire cas
  connu pour les anti-hubs, et un corpus réel peut ne pas en avoir.
- **Un point ordinaire comme témoin** : la recherche par son propre vecteur sur des lignes tirées au hasard, pas
  seulement isolées. Elle dira si l'échec est propre aux anti-hubs, ou au réglage de recherche par défaut (`efs`).
- **Ensuite, la décision** : une recherche plus large (`efs`), plusieurs points d'entrée, ou une liste des points
  isolés balayée à chaque recherche (la variante 3 de la page de hotfixs).

## Où rejouer

Le test est dans `test/transaction/concurrence/vector_index_update_test.cpp`, sur `isolated-point-found-first-measure`.
Il se lance par `CONCURRENCE_MEASURE=1 BANC_SEED_POINTS=… BANC_THREADS=… concurrence_test
--gtest_filter=VectorIndexSeedMeasure.*`, sous `poste mesure`. Les journaux du 19 novembre sont à luciepc :
`~/.cache/rag3db-banc-notes/amorce/run-seed*-th*.log`.
