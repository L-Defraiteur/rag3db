# Le point isolé que rejoint un nuage : trois variantes à mesurer

> **Revu le 19 novembre, 10 h 30 : la cause première est le bâti parallèle.** La même recette au shell, 12 essais par
> nombre de fils du bâti (`CALL threads`), sur disque : **0 manqué à 1 fil, 0 à 4, 5 à 8, 4 à 24**. Le témoin gtest
> passait parce que les tests tournent à 2 fils ; il fixe désormais 8 fils et joue dix tours (92f40e9ae4).
>
> **La course.** Dans `InMemHNSWLayer::insertRel`, un fil réserve une case de la liste de voisins par un incrément
> atomique (`incrementCSRLength`), puis l'écrit (`setDstNode`). Le fil qui prend la dernière case lance
> `shrinkForNode`, qui relit la liste et la réécrit. Une case réservée par un autre fil mais pas encore écrite est
> lue vide et perdue. Une case écrite après l'élagage tombe hors de la nouvelle longueur, ou écrase un voisin gardé.
> Les arêtes perdues touchent d'abord un point isolé, qui en a peu : ses arêtes de retour depuis le nuage.
>
> **11 h 45 : l'amorce séquentielle tient (variante de c7).** Les N premiers points insérés par un seul fil avant la
> phase parallèle, pour qu'aucun fil n'entame sa tranche sur un graphe de quelques nœuds. Même binaire, réglé par une
> variable d'expérience, à luciepc sous poste lourd (5 cœurs) :
>
> | | manqués à 8 fils | à 24 fils | CREATE de 20 000 lignes, 8 fils |
> |---|---|---|---|
> | sans amorce | 3/12 | 3/12 | 1 534 à 1 569 ms |
> | amorce de 1 000 points | 0/12 | 0/12 | 1 614 à 1 674 ms (+6,5 %) |
>
> Le lot se fait sur elle : une amorce de min(N, 1 000) points, le témoin du shell, 36 essais de la recette ; puis le
> coût et le rappel sur la base du banc (120 000 × 768).
>
> **11 h 15 : le verrou par nœud ne corrige pas.** Écrit et mesuré (non commité) : avec lui, la recette manque encore
> 5/12 à 8 fils et 8/12 à 24, et le témoin du shell (`tools/shell/test/test_shell_isolated_point.py`) reste rouge.
> La perte d'arêtes par la course existe peut-être, mais ce n'est pas elle qui isole le point. Ce qui tient : le
> rang 1, au décalage 1, est inséré au tout début du bâti, quand chaque fil entame sa tranche de 2 048 lignes et que
> le graphe ne compte que quelques nœuds ; ses voisins sont choisis parmi eux. Prochaine piste à mesurer : réinsérer,
> à la fin du bâti, les nœuds insérés en tête de chaque tranche ; ou la promotion d'un point isolé (variante 1).
> Le témoin gtest ne reproduit pas (plus de 100 tours, même avec une pause injectée) : retiré (2666554278).
>
> **Le correctif retenu (c7)** fait ce que fait hnswlib sous `addPoint` concurrent : un verrou par nœud (ici un
> tableau de verrous répartis par décalage). Il est tenu pour ajouter à une liste, pour l'élaguer, et pour la copier
> pendant une recherche du bâti. Les trois variantes ci-dessous visaient la conséquence ; elles restent en réserve,
> au cas où le point isolé manquerait encore à 1 fil sur une autre recette.

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
