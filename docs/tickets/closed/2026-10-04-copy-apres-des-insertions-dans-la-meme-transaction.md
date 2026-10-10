# Dans une transaction, après des insertions ordinaires puis un COPY dans la même table, la clé d'une ligne mène à une autre ligne

- **État** : corrigé le 5 octobre 2026 (« fix(stockage): un COPY verse d'abord dans la table les lignes que sa transaction y a déjà insérées — le cas refusé par un nom est rendu juste »). Refusé par son nom entre-temps (`0f4a54b2c`, 4 octobre) ; le refus et son nom sont retirés
- **Gravité** : réponse fausse et écriture sur la mauvaise ligne, validées en silence ; plantage quand la ligne visée n'existe pas
- **Atteignable en service** : oui — un seul écrivain, une transaction explicite
- **Touche rag3weaver** : contourné — sous la transaction par paquet à K > 1, un groupe replié sur `MERGE` pouvait être suivi d'un `COPY` dans la même table ; depuis `41136459a` une table de nœuds écrite par `MERGE` dans une transaction ne repasse plus par `COPY` avant la validation. Pour les tables de relations (un petit lot par `MERGE` puis un gros par `COPY`, cas courant), le même défaut n'est **pas vérifié**
- **Ouvert le** : 4 octobre 2026, session cœur C++ (en écrivant les témoins du chargement journalisé)
- **Pour** : cœur C++ (stockage, transactions)

## Ce que c'est

Les lignes insérées par une transaction (hors `COPY`) restent dans son stockage local, sous des décalages provisoires qui commencent au nombre de lignes de la table au moment de la première insertion (`LocalNodeTable::getStartOffset`, `Transaction::getMinUncommittedNodeOffset`). Un `COPY` de la même transaction écrit, lui, directement dans la table — aux mêmes décalages. Tout ce qui suit prend un décalage du `COPY` pour une ligne locale (`Transaction::isUnCommitted`) : la recherche par clé va lire une ligne locale qui n'existe pas.

## Recette minimale

```cypher
CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);
CREATE REL TABLE L(FROM Doc TO Doc);
BEGIN TRANSACTION;
UNWIND range(1000, 1019) AS i CREATE (:Doc {id: i, name: 'locale ' + CAST(i AS STRING)});
COPY Doc FROM 'f.csv';                               -- cent lignes, id 0 à 99
MATCH (n:Doc {id: 5}) RETURN n.id, n.name;           -- rend 1005 | locale 1005
MATCH (n:Doc {id: 5}) SET n.name = 'X';
MATCH (a:Doc {id: 7}), (b:Doc {id: 8}) CREATE (a)-[:L]->(b);
COMMIT;
MATCH (n:Doc) WHERE n.name = 'X' RETURN n.id;        -- 1005, pas 5
MATCH (a:Doc)-[:L]->(b:Doc) RETURN a.id, b.id;       -- 1007 | 1008, pas 7 | 8
```

Exécuté le 4 octobre sur le moteur de master (`annexes/essai-copy-apres-insertions.cpp` de la session cœur C++). Aucune erreur, 120 lignes au compte. Avec une seule ligne locale au lieu de vingt, la recherche par la clé 10 plante (`NodeGroup::lookup`).

L'ordre inverse (`COPY`, puis `CREATE`, puis les recherches) est juste : le stockage local est créé après le `COPY`, ses décalages provisoires commencent après ses lignes.

**Les tables de relations ne sont pas touchées**, sur ce qui a été joué (`annexes/essai-relations-merge-puis-copy.cpp`) : un petit lot par `UNWIND … MERGE` puis un gros par `COPY` dans la même table de relations et la même transaction rend les bons comptes dans les deux sens, les bonnes extrémités et propriétés, et les bons `SET`, avant et après la validation — nœuds validés avant, nœuds copiés dans la transaction, et mêmes nœuds d'origine dans les deux lots. Joué aussi, à la demande de la session de l'arbre principal (`annexes/essai-relations-vers-des-noeuds-de-la-transaction.cpp`) : des nœuds créés par `UNWIND … MERGE` dans la transaction, puis 600 relations par `COPY` vers eux, les deux bouts pouvant être tout juste créés — juste, dans la transaction comme après la validation. Non joué : des suppressions de relations (rag3weaver n'en fait pas dans ses groupes de paquets), et une réouverture après ces formes.

## Témoin

`test/transaction/journaled_copy_test.cpp`, `CopyAfterInsertsTest`, sept cas, chacun sous les deux réglages du chargement journalisé : la forme du défaut (vingt lignes insérées, cent copiées ; la clé, le `SET` et les relations, créées avant comme après le `COPY`, tombent sur leurs lignes — dans la transaction, après la validation, après la réouverture) ; une seule ligne locale (l'ancien plantage) ; une clé du `COPY` déjà insérée par la transaction (refus pour clé en double, rien ne reste, clés libres) ; une ligne locale modifiée et une autre supprimée avant le `COPY` ; des insertions après le `COPY`, un second `COPY`, d'autres insertions ; `ROLLBACK` puis la même suite validée ; et la mort base ouverte, par le journal seul. Sur une table à index vectoriel : `VectorIndexAfterFailedCopyTest.InsertsThenACopyInOneTransactionAreAllIndexed` (chemin validé ; le chemin annulé est le ticket `2026-10-05-copy-annule-sur-une-table-a-index-vectoriel.md`, qui existe sans ce cas). Les quatre cas Cypher d'origine (`transaction/copy/copy_node`, `InsertAndCopy…`) ont repris leur forme, avec une recherche par clé en plus. Au banc : `CopyAfterLocalInserts` (`upstream_fixes_test.cpp`), qui dit l'invariant et reste vert.

## Cause

Deux espaces de décalages qui se recouvrent dans la même transaction. Défaut d'origine : Vela (`vela/master`) et Ladybug (`ladybug-main-2026-08-31`) portent le même `getMinUncommittedNodeOffset`.

## Le correctif (5 octobre 2026)

Au début d'un `COPY` de nœuds, les lignes locales que la transaction a dans cette table sont versées dans la table (`LocalStorage::flushNodeTable`) : c'est le commit d'une table (`NodeTable::commit`), appelé plus tôt — ajout aux groupes sous l'identité de la transaction, suppressions appliquées, clés inscrites à l'index, lignes écrites au journal de la transaction (donc avant celles du `COPY`, dans l'ordre des décalages), relations locales remappées. Elles deviennent des lignes non validées ordinaires, annulables par le chemin qui annule un `COPY`. Le stockage local, vidé, repart de la fin de la table à sa prochaine insertion ; vide, il ne désigne plus aucun décalage.

Ce que le correctif ne prouve pas : le remappage des relations locales en cours de transaction sous plusieurs écrivains (témoin attendu de la marche A3′, écrit dans le code).

## La décision (orchestration, 4 octobre 2026)

Un refus qui retire un usage vaut mieux qu'un résultat faux validé en silence : le refus nommé d'abord, puis le vrai correctif juste après le chargement journalisé des relations — au début d'un `COPY`, verser dans la table les lignes locales de la transaction (ce que `NodeTable::commit` fait à la validation, remappage des relations locales compris), ou décaler les décalages provisoires au-delà de ce que le `COPY` écrit. Une journée de session et trois ou quatre passes, sur le terrain du remappage (marche A2). Le correctif retire le refus et rend les quatre cas Cypher à leur forme d'origine.

Côté rag3weaver : depuis `41136459a`, une table de nœuds écrite par `MERGE` dans une transaction ne repasse plus par `COPY` avant la validation. Une base produite en transaction par paquet à K > 1 avant cette règle est suspecte si un repli sur `MERGE` y a eu lieu.
