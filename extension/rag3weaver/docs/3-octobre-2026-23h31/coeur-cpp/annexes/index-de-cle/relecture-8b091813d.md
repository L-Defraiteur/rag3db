# Relecture de 8b091813d (banc-verrous) — fenêtre A du DROP d'une colonne déclarée avant la clé

Relecture indépendante par un agent, à la demande de la session cœur C++, le 5 octobre 2026.
Lecture seule : rien bâti, rien exécuté. Tout vient de la lecture du code au commit ; les
scénarios sont inférés, non rejoués. La session cœur C++ n'a pas revérifié ces constats.

Note : l'arbre de travail de rag3db-banc n'était pas au commit pendant la lecture (les
fichiers src du correctif revenus à l'état d'avant, plus une sonde non commitée). Un bâti
fait là à ce moment ne testait pas le correctif.

## Bloquant

**1. `LocalRelTable::addColumn` n'agrandit pas la table de conversion — régression, sans aucun DROP.**
`src/storage/local_storage/local_rel_table.cpp:165-169`. `positionOfColumn` est dimensionné
une fois au constructeur (l.36-39, `getMaxColumnID() + 1`) ; `LocalNodeTable::addColumn`
l'agrandit, pas `LocalRelTable::addColumn`. `rewriteLocalColumnID` (l.326-338) lit
`positionOfColumn[columnID]` derrière un `KU_ASSERT`, inactif en Release. La colonne ajoutée
reçoit un numéro au moins égal à l'ancienne taille : lecture hors bornes. Avant le commit :
`columnID + 1`, juste dans ce cas.
Scénario : `BEGIN; MATCH (a:P{id:1}),(b:P{id:2}) CREATE (a)-[:R {b:'v'}]->(b); ALTER TABLE R
ADD d INT64 DEFAULT 7; MATCH ()-[r:R]->() RETURN r.d;` (ou `SET r.d = 1`). Passe par
`RelTableScanState::setToTable` (rel_table.cpp:52-55) ou `LocalRelTable::update` (l.109-113).
Témoin à écrire : ce scénario, lecture puis SET, sans DROP.

## À mesurer avant de pousser

**2. Une recherche au catalogue et une copie des définitions par LIGNE insérée.**
`insert_executor.cpp:109-111` appelle `initInsertState` par ligne ; depuis le commit
(`node_table.cpp:499-515`) il fait par ligne `Catalog::getTableCatalogEntry` (parcours
linéaire sous verrou partagé, `catalog_set.cpp:223-237`) et un `getPropertyPosition` par
colonne d'index, qui recopie toutes les définitions (`table_catalog_entry.cpp:93-104`).
Mesure : `UNWIND range(0, 999999) AS i CREATE (:T {...})` sur une table d'une trentaine de
colonnes, avant et après. Piste : poser les positions une fois (dans
`NodeTableInsertInfo::init`), et écrire `getPropertyPosition` sans copie.

**3. Même coût dans `skipInsert`** (`insert_executor.cpp:122-124`), par ligne sautée d'un
MERGE. La conversion elle-même est du bon côté.

## À suivre (avec la fenêtre B)

**4.** `local_node_table.cpp:35-38` (et `local_rel_table.cpp:36-39`) : n appels qui copient
chacun n définitions, à la première insertion de chaque transaction. Une passe sur
`getProperties()` suffit.

**5. `NodeTable::commit` et `RelTable::commit` non convertis** (`node_table.cpp:745-756`,
`rel_table.cpp:453-458`) : l'ordre des colonnes vient du catalogue AU COMMIT, les groupes
locaux sont rangés selon le catalogue à la création de la table locale. Les deux diffèrent
si le DROP a lieu dans la transaction après des lignes locales. `KU_ASSERT` en Debug
(`node_group_collection.cpp:71`), colonnes décalées en Release. Antérieur au commit, même
famille. Témoin : `BEGIN; CREATE (:Item {extra, id, name}); ALTER TABLE Item DROP extra;
CREATE (:Item {id, name}); COMMIT;` puis lecture par clé, puis réouverture.

**6.** `LocalNodeTable::addColumn` (`local_node_table.cpp:140-147`) prend le numéro de
`columns.size() - 1`, pas du catalogue. Cohérent pour ADD seul, ADD après un DROP validé,
ADD après le DROP de la dernière colonne. Inféré, non vérifié : après un ADD annulé,
`columns` garderait sa colonne ; l'ADD suivant divergerait. Témoin possible : `BEGIN; ALTER
ADD c; ROLLBACK; BEGIN; CREATE …; ALTER ADD d; CREATE …; MATCH … RETURN n.d`.

**7.** Pas de garde en Release : `getPropertyPosition` peut rendre `INVALID_IDX`,
`propertyVectors[position]` (`node_table.cpp:541`) n'est gardé par rien ; `getLocalColumnID`
n'a qu'un `KU_ASSERT` ; `node_table.cpp:765` calcule `getLocalColumnID(pkColumnID)` à chaque
commit — en fenêtre B, avec un `pkColumnID` périmé et hors table, lecture hors bornes. À
traiter avec la fenêtre B par une exception nommée.

**13.** `getLocalColumnIDs` alloue un vecteur à chaque `initScanState` UNCOMMITTED.

## Rien de faux trouvé

**8. Le sens des conversions** : `initScanState`, `scanIndexColumns`, `NodeTable::insert`,
`NodeTable::update`, `getStats`, `NodeBatchInsert`, le binder — tous du bon côté.
**9. Sites oubliés en fenêtre A** : aucun. Tout balayage UNCOMMITTED de nœuds passe par
`NodeTable::initScanState` ; l'index vectoriel et FTS lisent une ligne locale par
`setToTable` + `initScanState`. Fenêtre B, comme annoncé : `indexInfo.columnIDs`
(hnsw_index.cpp:488-991, fts_index.cpp:141-213) et `pkColumnID` (node_table.cpp:293, 596,
765).
**10. Relations** : `2 + position` cohérent partout, sauf constats 1 et 5.
**11. Journal** : forme inchangée ; un enregistrement d'avant se rejoue pareil sur une table
sans DROP. Deux écarts au rejeu : une recherche au catalogue par enregistrement, qui lève si
la table n'y est pas ; et le cas du constat 5.

## Tests

**12.** Le commit ne touche aucun .cpp de test, seulement known_red.txt (10 lignes
retirées). Le message dit « dix des douze Cases » : c'est 8 des 12 Cases plus les 2 Copy,
10 des 14 témoins. Aucun EXPECT_EQ sur un grand texte. Annoncé sans témoin : le MERGE d'une
ligne de la transaction (`skipInsert`) ; la clé des suppressions locales au commit ; la
fusion des statistiques ; un index secondaire en fenêtre A ; ADD après DROP avec des lignes
locales ; les scénarios des constats 1 et 5.

**Verdict du relecteur : bloquant pour pousser — oui**, à cause du constat 1 (régression
hors de tout DROP, corrigible en quelques lignes avec son témoin) ; le constat 2 est à
mesurer avant de pousser ; le reste peut suivre avec la fenêtre B.
