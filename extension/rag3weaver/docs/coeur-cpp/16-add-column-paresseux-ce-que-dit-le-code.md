# ADD COLUMN paresseux : ce que dit le code (18 octobre 2026)

Demande de l'orchestration après la mesure du banc (`DdlEveryMinute`, 2 Gio) : un `ADD COLUMN`
de 1,08 s fait attendre tout le monde 1,3 s, écrivains des autres tables compris ; `ADD rang
INT64 DEFAULT 0` prend 345 ms et retient `Doc` 1,3 s ; un `CHECKPOINT` fige tout 1,4 s. Le cas
d'usage de Lucie dit « aucun bloqué ». PostgreSQL, depuis la 11, range le défaut constant au
catalogue (`attmissingval`) et ne réécrit aucune ligne. Cette page lit notre code avant toute
écriture.

## 1. Ce que fait un ADD COLUMN aujourd'hui

- **Catalogue** (`CatalogSet::alterTableEntry`) : copie de l'entrée de la table, nouvelle
  propriété avec son défaut (`PropertyDefinition` porte déjà l'expression), sous le verrou
  exclusif du `CatalogSet` le temps de la copie : court. Depuis E10, un second ALTER vivant sur la
  même table est refusé par un nom.
- **Stockage** (`NodeTable::addColumn` → `NodeGroupCollection::addColumn`) : **sous
  `nodeGroups.lock()` de la table pour toute la durée**, pour chaque groupe puis chaque bloc
  (`ChunkedNodeGroup::addColumn`) : un `ColumnChunk` neuf à la capacité du bloc,
  `populateWithDefaultVal` qui évalue l'expression du défaut par tranche de 2 048 valeurs et les
  copie (nulls et statistiques compris), puis, pour un bloc sur disque, `flush` : pages allouées
  et écrites tout de suite dans le fichier de données. Le stockage local de la transaction reçoit
  la colonne de même. La table de relations fait tout cela deux fois (ses deux sens).
- **Compression constante** (`ConstantCompression::analyze`) : un bloc dont toutes les valeurs
  sont égales s'écrit en **zéro page**, métadonnées seules, pour les booléens, entiers et
  flottants. Les chaînes n'ont pas cette forme : un défaut `STRING` écrit un dictionnaire par bloc.
  C'est l'écart mesuré par le banc : 345 ms pour `INT64 DEFAULT 0` (évaluation et copie par
  ligne, rien d'écrit), 1,08 s pour `STRING DEFAULT 'b-defaut'` (dictionnaires écrits).
- **Journal** : un seul enregistrement `ALTER` ; le rejeu refait tout le remplissage
  (`WALReplayer::replayAlterTableEntryRecord` relie le défaut et rappelle `addColumn`) : même
  coût à la réouverture après un arrêt brutal.
- **Validation** : rien de particulier (`incrementVersion` du catalogue, fsync du journal sous le
  mutex public du gestionnaire). **Aucun point de reprise forcé** : seuls `COPY` et les index le
  forcent (`setForceCheckpoint`).
- **Annulation** : depuis E10, la colonne est retirée du stockage au ROLLBACK (pages rendues).

## 2. Qui attend, et pourquoi

- **La table** : `nodeGroups.lock()` est tenu pendant tout le remplissage. Tout balayage et
  toute insertion de `Doc` attend : c'est le « retient Doc 1,3 s ».
- **Les autres tables** : l'ADD lui-même ne tient rien de commun (le verrou du catalogue un
  instant, celui du gestionnaire de pages par allocation). Le code ne montre pas de verrou global
  pendant l'instruction. Deux choses peuvent retenir les écrivains des autres tables, et il faut
  les départager par la mesure :
  1. **un point de reprise automatique à la validation de l'ADD** : si le journal dépasse le seuil
     (`Checkpointer::canAutoCheckpoint`), la validation arrête les nouveaux BEGIN, attend le départ
     de tous (A8), puis écrit ; c'est exactement « tout le monde attend 1,3 s », à côté du
     « CHECKPOINT fige 1,4 s » mesuré à part ;
  2. la validation sous le mutex public (fsync), de l'ordre de la milliseconde : trop court.
  À faire au banc : rejouer `DdlEveryMinute` avec `RAG3DB_PROFILE_CHECKPOINT=1` ; la ligne
  `[commit-profile]` de la validation de l'ADD dit si un point de reprise l'a suivie (« au
  seuil ») et combien il a pris. Si oui, l'attente globale n'est pas celle de l'ADD mais celle du
  point de reprise, et la forme paresseuse ne la supprime pas.

## 3. La forme à la PostgreSQL 11, chez nous

Le défaut est déjà au catalogue. Ce qui manque, c'est que le stockage n'ait rien à faire pour les
lignes d'avant. Deux formes :

- **(a) Un bloc constant par bloc existant** : au lieu de remplir, poser dans chaque bloc un
  `ColumnChunk` « constant » (métadonnées `CompressionType::CONSTANT`, zéro page, pas
  d'évaluation par ligne). O(nombre de blocs), toujours sous `nodeGroups.lock()` mais bref. Limite :
  pas de forme constante pour les chaînes, les listes, les structs ; il faudrait l'ajouter au moins
  aux chaînes (une valeur dans les métadonnées, ou un dictionnaire d'une entrée partagé), et le
  stockage local, les statistiques et le rejeu suivent le même chemin.
- **(b) Un bloc absent vaut le défaut du catalogue** : l'ADD ne touche aucun bloc existant
  (`columns.push_back` et le type, c'est tout). Le balayage, la recherche et la mise à jour d'un
  bloc où `columnID >= chunks.size()` rendent le défaut lu au catalogue (évalué une fois par
  requête) ; la première **écriture** dans un tel bloc le matérialise (remplissage d'alors, sous le
  verrou du bloc) ; le **point de reprise** matérialise ce qu'il écrit (il tourne sans aucune
  transaction) ; un bloc neuf (append) reçoit la colonne normalement. O(1) par ADD, aucune page
  écrite, aucun verrou long, tous les types. Le rejeu de l'ALTER devient O(1) aussi. Le ROLLBACK
  d'E10 n'a rien à rendre pour les blocs non matérialisés.

**Préférence : (b)**, parce qu'elle vaut pour tous les types et supprime le remplissage plutôt
que de l'accélérer. Ce qu'elle touche : `ChunkedNodeGroup::scan`, `lookup`, `update` et
`addColumn` (une branche « colonne absente »), `NodeGroup::checkpoint` (matérialiser), le
stockage local (`LocalNodeTable::addColumn`), `RelTableData` pour les deux sens, et un témoin par
chemin (lecture, SET, suppression, point de reprise, réouverture, E10). Les défauts **non
constants** (`uuid()`, `current_timestamp()`) gardent le remplissage immédiat : leur valeur
dépend de la ligne.

## 4. Ce que cela ne règle pas

- Si le point de reprise automatique suit l'ADD (à mesurer, §2), l'attente globale reste celle
  du point de reprise, qui est un autre sujet (le `CHECKPOINT` qui fige tout 1,4 s : le point de
  reprise arrête les BEGIN et attend le départ de tous ; le rendre plus court ou plus rare est le
  lot « point de reprise sous lecteurs », pas celui-ci).
- `DROP COLUMN` ne touche déjà rien (il ne fait que marquer la table) ; `RENAME` non plus.
