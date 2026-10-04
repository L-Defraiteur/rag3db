# Une ligne très loin des autres est injoignable dans un index bâti d'un coup

- **État** : ouvert
- **Gravité** : réponse fausse
- **Atteignable en service** : oui, sans aucune mise à jour
- **Touche rag3weaver** : peu probable (ses plongements forment un nuage sans point isolé à ce point), non vérifié
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
