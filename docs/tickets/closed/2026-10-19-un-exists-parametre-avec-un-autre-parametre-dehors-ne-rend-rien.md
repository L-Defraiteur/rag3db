# Un EXISTS paramétré, avec un autre paramètre hors de lui, ne rend rien

- **État** : corrigé le 19 octobre 2026 par le commit qui déplace ce fichier (`git log --follow`) — trouvé le 19 octobre 2026 par l'arbre principal, en écrivant l'IR du Plan déclaré (`through` rendu en `EXISTS { MATCH … }`). Pour le cœur C++.
- **Gravité** : réponse fausse, en silence : zéro ligne, sans erreur.
- **Atteignable en service** : oui, dès qu'un Plan déclaré combine un `through` filtré et une autre condition paramétrée (`cited_by` de la mémoire : la Ref qui répond, plus `kinds`, `states` ou `subject`).
- **Touche rag3weaver** : oui (le rendu de `Predicate::Through` dans le dialecte rag3db).

## Ce que c'est

Dans une même clause `WHERE`, un sous-`EXISTS { MATCH … WHERE … }` qui lit un paramètre, et une
condition hors de lui qui lit un **autre** paramètre : la requête ne rend aucune ligne, et un
`count` y rend 0. Chacun des deux, seul, est juste. Le même `EXISTS` sans paramètre, avec le
paramètre dehors, est juste aussi.

## La recette

Moteur lib 4f9274a38 (`RAG3DB_LIB`), une base neuve :

```cypher
CREATE NODE TABLE Memory(_uuid STRING, kind STRING, PRIMARY KEY(_uuid));
CREATE NODE TABLE Ref(_uuid STRING, valeur STRING, PRIMARY KEY(_uuid));
CREATE REL TABLE CITE(FROM Memory TO Ref);
CREATE (:Memory {_uuid: 'f2', kind: 'rule'});
CREATE (:Ref {_uuid: 'r2', valeur: 'd:src/a.rs#m::f'});
MATCH (m:Memory {_uuid: 'f2'}), (r:Ref {_uuid: 'r2'}) CREATE (m)-[:CITE]->(r);
```

Puis, avec `$value_dir = 'd:src/'` et `$k = 'rule'` :

| requête | rendu | attendu |
|---|---|---|
| `MATCH (m:Memory) WHERE EXISTS { MATCH (m)-[:CITE]->(t:Ref) WHERE t.valeur STARTS WITH $value_dir } AND m.kind = $k RETURN m._uuid` | aucune ligne | `f2` |
| la même, `'d:src/'` écrit en littéral dans l'EXISTS | `f2` | `f2` |
| la même, `$k` remplacé par `'rule'` | `f2` | `f2` |
| `MATCH (m:Memory) WHERE EXISTS { … $value_dir … } RETURN count(m)` | juste | — |
| `MATCH (m:Memory) WHERE m.kind = $k AND m._uuid >= $x RETURN m._uuid` (deux paramètres, sans EXISTS) | juste | — |

Le dehors peut être `=`, `IN $liste` ou `list_contains($liste, …)`, et l'ordre des deux
conditions n'y change rien : vu sur les quatre formes. Un `WITH m` avant le `count` n'y change
rien non plus.

## Le témoin

`plan_declare_tests::isoler_le_compte_sous_une_liste_en_parametre`, ignoré, dans le lot de l'IR
du Plan (`extension/rag3weaver/src/plan_declare_tests.rs`). Il joue chaque variante sur la
connexion, sans le Plan, et imprime ce qu'elle rend. Un témoin C++ déterministe reste à écrire
côté moteur (`test/` : la recette ci-dessus, et l'attendu exact).

## La cause

La préparation, pas la liaison. À la préparation (sans valeurs), `ExpressionBinder::bindExpression`
passe un `ParsedParamExprCollector` sur chaque expression : s'il y trouve un paramètre inconnu,
l'expression entière devient un paramètre NULL sans être liée, et les noms vus sont enregistrés
comme inconnus, que l'exécution exige. Or `ParsedExpressionVisitor::visitChildren` ne visitait
que `getChild(i)` : les motifs et la clause `WHERE` d'une `ParsedSubqueryExpression` ne sont pas
des enfants. Avec `$k` dehors et `$value_dir` dedans, le second n'était jamais enregistré ;
l'exécution l'ignorait (ni connu ni inconnu), et le rebind, liant cette fois l'`EXISTS`, le
remplaçait par NULL : `STARTS WITH NULL`, aucune ligne, aucune erreur. Un seul paramètre
dedans (l'`EXISTS` est lié à la préparation, son `WHERE` enregistre le paramètre), deux
paramètres sans sous-requête (tous deux enfants), ou deux `EXISTS` paramétrés sans rien dehors,
passaient par construction.

Correctif : `visitChildren` descend dans une sous-requête (propriétés de ses motifs, `WHERE` et
projections d'une relation récursive, clause `WHERE`) ; tous les visiteurs en profitent.

## Le témoin du moteur

`test/api/prepare_subquery_parameters_test.cpp` (`PreparedSubqueryParameters.*`) : la recette
rejouée trois fois avec d'autres valeurs, le paramètre de dedans oublié refusé par son nom (il
était accepté en silence), le paramètre dans un motif `{valeur: $v}` de l'`EXISTS`, la
sous-requête `COUNT`, et les trois formes qui passaient déjà. Rouge sur master 60348d75a (4 sur
5), vert sur le correctif.

## Ce qu'il fallait pour le fermer

Un correctif dans le moteur, avec son témoin C++ : fait. Côté produit, `through` ne se rend en
`EXISTS` que lorsque le correctif est sur la lib `courant` ; d'ici là, le lot de l'IR du Plan
le dit dans son témoin, et `cited_by` ne doit pas combiner un `through` filtré et une autre
condition paramétrée.
