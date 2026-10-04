# Une ligne très loin des autres est injoignable dans un index bâti d'un coup

- **État** : ouvert — **bloque la stèle, provisoirement** (orchestration, 5 octobre 2026) : tant qu'une mesure sur un corpus réel n'a pas dit si c'est la rareté d'un jeu fait dur exprès ou un taux réel. Une ligne indexée invisible durablement n'est pas un rappel approché
- **Gravité** : réponse fausse
- **Atteignable en service** : oui, sans aucune mise à jour
- **Touche rag3weaver** : non mesuré. La forme du 5 octobre (plus bas) est celle de son chargement : index posé d'avance, morceaux par `COPY` successifs. Mesure demandée à l'arbre principal sur un corpus réel (taux de morceaux introuvables par leur propre vecteur)
- **Ouvert le** : 4 octobre 2026, session du banc (au journal des chantiers §6 depuis le 3 octobre)
- **Pour** : cœur C++ (extension vector)

## Ce que c'est

Dans un nuage serré de points (toutes les coordonnées sous 31), une ligne isolée très loin
des autres, à `[9000, 2, 3, 4]`, puis un index bâti d'un coup après le remplissage : la
ligne, ou d'autres, manquent à une recherche exhaustive environ un essai sur trois. Avec un
nuage plus étalé (la dernière coordonnée jusqu'à 499), c'est vert. Le défaut est encore
rouge sur master `93f4e81e8`, après `1ea49837f` qui borne autrement le parcours de la
construction (9 essais sur 30 le 4 octobre au soir).

## Recette minimale

```cypher
CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);
UNWIND range(0, 499) AS i CREATE (n:Doc {id: i}) SET n.vec = [i % 17, i % 23, i % 29, i % 31];
MATCH (n:Doc) WHERE n.id = 250 SET n.vec = [9000, 2, 3, 4];
CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');
-- recherche exhaustive (k = efs = 500) : moins de 500 lignes rendues, un essai sur trois
```

## Témoin

`test/transaction/concurrence/vector_index_update_test.cpp` —
`FarRowFromATightCloudInAnIndexBuiltAtOnce` (rouge connu, 30 essais, étiquette
`vector-index-exact`) ; le garde-fou vert `FarRowInAnIndexBuiltAtOnce` (nuage étalé).

## Cause

Inconnue. Parent probable du défaut de l'index bâti sur cent lignes qui perd le nœud 13
(`MinimalReproduction.HnswBuiltOnTheFirstHundredRowsLosesANode`). Là, `ml := 20, mu := 10`
ne fait manquer aucun nœud, ce qui désigne l'élagage des voisins à la construction.

## Correctif de l'amont (à lire, ne pas copier)

Aucun trouvé.

## Pour le fermer

Le témoin passe au vert sur ses 30 essais.

## La même famille par des COPY successifs (5 octobre 2026)

D'abord lu comme un dégât de l'annulation d'un `COPY` (ticket
`2026-10-05-annulation-d-un-copy-abime-l-index-vectoriel.md`, requalifié) ; ce n'est pas
l'annulation. Une ligne peut être injoignable **dès sa construction par des `COPY`
successifs dans une table déjà indexée**, sans aucune annulation ni mise à jour.

