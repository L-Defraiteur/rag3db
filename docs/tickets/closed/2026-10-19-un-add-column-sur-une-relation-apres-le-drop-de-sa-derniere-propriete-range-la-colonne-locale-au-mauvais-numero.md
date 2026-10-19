# Un ADD COLUMN sur une relation, après le DROP de sa dernière propriété, range la colonne locale au mauvais numéro

- **État** : corrigé le 19 octobre 2026 par le commit qui déplace ce fichier (`git log --follow`),
  V2-1a : la table locale range la colonne ajoutée à son numéro (`LocalRelTable::addColumn`) ;
  témoin `StableColumnIDs.AnAddOnARelationAfterDroppingItsLastPropertyWritesTheRightColumn`
- **Gravité** : si confirmé, une valeur écrite dans la même transaction que l'ADD va dans une
  autre colonne locale, ou hors du vecteur (lecture après la fin).
- **Atteignable en service** : oui si confirmé : DROP d'une propriété de relation, puis, avant un
  point de reprise, ADD d'une propriété et écriture d'une relation dans la même transaction.
- **Touche rag3weaver** : oui si confirmé.
- **Ouvert le** : 19 octobre 2026, session cœur C++.
- **Pour** : cœur C++, lot V2-1a.

## Ce que c'est

`LocalRelTable::positionOfColumn` est dimensionné à `getMaxColumnID() + 1`
(`src/storage/local_storage/local_rel_table.cpp:38-41`). `LocalRelTable::addColumn` (:167-175)
fait `positionOfColumn.push_back(...)` : la nouvelle position est rangée à l'indice
`maxColumnID + 1`. Or le numéro que le catalogue donne à la colonne ajoutée est `nextColumnID`,
qui ne recule pas à un DROP (`property_definition_collection.cpp:61-67`). Si la dernière
propriété a été supprimée sans point de reprise depuis, `nextColumnID > maxColumnID + 1` : la
colonne ajoutée est rangée sous le numéro de la colonne supprimée, et une lecture par son vrai
numéro tombe hors du vecteur. `LocalNodeTable::addColumn` fait un `resize(columnID + 1)`
(`local_node_table.cpp:151-153`) mais prend `columnID = NodeTable::getNumColumns() - 1`, qui
suppose `columns.size() == nextColumnID` : même famille.

## Recette (lue, pas jouée)

```cypher
CREATE NODE TABLE A(id INT64 PRIMARY KEY); CREATE NODE TABLE B(id INT64 PRIMARY KEY);
CREATE REL TABLE R(FROM A TO B, p INT64, q INT64);
CREATE (:A {id: 1}); CREATE (:B {id: 2});
ALTER TABLE R DROP q;             // la dernière propriété ; pas de CHECKPOINT
BEGIN TRANSACTION;
ALTER TABLE R ADD r INT64;
MATCH (a:A), (b:B) CREATE (a)-[:R {p: 1, r: 7}]->(b);
MATCH ()-[x:R]->() RETURN x.p, x.r;   // attendu 1, 7
COMMIT;
```

## Témoin

À écrire dans `test/transaction/`, sous ASAN (une lecture hors du vecteur peut passer en
Release).

## Pour le fermer

Avec V2-1a, les numéros sont stables et la table locale range la colonne ajoutée à son numéro
(`resize(columnID + 1)` puis affectation), nœuds et relations. Le témoin passe.
