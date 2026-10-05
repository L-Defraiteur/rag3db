# Relecture de 6e7aeb813 (banc) — fenêtre B du DROP d'une colonne déclarée avant la clé

Relecture indépendante par un agent, à la demande de la session cœur C++, le 5 octobre 2026.
Lecture seule (`git show`) : rien bâti, rien exécuté. La session cœur C++ n'a pas revérifié ces
constats ; les lignes sont celles des fichiers au commit.

## Bloquant (sauf si ATTACH d'une base rag3db est tenu pour hors d'usage)

**B1. `ATTACH` d'une base rag3db : la réparation à l'ouverture interroge le catalogue de la
mauvaise base.** Vérifié par lecture, non exécuté.
- `src/storage/table/node_table.cpp:1164-1167` : `NodeTable::deserialize` appelle
  `Catalog::Get(*context)->getTableCatalogEntry(&DUMMY_CHECKPOINT_TRANSACTION, tableID)`, et
  `renumberColumns` (l.1173) refait `Catalog::Get(*context)->getIndexEntries(...)`.
- Sur le chemin ATTACH, `src/main/attached_database.cpp:63` passe son propre catalogue à
  `Checkpointer::readCheckpoint`, mais `Catalog::Get(context)` (`src/catalog/catalog.cpp:35-40`)
  rend encore celui de la base principale : `setDefaultDatabase` n'est appelé qu'après le
  constructeur (`src/processor/operator/simple/attach_database.cpp:35-37`).
- `StorageManager::deserialize` (`src/storage/storage_manager.cpp:296-299`) a le bon catalogue
  et la bonne entrée en main.
- Selon ce que la base principale porte sous le même `tableID` (les numéros partent de 0 dans
  les deux bases) : numéro absent → « Cannot find table catalog entry with id N », l'ATTACH
  échoue (le `if (const auto* entry = …)` de la l.1164 est une garde morte : la recherche lève) ;
  groupe de relations → `constCast` = `reinterpret_cast` en Release, comportement indéfini ;
  autre table de nœuds → `pkColumnID` et colonnes d'index recopiés d'une table étrangère.
- Scénario : depuis une base principale vide,
  `ATTACH '/chemin/base-avec-une-table-de-noeuds' AS autre (dbtype rag3db);`
- Aucun test n'attache une base rag3db ; rag3weaver n'utilise pas ATTACH.
- Correction petite : faire passer l'entrée et le catalogue depuis `StorageManager::deserialize`.

## À mesurer avant de pousser

Rien : le commit n'ajoute aucun coût par ligne.

## À suivre

**S1.** « Une base déjà écrite se répare » n'est vrai que pour les numéros, pas pour le contenu
de l'index de clé (inféré). Si l'ancien moteur a inséré après son DROP + point de reprise, il a
lu la clé dans la colonne périmée : l'index sur disque porte des clés fausses. L'ouverture
corrige `columnIDs` mais ne rebâtit ni ne détecte rien. La base gardée `stale-index-columns`
s'arrête juste après le CHECKPOINT : seul le cas favorable est témoigné. Scénario (ancien moteur
ff76b1ee3 puis nouveau) : `CREATE NODE TABLE T(extra INT64, id INT64, v INT64, PRIMARY KEY (id));
ALTER TABLE T DROP extra; CHECKPOINT; CREATE (:T {id: 5, v: 7});` — nouveau moteur :
`MATCH (n:T {id: 5}) RETURN n.v;` probablement vide ; `CREATE (:T {id: 5, v: 1});` probablement
accepté. À écrire au ticket comme limite.

