# QUERY_VECTOR_INDEX déclare une ligne au lieu de k, et son masque se dimensionne sur ce chiffre

- **État** : corrigé (session cœur C++ hotfixs, 12 octobre 2026 ; voir « Correctif »)
- **Gravité** : estimation fausse. Une recherche de plus de 2 048 résultats aurait reçu un seul fil
  sous la règle des fils du cœur. Le semi-masque prendrait le mauvais type de roaring au-delà de
  4 milliards de lignes. Aucun résultat faux vu.
- **Atteignable en service** : l'estimation, oui, à chaque recherche vectorielle ; ses effets, non
  vus (le produit cherche quelques dizaines de résultats).
- **Touche rag3weaver** : oui, par les deux formes de `rag3db_search_backend.rs`.
- **Ouvert et fermé le** : 12 octobre 2026, à la demande de l'orchestration.

## Ce que c'est

`QueryHNSWIndexBindData` posait `numRows = 1` en dur, avec le commentaire « maxOffset ». Le plan
lisait donc une cardinalité de 1 pour la recherche directe. Pour la recherche filtrée, le
remetteur à jour des cardinalités reprenait celle de l'enfant, le plan du filtre (300 lignes sur
une table de 3 000). Le semi-masque du filtre était créé par `createMask(numRows)`, sur ce même 1.

La demande de départ disait la recherche « sans estimation », recevant 32 fils. La mesure l'a
démentie (un seul fil, card=1). Les « card=? » de la mesure de F viennent d'autres CALL. Le coût
qui suit la recherche est le balayage de la table pour rendre ses propriétés : c'est le §7 de la
page 12 du cœur, pas ce ticket.

## Témoin

`ThreadsByCardinality.AVectorSearchDeclaresKRows` (`test/transaction/threads_by_cardinality_test.cpp`) :
la trace par tâche (`RAG3DB_TRACE_TASKS`) doit lire `card=10` pour une recherche de k = 10, sur les
deux formes du produit (directe, et filtrée par `PROJECT_GRAPH_CYPHER`), avec un seul fil et le
même résultat à 8 fils et à 1. Rouge sans correctif : card=1 et card=300.

## Correctif

- `QUERY_VECTOR_INDEX` déclare `numRows = k` quand k est un littéral ou un paramètre positif, et 0
  (inconnu) sinon. `copy()` le recopie.
- Le semi-masque se dimensionne sur les lignes de la table (comme `map_scan_node_table`).
- `CardinalityUpdater::visitTableFunctionCall` : une fonction de table qui déclare `numRows > 0` le
  garde, borné par ses enfants estimés (au plus les lignes retenues par le filtre). Seul
  `QUERY_VECTOR_INDEX` filtré cumule une estimation déclarée et un enfant : les fonctions GDS ont des
  enfants, mais `numRows = 0`.
