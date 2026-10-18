# Une colonne ajoutée puis annulée reste au stockage, et l'ajout suivant la relit à sa place

- **État** : ouvert (lu dans le code, pas encore joué : le banc le joue dans le scénario des
  migrations concurrentes, minute 6 suivie d'un `ADD b`)
- **Gravité** : réponse fausse silencieuse (la seconde colonne rend la valeur par défaut de la
  première), plantage probable si les deux types diffèrent ; **rendue permanente par le point de
  reprise**, qui garde la colonne annulée et jette la vraie.
- **Atteignable en service** : oui, par une migration qui échoue après un `ADD` et se rejoue
  (étendre puis contracter : l'`ADD` est la première instruction de toute migration).
- **Touche rag3weaver** : oui, dès que l'hôte joue des migrations dans une transaction et les rejoue
  après un échec.
- **Ouvert le** : 18 octobre 2026, session cœur C++, d'après l'étude des migrations concurrentes
  (`extension/rag3weaver/docs/coeur-cpp/15-migrations-concurrentes-la-part-moteur.md` §1, trou 1 ;
  E10 de la page de l'arbre principal `13-octobre-2026-arbre-principal/01-plusieurs-developpeurs-et-leurs-migrations.md`).
- **Pour** : cœur C++.

## Ce que c'est

Le catalogue est versionné et s'annule ; le stockage des colonnes ne s'annule pas. Un `ADD` annulé
laisse sa colonne au stockage, et l'`ADD` suivant reçoit du catalogue l'indice de cette colonne
morte : il la relit, avec sa valeur par défaut et son type, à la place de la sienne.

## Recette (lue, pas jouée)

```cypher
CREATE NODE TABLE T(id INT64, PRIMARY KEY(id));
CREATE (:T {id: 1});
BEGIN TRANSACTION;
ALTER TABLE T ADD b INT64 DEFAULT 1;
ROLLBACK;
ALTER TABLE T ADD b INT64 DEFAULT 2;
MATCH (t:T) RETURN t.b;
```

Attendu `2`. D'après le code : `1`, la valeur par défaut de la colonne annulée. Avec
`ADD b STRING DEFAULT 'deux'` au second `ADD`, le balayage construit son vecteur de sortie avec le
type de la colonne du stockage (`INT64`) là où le plan attend une chaîne : assertion en debug,
valeur fausse ou plantage en release. Le nom ne compte pas : `ADD c` après un `ADD b` annulé relit
de même la colonne de `b`.

Après un `CHECKPOINT`, la table ne garde que les colonnes que le catalogue nomme : la colonne
annulée (indice réutilisé) est gardée, la vraie colonne du second `ADD` est jetée. Le défaut ne se
répare plus.

## Témoin

Aucun encore. Le banc le joue dans le scénario des migrations concurrentes
(`extension/rag3weaver/docs/13-octobre-2026-arbre-principal/01-plusieurs-developpeurs-et-leurs-migrations.md`,
E10). À écrire aussi en témoin court dans `test/transaction/` : la recette ci-dessus, puis la même
avec deux types différents, puis la relecture après `CHECKPOINT` et après réouverture.

## Cause

- `CatalogSet::alterTableEntry` → `TableCatalogEntry::alter` copie l'entrée
  (`src/catalog/catalog_entry/table_catalog_entry.cpp:18`) ; la copie emporte `nextColumnID` et
  `PropertyDefinitionCollection::add` y prend l'indice `N` puis l'incrémente dans la copie seule
  (`src/catalog/property_definition_collection.cpp:56`). L'annulation retire la copie de la chaîne
  des versions (`UndoBuffer::rollbackCatalogEntryRecord`) : l'ancienne entrée a toujours
  `nextColumnID == N`, et le second `ADD` reprend `N`.
- `NodeTable::addColumn` (`src/storage/table/node_table.cpp:843`) fait `columns.push_back`, puis
  chaque `NodeGroup` et `ChunkedNodeGroup` empile un bloc rempli de la valeur par défaut, et
  `NodeGroupCollection::addColumn` empile le type. Rien n'est noté dans le tampon d'annulation : au
  `ROLLBACK`, `columns[N]` reste. Le second `ADD` empile en `N+1`, mais le catalogue envoie tous les
  balayages et toutes les écritures vers `N`. Même chose pour `RelTable::addColumn`
  (`src/storage/table/rel_table.cpp:470`).
- `NodeTable::checkpoint` (`src/storage/table/node_table.cpp:999`) reprend `columns[columnID]`
  pour chaque propriété du catalogue, donc `columns[N]`, et `vacuumColumnIDs` renumérote : la
  colonne `N+1` est perdue.

Ce trou se combine au trou 2 de la page 15 : deux `ADD` concurrents sur la même table ne se
voient pas (`alterTableEntry` ne détecte pas le conflit écrivain-écrivain) et reçoivent tous deux
l'indice `N`.

## Le correctif de l'amont

Le même code (ADD COLUMN sans annulation du stockage, `vacuumColumnIDs` au point de reprise) : rien
à lire chez lui pour ce cas.

## Pour le fermer

L'`ADD` d'une colonne laisse un enregistrement dans le tampon d'annulation ; au `ROLLBACK`, la
colonne est retirée du stockage (`columns`, les blocs de chaque groupe, le type et les statistiques
de la collection), dans l'ordre inverse des ajouts. Pour que « la dernière colonne » soit bien celle
de la transaction annulée, deux `ADD` sur la même table doivent se sérialiser : le conflit
écrivain-écrivain sur `alterTableEntry` (trou 2 de la page 15) se ferme dans le même lot, ou le
`ADD` prend le verrou INDEX exclusif de la table comme un DDL d'index. Fermé quand la recette rend
`2`, que la variante à deux types rend `'deux'`, et que les deux tiennent après `CHECKPOINT` et
réouverture.
