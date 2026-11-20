# Les points les plus isolés d'un index HNSW ne sont pas atteints

- **État** : ouvert, en faible gravité (20 novembre 2026, au mot de c7 : un défaut connu et non corrigé garde son
  ticket). Fermé un moment le même jour, puis rouvert. La mesure de hotfixs sur des vecteurs réels
  (`docs/coeur-cpp-hotfixs-tickets/20-11-2026/03-les-points-isoles-sur-des-vecteurs-reels.md` ; branche
  `seed-measure-real-vectors`, 8caebb63e9) : sur les 126 998 `Scope_Chunk` indexés de code-lourd, 200 points isolés
  sur 200 dans leur propre top 10 et un rappel@10 de 0,998 à 0,999, avec ou sans amorce. Le défaut est réel sur la base
  gaussienne du banc, invisible sur ce corpus.
- **Ce qui le ferait changer** :
  - **remonter en réponse fausse** : un corpus réel, ou un autre embarqueur, où le rappel@10 tombe sous 0,99 ou
    des points isolés manquent dans leur propre top 10 ;
  - **le fermer** : la preuve qu'aucun embarqueur ne produit une distribution comme la gaussienne de dimension 768 du
    banc (de dimension intrinsèque maximale), ou un correctif dont le témoin passe sur la base gaussienne.
- **Famille** : moteur
- **Gravité** : faible (20 novembre) : le cas ne se montre que sur des gaussiennes de dimension 768. Avant :
  à établir. Sur la base synthétique, une ligne isolée n'est presque jamais rendue à sa propre requête.
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

## Fermé : la mesure sur des vecteurs réels (hotfixs, 20 novembre)

- **Le jeu** : `VectorIndexSeedMeasure` avec `BANC_VECTORS_CSV` (une ligne `[x,…]` par vecteur, les 200 dernières en
  requêtes), sur les 127 198 `Scope_Chunk` de code-lourd, à 8 fils, avec et sans amorce. Page de hotfixs :
  `docs/coeur-cpp-hotfixs-tickets/20-11-2026/03-…`.
- **Le résultat** : **200 points isolés sur 200 dans leur propre top 10**, et un rappel@10 de 0,998 à 0,999, avec ou
  sans amorce. Sur la base gaussienne, c'était 1 à 4 sur 200, pour un rappel de 0,75.
- **La lecture** : les anti-hubs sont le pire cas des gaussiennes en grande dimension. Un corpus de code réel n'en a pas
  assez pour que le graphe les perde. Une recherche par le vecteur exact d'une ligne la rend.
- **Une faute de ma mesure, relevée par hotfixs** : la colonne « rendue au premier rang » ne vaut rien. Sans `ORDER BY`,
  l'ordre des lignes de `QUERY_VECTOR_INDEX` n'est pas garanti : sur les vecteurs réels, elle vaut 12 à 22 sur 200, alors
  que chaque point est à distance nulle de lui-même. Le top 10 ne dépend pas de l'ordre : c'est lui qui juge, et sur la
  base gaussienne il restait à 1 à 4 sur 200.
- **Ce qui reste** : le cas gaussien, sans portée en service. S'il revient sur un vrai corpus, ce ticket se rouvre.
