# Après un COPY annulé sur une table à index vectoriel, la recherche échoue, puis le COPY suivant n'est pas indexé

- **État** : ouvert — **bloque la stèle** (résultat faux durable, en silence)
- **Gravité** : réponse fausse (des lignes absentes de l'index), précédée d'un blocage (la recherche échoue)
- **Atteignable en service** : oui — un seul écrivain, une transaction explicite, une table déjà indexée
- **Touche rag3weaver** : non vérifié — il faut un `COPY` annulé sur une table qui porte déjà son index vectoriel (le chemin d'échec d'un paquet sous la transaction par paquet, si l'index existe à ce moment). À établir par l'arbre principal
- **Ouvert le** : 5 octobre 2026, session cœur C++ (en éprouvant le versement des lignes locales avant un `COPY`)
- **Pour** : cœur C++ (index vectoriel, annulation)

## Ce que c'est

L'index vectoriel tient le compte des lignes qu'il a déjà reliées. Un `COPY` l'avance ; l'annulation de la transaction retire les lignes de la table et ne recule pas ce compte. L'index se croit alors en avance sur sa table.

## Recette minimale

```cypher
CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);
-- 200 lignes, puis :
CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');
BEGIN TRANSACTION;
COPY Doc FROM 'f.csv';      -- cent lignes, id 2000 à 2099, vec [1, 2, 3, id]
ROLLBACK;
CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', [1.0, 2.0, 3.0, 2050.0], 3) RETURN node.id;
                            -- erreur : vector::reserve
COPY Doc FROM 'f.csv';      -- le même, validé
MATCH (n:Doc {id: 2050}) RETURN count(*);   -- 1 : la ligne est dans la table
CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', [1.0, 2.0, 3.0, 2050.0], 3) RETURN node.id;
                            -- 199, 198, 197 : aucune des cent lignes copiées
```

Exécuté le 5 octobre 2026, par un essai jetable dans `test/transaction/vector_index_after_failed_copy_test.cpp` (retiré ensuite), sur le moteur de la branche du versement. Sans ligne locale le versement ne fait rien : c'est le chemin de `master`. Non rejoué sur un bâti de `master`.

Les deux effets :

1. **Après le `ROLLBACK`**, toute recherche échoue par `vector::reserve` : la recherche parmi les lignes pas encore reliées réserve « nombre de lignes − compte », qui reboucle (`OnDiskHNSWIndex::searchFromUnCheckpointed`, `extension/vector/src/index/hnsw_index.cpp:544`).
2. **Après le `COPY` suivant**, de même taille : le compte égale le nombre de lignes, et l'index conclut qu'il n'a rien à relier (`OnDiskHNSWIndex::finalize`, `hnsw_index.cpp:653`). Les lignes sont dans la table, pas dans l'index ; la recherche ne les rend jamais. Aucune erreur.

Des insertions ordinaires validées entre les deux remettent le compte à une valeur juste par accident (`commitInsert` le pose à « décalage + 1 », `hnsw_index.cpp:646`) — c'est ce qui masque le défaut dans la plupart des suites.

## La cause

`HNSWStorageInfo::numCheckpointedNodes` est avancé par `OnDiskHNSWIndex::finalize` (appelé à la fin d'un `COPY`) et par `commitInsert`. Rien ne le recule : l'interface des index (`src/include/storage/index/index.h`) n'a aucun crochet d'annulation, et l'annulation des lignes (`NodeTableVersionRecordHandler::rollbackInsert`, `src/storage/table/node_table.cpp`) ne connaît que l'index de clé primaire.

Depuis le versement des lignes locales avant un `COPY` (5 octobre), des insertions ordinaires suivies d'un `COPY` puis d'un `ROLLBACK` passent par le même chemin : même défaut, ni plus ni moins.

## Témoin

Aucun dans la liste : le cas validé est couvert (`VectorIndexAfterFailedCopyTest.InsertsThenACopyInOneTransactionAreAllIndexed`), le cas annulé non. La recette ci-dessus est le témoin à écrire, rouge, avec le correctif.

## Le correctif de l'amont

Non regardé.

## Pour le fermer

Un crochet d'annulation pour les index, appelé là où l'annulation retire des lignes d'une table, par lequel l'index vectoriel recule son compte à la première ligne retirée. À regarder avec : ce que deviennent les points d'entrée du graphe quand la ligne annulée en était un (table vide avant le `COPY`, ou nœud promu à la couche haute). Témoins : la recette, sur une table vide et sur une table peuplée ; la même avec des insertions ordinaires avant le `COPY` ; une recherche après chaque étape.