Recette (la forme du banc, `ProductReloadRecovery`) : une table `Chunk(id, emb FLOAT[64])` ;
l'index posé sur la table vide (`mu := 30, ml := 60, pu := 0.05, metric := 'cosine',
alpha := 1.1, efc := 200`) ; quatre fois `BEGIN`, `COPY` de 32 lignes, `COMMIT`, sans point
de reprise ; puis, pour chacune des 128 lignes, la recherche de son vecteur exact
(`k = 3, efs := 200`).

Exécuté par la session cœur C++ dans un seul processus, huit passes, sur `35d09c466` :
- quatre passes : la ligne 15 ne sort pas (« row 15 gives 87 ») dès la fin des quatre
  `COPY` validés ; le même manque, lui seul, se retrouve identique après trois `COPY`
  annulés, après un point de reprise, après une réouverture ;
- quatre passes : aucun manque, à aucune étape.
L'annulation rend donc exactement l'état d'avant, dans les huit passes. Dans la transaction
des `COPY` (avant le `ROLLBACK`), une à trois lignes anciennes manquent ; l'annulation les
rend.

Ce que ça dit : `efs = 200` sur 128 lignes visite tout ce qui est joignable ; une ligne
absente pour son propre vecteur n'a plus d'arête entrante. L'élagage des voisins à
l'insertion (`OnDiskHNSWIndex::shrinkForNode`) garde les meilleurs voisins d'une ligne et
jette les autres sans vérifier que la ligne jetée reste atteinte par quelqu'un. La garantie
« toutes joignables » n'existe que pour la mise à jour et la suppression
(`keepNodeReachable`, `c8fdaf196`). Le « une fois sur deux » vient du tirage des lignes de
la couche haute (`pu`). Les vecteurs de ce jeu sont très proches les uns des autres, faits
durs exprès ; la fréquence sur des vecteurs réels n'est pas mesurée.

Pourquoi le banc l'avait lu comme l'annulation (inférence, non exécutée) : sa fin « mort »
rejoue le journal, donc rebâtit le graphe par le chemin de l'insertion ordinaire ; ses deux
fins avec annulation passent par un point de reprise et relisent le graphe tel que le
`COPY` l'a bâti.

Témoin de diagnostic, hors dépôt parce que rouge une fois sur deux par nature :
`extension/rag3weaver/docs/3-octobre-2026-23h31/coeur-cpp/annexes/vector_index_rollback_test.avec-temoin-des-lignes-injoignables.cpp`
(classe `VectorIndexRollbackOfUncheckpointedRowsTest`). Au banc :
`Ends/ProductReloadRecovery.VectorsSetAfterRecoveryAreFound` (dans `probabilistic.txt`).

## Ce que font les index établis à l'insertion (lu le 5 octobre 2026)

Lecture des sources par un agent, branches principales du jour (hnswlib `ca426729`, pgvector
`f37c13f6`, Qdrant `6ab21cac`) ; les numéros de ligne valent pour ces commits.

| | hnswlib | pgvector | Qdrant |
|---|---|---|---|
| Une arête entrante garantie à l'élagage | non (`mutuallyConnectNewElement`, `hnswalg.h`) | non (`HnswUpdateConnection`, `hnswutils.c`) | non (`connect_with_heuristic`, `links_container.rs`) |
| Les voisins écartés comblent la liste (« keepPrunedConnections », algorithme 4 de l'article) | absent | présent, toujours actif (`SelectNeighbors`, l. 1154-1156) : au plus une arête perdue par voisin mis à jour | absent |
| Réparation après coup des orphelins de l'élagage | aucune ; `checkIntegrity()` n'est qu'un assert de test | aucune ; `VACUUM` ne répare que les trous des suppressions | aucune ; le « healer » ne soigne que les voisins de points supprimés ; `subgraph_connectivity` mesure sans réparer |
| La parade pratique | reconstruire | la perte bornée ci-dessus, `VACUUM`, `REINDEX` | recherche exacte sous un seuil de taille (`indexing_threshold_kb`, `full_scan_threshold_kb`), graphe rebâti à l'optimisation de segment, début du bâti sur un seul fil |

Donc aucune des trois ne garantit la joignabilité à l'insertion : la parité avec Qdrant
n'exige pas cette garantie. Ce qu'elles ont et que nous n'avons pas : pgvector borne la
perte (une arête par élagage, liste toujours pleine) ; Qdrant ne passe pas par le graphe pour
les petits ensembles.

Les chiffres trouvés, lus par résumé de page et non dans du code : à la construction pure la
probabilité est dite « extrêmement faible » (Xiao et al., arXiv 2407.07871), mais une issue
hnswlib ouverte montre des degrés entrants nuls dès la construction (#586) ; avec des
suppressions et réinsertions répétées, 2 à 4 % de points injoignables ; chez Qdrant, sur
LAION 2M, environ 0,3 % d'injoignables restent après leur correctif des doublons exacts
(PR #10239). Non vérifié par nous.

Une piste que cette lecture ouvre, non décidée : combler la liste avec les voisins écartés,
comme pgvector — c'est dans l'élagage lui-même, sans recherche de plus.

## La décision (orchestration, 5 octobre 2026)

Dans cet ordre : (c) mesurer d'abord, sur un corpus réel, le taux de lignes introuvables par
leur propre vecteur (arbre principal) ; puis (b), si le taux n'est pas nul, une passe de
rattrapage hors du chemin chaud — en fin de `COPY` ou au point de reprise, chercher les
lignes sans arête entrante et les rattacher ; (a), la garantie à l'insertion même
(`keepNodeReachable` après chaque élagage), seulement si (b) ne suffit pas. Rien n'est codé
avant la mesure.
