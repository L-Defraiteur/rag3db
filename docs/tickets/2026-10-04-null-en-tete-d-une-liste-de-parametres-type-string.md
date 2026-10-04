# Un NULL en tête d'une liste de paramètres type sa colonne en STRING

- **État** : ouvert dans le moteur ; contourné dans rag3weaver (l'écrivain de liens ne
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

## La cause

Non lue dans le binder. Le comportement observé : le type d'un champ de la liste suit sa
première valeur, et NULL vaut STRING. Le chemin d'annulation l'avait déjà rencontré le
3 octobre (`_absent_since`, INT64 ; `colonnes_toutes_nulles` dans `record_nodes.rs`).

## Pour le fermer

Que le binder unifie le type d'un champ sur toute la liste (NULL ne contraint rien), ou
type un NULL de paramètre comme ANY. Le contournement de rag3weaver peut alors rester (il
ne coûte que des groupes de plus quand des lignes manquent) ou partir.
