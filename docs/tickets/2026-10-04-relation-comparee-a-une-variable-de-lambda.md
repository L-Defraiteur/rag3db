# Comparer une relation à une variable de lambda fait planter

- **État** : ouvert
- **Gravité** : plantage
- **Atteignable en service** : oui
- **Touche rag3weaver** : non vérifié
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : binder (banc, périmètre levé le 4 octobre)

## Ce que c'est

`r <> t` où r est une variable de lambda, ou label() sur un élément de nodes(p), font une conversion fautive et tuent le processus.

## Recette minimale

```cypher
CREATE NODE TABLE person(ID INT64 PRIMARY KEY);
CREATE REL TABLE knows(FROM person TO person);
UNWIND range(0, 3) AS i CREATE (:person {ID: i});
MATCH (a:person), (b:person) WHERE a.ID <> b.ID CREATE (a)-[:knows]->(b);
MATCH (a:person)-[t:knows]->(d:person) WITH a, d, t
MATCH p = (a)-[:knows*1..3]->(d) WHERE length(p) >= 2 AND ALL(r IN relationships(p) WHERE r <> t)
WITH DISTINCT t RETURN count(t);   -- SIGSEGV
MATCH p = (n)-[:knows]->(m:person {ID: 2}) WITH nodes(p) AS ns RETURN label(ns[1]);   -- SIGSEGV
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — RelationComparedToALambdaVariable

## Cause

`src/binder/bind_expression/bind_comparison_expression.cpp:42-46`, `src/function/pattern/label_function.cpp:91` : conversion vers NodeOrRelExpression d'une expression qui n'est pas un motif.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `0f3f03d63`, `57dc0b4ed` (21-22 juin 2026).

## Pour le fermer

le témoin passe au vert.
