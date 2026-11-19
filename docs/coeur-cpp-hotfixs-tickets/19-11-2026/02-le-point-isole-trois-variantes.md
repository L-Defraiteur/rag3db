# Le point isolé que rejoint un nuage : trois variantes à mesurer

Suite du ticket
[un rang déplacé pendant un CREATE…](../tickets/2026-11-07-un-rang-deplace-pendant-un-create-n-est-pas-toujours-rattrape-premier.md),
décision de c7 du 19 novembre, à relire par Lucie : une recherche du vecteur exact qui ne rend pas le point
lui-même est une réponse fausse. Le lot se fera sur la variante qui tient, avec ses chiffres.

## La cause, telle que la sonde la montre

Un point loin de tout (le rang 1 à `[500, 0, …, -1]`) est relié au bâti à des nœuds du nuage d'origine. Toutes ses
distances au nuage sont proches de 490, et la recherche ne les départage pas : ses 60 voisins ne sont pas ceux par
où passent les recherches venues de sa direction. Quand des points sont ensuite posés à 1,41 de lui (SET, insertion
ou rattrapage), la recherche d'insertion explore environ `efc` = 200 nœuds, tous dans le nuage d'origine, et aucun
n'a d'arête vers lui. Le nouveau nuage se forme sans lui.

C'est un **échec de recherche à l'insertion**, pas de sélection des voisins. L'heuristique de l'article HNSW
(algorithme 4 : `extendCandidates`, `keepPrunedConnections`) n'ajoute que des points déjà vus. hnswlib et faiss ne
l'activent d'ailleurs pas. Augmenter `efc` ne fait que déplacer la probabilité.

## Les variantes

1. **Promouvoir un point isolé dans la couche haute.** À l'insertion (bâti en mémoire, `insertInternal`, rattrapage),
   un point dont le plus proche voisin trouvé est loin, comparé aux distances entre ces voisins eux-mêmes, entre
   aussi dans la couche haute. Les recherches qui viennent de sa direction y passent par lui, puis descendent à
   partir de lui. Risque : la couche haute grossit d'autant de points isolés, et le critère « loin » est à régler.
2. **Chercher depuis plusieurs points d'entrée.** La recherche de la couche basse part de l'entrée trouvée en
   couche haute, et aussi de quelques autres nœuds de la couche haute. C'est probabiliste : moins de manques, aucune
   garantie.
3. **Une liste des points isolés, balayée à chaque recherche.** Les points dont le plus proche voisin, à
   l'insertion, est au-delà d'un seuil sont tenus dans une petite liste. Chaque recherche calcule leur distance, à
   la manière de `searchFromUnCheckpointed`, et les fusionne aux résultats. C'est exact pour eux, tant que la liste
   reste petite. Il faut la tenir à jour sous MVCC (suppression, SET, rattrapage) et la persister, ou la rebâtir à
   l'ouverture.

## Ce qui est mesuré pour chacune

- le témoin `VectorIndexIsolatedPoint` : 6/6 par SET et 6/6 par insertion ;
- le rappel global (k = 10) sur la base du banc (120 000 vecteurs de dimension 768) et le rappel sur ses points les
  plus isolés, contre la recherche exacte ;
- le temps de bâti, d'insertion et de recherche, avant et après ;
- les suites du lot : transaction_test, concurrence VectorIndex*, les .test de vector, e2e_test.

Une variante qui n'atteint pas 6/6 sur le témoin est écartée, quels que soient ses autres chiffres.
