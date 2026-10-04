# Une erreur du moteur dans la branche vecteur fait tomber toute la recherche hybride

- **État** : ouvert. Le comportement a été lu dans le code, pas testé. Le choix d'un repli par branche revient à Lucie.
- **Gravité** : blocage. La recherche ne rend rien, alors que la branche texte avait répondu.
- **Atteignable en service** : oui, dès que l'index vectoriel refuse une recherche. C'est le cas de l'index vectoriel laissé en retard par un COPY annulé (« vector::reserve », sonde `sonde_vecteurs_apres_rollback`), et des gardes « is behind its table » du correctif du cœur C++.
- **Touche rag3weaver** : oui, par toutes les entrées de recherche.

## Ce que c'est

Une recherche hybride fait tourner ses branches dans le même graphe de flux de données (vecteur, plein texte, creux), puis les fusionne. Quand la branche vecteur reçoit une erreur du moteur, tout le graphe s'arrête. Les résultats de la branche texte, déjà calculés, sont jetés, et l'appelant ne reçoit qu'une erreur.

## Le chemin, lu le 5 octobre 2026

- `src/dataflow/generic_search_nodes.rs:689` : `.map_err(|e| format!("VectorSearchNode: search failed: {e}"))?`. Le nœud ne remplit pas ses sorties.
- `src/dataflow/runtime.rs`, dans `execute_inner` :
  - les nœuds d'un même niveau tournent en parallèle (`run_level`), donc la branche texte finit ;
  - à la première `Err`, le moteur de flux émet `NodeFailed` puis `Failed`, et fait `return Err(error)` (vers les lignes 1106 à 1114) ;
  - `FuseResultsNode` ne tourne donc pas.
- Aucune entrée ne se replie sur le texte seul :
  - `Catalog::rechercher` (`src/catalog.rs`, vers 7857) traduit l'erreur en `EmbeddingModelUnavailable` ou en `DbError`. Sur plusieurs portées, la première erreur arrête la boucle.
  - `SearchPlan::execute` (`src/dataflow/search_chain.rs:208`) n'en fait pas plus.
  - Les outils du backend (`src/backend.rs`, `execute(&mut graph)?`) non plus.
  - Les outils de graphe pour l'agent (`src/dataflow/graph_tool.rs:607`) rendent un JSON `{"error", "detail"}`, sans résultat partiel.
- `node_warnings` existe dans le nœud vecteur. Il ne sert qu'aux problèmes légers : un filtre qui ne compile pas, ou un service `catalog` absent. Les échecs de recherche, d'embarquement et de résolution du modèle sont des erreurs dures.

## Recette minimale

Aucune recette Cypher, puisque le défaut est dans l'enchaînement de rag3weaver. Pour le reproduire :

1. prendre une base dont l'index vectoriel refuse la recherche (la base « rollback » de la sonde, avant le correctif du cœur C++) ;
2. lancer une recherche dont les signaux comprennent le vecteur et le BM25 ;
3. on obtient une erreur, au lieu des résultats BM25.

## Témoin

Il n'y en a pas. Pour l'écrire, il faut un nœud vecteur qu'on peut faire échouer (un crochet de test, ou une base piégée) et une assertion sur ce qu'on attend. C'est justement ce qu'on attend qui reste à décider.

Un test existant fige le comportement actuel pour un cas voisin. `tests/e2e_prise_atomique.rs`, vers la ligne 979, exige `Err(EmbeddingModelUnavailable)` quand le modèle est absent. Un repli changerait ce contrat, ou devrait l'excepter.

## La décision qui reste (Lucie)

1. **Garder l'échec entier**, comme aujourd'hui. Une recherche amputée d'une branche ne se fait pas passer pour complète.
2. **Un repli par branche** :
   - la branche en erreur rend une liste vide et un avertissement nommé, par exemple « branche vecteur : … » dans les `warnings` du résultat ;
   - la fusion se fait avec les branches restantes ;
   - c'est l'échec entier si toutes les branches échouent ;
   - à décider : la portée (le vecteur seul, ou toute branche), et ce qu'on fait du cas « modèle absent », aujourd'hui une erreur exigée.
3. **Un repli au choix de l'appelant**, par une option de recherche. L'agent pourrait alors l'activer, et l'API rester stricte.

## Pour le fermer

Il faut le choix de Lucie, puis le témoin qui le fige.
