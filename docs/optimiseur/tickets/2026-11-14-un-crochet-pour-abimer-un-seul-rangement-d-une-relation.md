# Un crochet pour abîmer un seul rangement d'une relation (souhait)

- **État** : ouvert (14 novembre 2026), souhait ; pas avant que le cœur ait la place.
- **Famille** : moteur
- **Gravité** : coût ; sans lui, le témoin de `check_rel_directions` dépend d'une base réelle abîmée, qui n'existe qu'à luciepc.
- **Atteignable en service** : non, c'est un crochet de test.
- **Touche rag3weaver** : non.
- **Pour** : cœur c++.
- **Ouvert le** : 14 novembre 2026

## Ce que c'est

Le témoin du contrôle des deux sens (`tools/check_rel_directions/test_check_rel_directions.sh`) doit voir une relation
lisible par l'ancrage d'un seul sens et cachée à la lecture non ancrée. Rien en Cypher ne fabrique cet état : il faut
qu'un seul des deux groupes CSR d'une relation la perde, ou la rattache au mauvais nœud. Aujourd'hui, le témoin joue sur
une copie jetable de code-principal (instantané `avant-373b5477b`), lancé par son nom, hors de tout jeu automatique
(décision de c7).

## Ce qu'il faudrait

Un crochet de test du moteur, comme `FailedCheckpointHook`, qui retire une relation d'un seul rangement, ou la déplace
d'un cran dans une colonne. Le témoin bâtirait alors sa base lui-même, dans un dossier temporaire, et entrerait dans un
jeu automatique.