**S2. Annoncé sans témoin** (`upstream_fixes_test.cpp:331-425, 644-720`) : fts (aucun cas ne
crée d'index fts) ; l'index vectoriel utilisé dans la même session après DROP + CHECKPOINT (le
seul cas vectoriel passe par REOPEN) ; l'extension non chargée pendant le DROP + CHECKPOINT ;
« tiennent après un point de reprise et une réouverture » (le second fils de
`StaleIndexColumnsAreRepairedAtOpening` ne revérifie que `Item {id: 1000}`) ; les deux gardes
Release (`local_node_table.cpp:55-60`, `local_rel_table.cpp:341-346`) et l'ordre des colonnes
pris à la table locale ; ATTACH.

**S3.** `renumberColumns` saute en silence un index sans entrée au catalogue
(`node_table.cpp:1182-1184`, `continue`) : une exception nommée serait dans l'esprit du commit.

**S4.** `getColumnID(propertyID)` reste un `KU_ASSERT` suivi de `.at()`
(`src/catalog/property_definition_collection.cpp:41-44`) : `std::out_of_range` anonyme en
Release. Non atteignable aujourd'hui (le DROP d'une colonne indexée est refusé, `alter.cpp:131`).

**S5.** Le test dépend de `gzip` et d'un shell (`std::system("gzip -dc …")`, l.~662) ; un chemin
à apostrophe casse la commande.

Non regardé : les plans préparés ou en cache qui porteraient des numéros de colonne à travers un
point de reprise (préexistant).

## Rien de faux trouvé

**1. La question de l'auteur — un lecteur entre la désérialisation et `renumberColumns`, à
l'ouverture de la base principale : aucun** (vérifié). Le catalogue est lu avant le stockage
(`checkpointer.cpp:260-264`) ; le constructeur de `NodeTable` prend `pkColumnID` au catalogue
(`node_table.cpp:296`) ; entre la création des `IndexHolder` (l.1154-1160) et `renumberColumns`
(l.1166) seul l'index de clé intégré est chargé, et `hash_index.cpp` ne lit que `keyDataTypes` ;
hnsw et fts sont chargés plus tard (`vector_extension.cpp:33`, `fts_extension.cpp:31`, au plus
tôt dans `autoLoadLinkedExtensions`, après `readCheckpoint`, `checkpointer.cpp:249`) et
reçoivent l'`IndexInfo` déjà corrigé (`index.cpp:101`) ; le rejeu appelle `readCheckpoint()`
avant de rejouer (`wal_replayer.cpp:245-310`) ; rien dans hnsw ou fts ne garde un numéro dérivé
à la construction. Seule exception : ATTACH (B1).

**2. Point de reprise** (vérifié). Ordre dans `NodeTable::checkpoint` (`node_table.cpp:853-880`) :
compactage, `nodeGroups->checkpoint`, `index.checkpoint`, `vacuumColumnIDs`, `renumberColumns` —
tout en mémoire, avant `serializeCatalogAndMetadata`. Entre compactage et renumérotation,
`index.checkpoint` ne lit pas la table par `columnIDs`. Échec après la renumérotation :
`checkpointFailed` puis plus rien n'est servi ; à la réouverture, ancien catalogue + rejeu,
cohérent. Lecteurs concurrents : aucun pendant le point de reprise.

**3. Base déjà écrite par l'ancien moteur** : réparée à l'ouverture pour les numéros, sans
changement de format ; réserves S1 et B1.

**4. Chemin ordinaire** (vérifié) : aucun coût par ligne (`getCommittedColumnIDs` une fois par
commit et par table ; `getIndexEntries` une fois par table et par point de reprise) ; pas de
lecture hors bornes ; les deux `KU_ASSERT` devenus exceptions sont au bon endroit. Hérité de la
fenêtre A, hors périmètre : `getPropertyPosition` recopie les définitions à chaque appel
(`table_catalog_entry.cpp:96`) ; au rejeu, `initInsertState` refait une recherche au catalogue
par enregistrement (`node_table.cpp:516-520`).

**5. Tests** : aucun `EXPECT_EQ`/`ASSERT_EQ` entre grands textes.

## Verdict du relecteur

Bloquant pour pousser : oui, pour B1 seul (quelques lignes, plus un témoin ATTACH). Si ATTACH
d'une base rag3db est tenu pour hors d'usage, B1 devient un ticket et le reste peut partir. S1
est à écrire au ticket avant de le fermer.
