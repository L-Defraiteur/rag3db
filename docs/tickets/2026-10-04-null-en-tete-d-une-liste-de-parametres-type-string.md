# Un NULL en tête d'une liste de paramètres type sa colonne en STRING

- **État** : ouvert — la cause est dans rag3weaver (lue le 10 octobre 2026), pas dans le moteur ; contourné dans rag3weaver (l'écrivain de liens ne
  regroupe plus que les propriétés non nulles).
- **Gravité** : perte (les écritures du lot entier sont refusées ; rag3weaver les comptait
  sans les dire).
- **Atteignable en service** : oui.
- **Touche rag3weaver** : oui, par `LinkRecordNode` (le chemin par `UNWIND`, sous le seuil
  du COPY).

## Ce que c'est

Dans une liste de maps passée en paramètre, une clé dont la **première** valeur est NULL
prend le type STRING pour toute la liste, et une colonne typée (INT64 ici) refuse alors le
lot entier : `Binder exception: Expression STRUCT_EXTRACT(item,line) has data type STRING
but expected INT64. Implicit cast is not supported.`

## Recette

Par paramètres (la forme littérale n'a pas été essayée) :

```cypher
CREATE NODE TABLE A(id INT64 PRIMARY KEY);
CREATE REL TABLE R(FROM A TO A, line INT64);
CREATE (:A {id: 1}), (:A {id: 2}), (:A {id: 3});
-- $items = [{a: 1, b: 2, line: NULL}, {a: 1, b: 3, line: 7}]
UNWIND $items AS item
MATCH (x:A {id: item.a}), (y:A {id: item.b})
MERGE (x)-[r:R]->(y) SET r.line = item.line;
```

Attendu : deux relations, `line` NULL puis 7. Obtenu : l'erreur du binder, aucune relation.

## Témoin

`extension/rag3weaver/tests/e2e_copy_liens.rs`, `un_null_en_tete_d_un_lot_ne_fait_pas_tomber_le_lot` :
un lot de trois liens dont le premier et le dernier n'ont pas de ligne. Rouge sans le
contournement (vérifié le 4 octobre 2026), vert avec.

## Comment il a été trouvé

Une méthode C++ définie hors de sa classe (`int Foo::bar() {…}` dans foo.cpp) ne se
reliait jamais à sa classe. Son rendez-vous `HAS_PARENT` n'a pas de site d'usage, donc
pas de ligne ; trié en tête du lot de `MENTIONS`, il faisait refuser les cinq rendez-vous
du fichier, `#include` compris. Le rapport d'ingestion portait `failed: 5`, que personne
ne lisait. Sur master avant le contournement, des `MENTIONS` ont donc pu manquer dans tout
index où un rendez-vous sans ligne ouvrait un lot.

## La cause (lue le 10 octobre 2026, seconde session cœur C++) — dans rag3weaver, pas dans le moteur

- `cypher_to_rag3db_value` (`extension/rag3weaver/src/rag3db_connection.rs:663-688`), qui
  convertit les paramètres des requêtes préparées :
  - un `NULL` non typé devient un `NULL` de type `STRING` (`:666`, et un test l'affirme, `:1007`) ;
  - le type des éléments d'une liste est pris sur le **premier** élément seul (`:672-678`) ;
  - une map devient une structure dont chaque champ porte le type de sa propre valeur (`:680-686`).
  `[{line: NULL}, {line: 7}]` devient donc `LIST(STRUCT(line STRING))`.
- Le moteur fait ce qu'on lui demande : la liaison Rust passe les types tels quels
  (`tools/rust_api/src/value.rs:729`, `:788-799`), `Value(type, children)` ne les vérifie pas
  (`value.cpp:334-338`), et un paramètre prend le type de sa valeur
  (`bind_parameter_expression.cpp:16-17`) ; il ne se laisse retyper que s'il contient `ANY`
  (`ParameterExpression::cast`, `parameter_expression.cpp:10-20`), d'où le refus du binder.
- La voie typée de rag3weaver (`CypherValue::Typed`, `typed_rag3db_value`, `:704-712`) donne au
  `NULL` le type déclaré du champ : elle n'a pas le défaut.

## Pour le fermer

Dans rag3weaver (chantier de l'arbre principal) : unifier le type de chaque champ sur toute la
liste (un `NULL` ne contraint rien), ou typer un `NULL` non typé en `LogicalType::Any` — la
liaison Rust le prévoit (`tools/rust_api/src/logical_type.rs:9-10`, « Special type for use with
Value::Null ») ; non vérifié : qu'un champ `ANY` dans une structure se laisse retyper par le
binder (le témoin le dira). Ou passer par la voie typée. Côté moteur, rien n'est faux ; une
unification des types des éléments d'une liste de paramètres serait un confort, non demandé. Le contournement de rag3weaver peut alors rester (il
ne coûte que des groupes de plus quand des lignes manquent) ou partir.
