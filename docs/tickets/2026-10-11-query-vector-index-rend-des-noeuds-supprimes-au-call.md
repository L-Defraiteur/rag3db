# QUERY_VECTOR_INDEX rend des nœuds supprimés au niveau du CALL

- **État** : ouvert
- **Gravité** : réponse fausse (le compte des lignes du CALL ; les nœuds projetés sont justes)
- **Atteignable en service** : oui, après toute suppression d'une ligne indexée
- **Touche rag3weaver** : non vu. Ses recherches projettent le nœud, et cette projection filtre les lignes supprimées. Une requête qui compterait ou limiterait les lignes du CALL avant la projection en serait touchée.
- **Ouvert le** : 11 octobre 2026, session du banc, sur un relevé du cœur C++ (témoin d'un fil de la marche I1)

## Ce que c'est

Après la suppression de lignes indexées, la recherche vectorielle rend encore leurs nœuds au
niveau du CALL. Avec `RETURN count(*)`, on compte plus de lignes qu'il n'en reste de vivantes.
Avec `RETURN node.id`, les lignes supprimées disparaissent, parce que la projection par `node`
les filtre. Un CALL qui demande k lignes peut donc en rendre moins de k vivantes, alors qu'il en
existe davantage.

## Recette minimale

```cypher
CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[8]);
-- 100 documents avec un vecteur, puis l'index
CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');
MATCH (n:Doc) WHERE n.id IN [10, 11] DELETE n;
CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', <un vecteur>, 100) RETURN count(*);  -- 100
CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', <un vecteur>, 100) RETURN node.id;   -- 98, sans 10 ni 11
```

## Témoin

Le témoin d'un fil de la marche I1 (cœur C++, branche `verrous-i1`) en a donné le relevé. Le
banc n'a pas encore de témoin dédié.

Le vérificateur d'intégrité (`integrity_checker.cpp`, contrôle `vector-index-dead`) demande
k = efs = le nombre de lignes vivantes et projette la clé. Il ne voit donc rien : les nœuds
morts sont filtrés par la projection, et ils ne prennent une place dans les k que si la
recherche les range avant des vivantes. Pour le mesurer, il faut un témoin qui demande
k = vivantes + une marge et qui compare `count(*)` au nombre de clés projetées.

## Cause

Elle est probable, mais non lue dans le code : la recherche ne consulte pas le masque de
visibilité de la table. Un nœud supprimé dont les arêtes n'ont pas encore été retirées du
graphe reste un résultat, et seul le balayage qui projette le nœud l'écarte.

## Correctif de l'amont (à lire, ne pas copier)

Non cherché.

## Pour le fermer

La recherche écarte les nœuds invisibles à la transaction avant de compter les k. Le témoin
compare `count(*)` à k et aux clés projetées, après une suppression validée et dans la
transaction qui supprime.
