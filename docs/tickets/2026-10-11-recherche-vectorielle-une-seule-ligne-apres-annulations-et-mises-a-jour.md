# Après des écritures annulées et des mises à jour de vecteur validées, la recherche ne rend plus qu'une ligne

- **État** : ouvert (11 octobre 2026, 2 h 15, trouvé par le témoin des verrous d'I1).
- **Gravité** : réponse fausse (une recherche vectorielle rend 1 ligne pour 100 vivantes).
- **Atteignable en service** : à établir — vu sous le mode multi-écrivains ; les quatre pièces
  prises une à une hors du mode (une annulation de `CREATE`, de `SET` de vecteur, de `DELETE` ;
  trois `SET` validés) sont vertes.
- **Touche rag3weaver** : si la composition est atteignable avec un seul écrivain, oui (le
  `SET` de vecteur par paquet, les refus de paquet annulés).

## Ce que c'est

Sur une table `Doc(id, vec FLOAT[4])` de 100 lignes avec un index vectoriel, déterministe (3 sur
3, luciepc, `verrous-i1` à `6ecf2448b`) : pour chacune des trois écritures W ∈ {`CREATE` d'une
ligne ; `SET` du vecteur de la ligne 1 ; `DELETE` de la ligne 2}, la connexion A fait `BEGIN`,
W ; la connexion B (délai de verrou 300 ms) tente `CREATE`, `SET`, `DELETE` d'autres lignes
(les trois refusées par le délai, avant de toucher l'index : le verrou est pris d'abord) ; A
`ROLLBACK` ; B `SET` du vecteur de la ligne 3 à `[8, 8, 8, 8]`, validé. Puis
`QUERY_VECTOR_INDEX(v, 100) RETURN node.id` rend **1** ligne ; `MATCH` en compte 100.

Chaque pièce seule est verte (`IndexedTableRollbackTest`, quatre cas, hors du mode). C'est la
composition — des annulations suivies de mises à jour validées — qui casse.

## Pistes

- Les points d'entrée de l'index (`HNSWStorageInfo`) vivent en mémoire et **ne s'annulent pas
  seuls** (`hnsw_index.h`, `rollbackInsert` ne corrige qu'une insertion annulée ; une
  suppression ou une mise à jour annulée ne rend pas le point d'entrée) ; le `SET` validé qui suit
  repart d'un point d'entrée périmé.
- Le correctif 2 du lot « mise à jour de vecteurs » du banc (`insertInternal` ne part jamais du
  nœud qu'il insère ; une requête qui entre par une ligne sans arête ne rend qu'elle) a la même
  signature (« 1 results »). À rejouer sur sa lib.

## Témoin

`test/transaction/insert_lock_test.cpp`,
`IndexedTableLockTest.EveryWriterOfAnIndexedTableHoldsTheIndexExclusiveUntilTheEnd` avec son
dernier contrôle remplacé par `liveRowsByIndex(*conn, 100) == 100` (le témoin livré compte par
`MATCH`, pour ne pas porter ce défaut) ; les quatre pièces dans `IndexedTableRollbackTest`.

## Pour le fermer

Rejouer la séquence sur la lib du lot « mise à jour de vecteurs » ; si elle reste rouge, rendre
les points d'entrée à l'annulation d'une mise à jour et d'une suppression (ou les recalculer à
la première écriture validée qui suit), avec ce témoin au vert. Propriétaire : le banc (extension
vector).
