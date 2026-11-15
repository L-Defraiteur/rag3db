# Un crochet pour abîmer un seul rangement d'une relation (souhait)

- **État** : corrigé (15 novembre 2026, 84c772e60) : le point de faute `rel-commit-between-directions` du cœur, et une base témoin fabriquée avec lui.
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

## Ce que le cœur a posé (15 novembre 2026)

- **Le crochet** : le point de faute `rel-commit-between-directions` (84c772e60). Il fait lever `RelTable::commit` une fois,
  après le premier sens et avant le second. Il s'arme par `RAG3DB_TEST_HOOKS=1 RAG3DB_TEST_FAIL_AT=rel-commit-between-directions`,
  ou par `TestHooks::armFailPoint` dans un témoin C++. Sous la lib posée, l'annulation se fait sous le verrou et rend les
  deux sens entiers. Une base à un seul sens ne se fabrique que sous une lib d'avant 373b5477b.
- **La base témoin** : à luciepc, `~/.cache/rag3weaver-build/bases-temoins/un-seul-sens/`, écrite sous f41c0ba5b, avec son
  `ATTENDU.txt`. Elle compte 4 200 nœuds N et 12 relations R. 1:60 et 1:111 n'y sont lisibles qu'au sens arrière.
  `check_rel_directions` de master la lit sous 373b5477b en 0,15 s : avant 10, arrière 12, union 12, perdues 2.
- **Sa limite** : l'outil d'avant voit lui aussi ces deux relations, parce que sa lecture non ancrée y joue le sens
  arrière, celui qui a tout. Elle sert donc de témoin exact des comptes, pas de rouge de l'ancrage. Le miroir (une
  relation présente au sens avant et absente du sens arrière) n'a pas été obtenu : sous le refus en fin de COMMIT, la
  relation se perd des deux côtés, par un mécanisme que le cœur porte au ticket des deux index. Le rouge de l'ancrage
  reste joué sur une copie de code-principal.
