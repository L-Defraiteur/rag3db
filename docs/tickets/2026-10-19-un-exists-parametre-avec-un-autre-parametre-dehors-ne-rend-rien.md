# Un EXISTS paramétré, avec un autre paramètre hors de lui, ne rend rien

- **État** : ouvert — trouvé le 19 octobre 2026 par l'arbre principal, en écrivant l'IR du Plan déclaré (`through` rendu en `EXISTS { MATCH … }`). Pour le cœur C++.
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

Inconnue. Soupçon : la liaison des paramètres d'une sous-requête `EXISTS` ne voit pas, ou
écrase, ceux de la requête qui l'entoure, quand les deux en ont.

## Pour le fermer

Un correctif dans le moteur, avec son témoin C++. Côté produit, `through` ne se rend en
`EXISTS` que lorsque le correctif est sur la lib `courant` ; d'ici là, le lot de l'IR du Plan
le dit dans son témoin, et `cited_by` ne doit pas combiner un `through` filtré et une autre
condition paramétrée.
