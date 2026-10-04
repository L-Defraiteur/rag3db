# Après un COPY annulé sur une table à index vectoriel, la recherche échoue, puis le COPY suivant n'est pas indexé

- **État** : corrigé le 5 octobre 2026 (« fix(index vectoriel): l'annulation prévient les index — après un COPY annulé, l'index ne se croit plus en avance sur sa table »). Une base abîmée avant le correctif est refusée par son nom (voir plus bas) et se rebâtit
- **Gravité** : réponse fausse (des lignes absentes de l'index), précédée d'un blocage (la recherche échoue)
- **Atteignable en service** : oui — un seul écrivain, une transaction explicite, une table déjà indexée
- **Touche rag3weaver** : oui (relevé de l'arbre principal, par lecture) — l'index est créé sur chaque table de morceaux avant les lignes, les morceaux arrivent par `COPY` à chaque paquet, et le chemin d'échec d'un paquet annule ces `COPY`
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

`test/transaction/vector_index_rollback_test.cpp`, cinq cas ; une ligne est « dans l'index » quand la recherche de son vecteur exact la rend en tête :
- un `COPY` annulé : chaque ligne ancienne se retrouve (les arêtes de retour vers les lignes annulées n'ont rien abîmé), aucune recherche ne rend une ligne annulée ni n'échoue ; de même après un point de reprise et une réouverture ;
- un `COPY` annulé puis le même validé : chaque ligne copiée se retrouve ;
- sur une table vide (les points d'entrée étaient des lignes annulées) ;
- des insertions ordinaires puis un `COPY`, annulés, puis validés ;
- la mise à jour de vecteurs, annulée.

Le refus nommé du filet n'a pas de témoin automatique : il faut une base abîmée par l'ancien moteur.

## Le correctif (5 octobre 2026)

L'annulation des lignes d'une table prévient ses index (`Index::rollbackInsert`, appelé par `NodeTableVersionRecordHandler::rollbackInsert`). L'index vectoriel y ramène son compte à la première ligne retirée et oublie un point d'entrée qui désignait une ligne retirée ; le code savait déjà repartir d'une ligne vivante sans point d'entrée. Rien n'est à retirer du graphe : les arêtes posées par la transaction vivent dans ses tables de relations locales, que l'annulation défait — c'est ce que prouve le témoin des lignes anciennes.

**Une base abîmée avant le correctif** (le compte en avance a pu être écrit par un point de reprise, par exemple à la fermeture après un paquet annulé) : toute recherche, mise à jour de vecteur, insertion ou suppression dans l'index, et toute fin de `COPY`, refuse sous le nom `is behind its table` :

> Index X is behind its table T: it counts N indexed rows and the table holds M. A load into the table was rolled back after the index had counted its rows. Drop it and build it again.

C'est le fragment que rag3weaver reconnaît déjà pour rebâtir un index. Le filet ne voit que le compte **au-delà** du nombre de lignes : un compte en avance masqué par des lignes ajoutées depuis (le second effet de la recette) ne se détecte pas — ces lignes manquent à l'index sans erreur, et seul un rebâti les y remet.

**Non établi** : le plantage vu par l'arbre principal (reprise après un paquet annulé puis une fermeture, `SIGSEGV` dans `shrinkForNode` sous une mise à jour de vecteur). Sur une copie de sa base, une mise à jour de vecteur par table passe sans que le filet parle : ce plantage n'est peut-être pas le compte en avance. À rejouer par sa sonde sur ce correctif.
