# V2-2 : le point de reprise lit un instantané, les autres continuent — ce que dit le code, et la forme

Session cœur C++, 25 octobre 2026 au soir, après V2-1c (66994dd43). V2-1 a ouvert la porte aux
**lecteurs nouveaux** pendant l'écriture. Restent, par la page 17 §3.2 (3) : le **lecteur ouvert
avant** le point de reprise, qu'il attend encore (borné, 5 s puis échec), et l'**écrivain
nouveau**, retenu jusqu'à la fin de l'écriture (`BeginDuringACheckpointDoesNotWait` rouge). Cette
page dit pourquoi ce n'est pas une porte de plus, et la forme qui le permet.

## 1. Ce que V2-2 rend vert

- `CheckpointNoStall.BeginDuringACheckpointDoesNotWait` (banc) : un BEGIN d'écrivain 30 ms après
  le départ du CHECKPOINT rend la main en moins de 100 ms ; il valide pendant l'écriture ; sa
  ligne est là après réouverture, même si le processus meurt entre sa validation et la fin du
  point de reprise.
- Un lecteur ouvert avant (BEGIN READ ONLY tenu) n'est plus attendu, et relit la même chose
  avant, pendant et après le point de reprise : une ligne modifiée et validée entre son BEGIN et
  le CHECKPOINT ne lui apparaît jamais (le banc l'écrit).
- Inchangé : les validations forcées (COPY hors journal, index) attendent les autres, leur
  durabilité est leur propre point de reprise ; `ReadersLock` entre processus.

## 2. Les faits qui comptent (lus dans le code, branche de V2-1c)

1. **Le point de reprise lit avec un seul instantané figé** : `DUMMY_CHECKPOINT_TRANSACTION`
   (`startTS = LATEST_COMMITTED_TS`, `ID = 0`, `transaction.cpp:391`), partout :
   `checkpointVersionInfo`, `hasAnyUpdates`, `scanCommitted`, `isDeleted`, `lookupMultiple`,
   `getNumDeletions`. Rien ne lit `oldestActiveStartTS` au point de reprise (seule
   `compactDeadRowsNoLock` des listes CSR s'en sert, `csr_node_group.cpp:440-482`).
2. **Une mise à jour est une « valeur après »** (`VectorUpdateInfo::data`, `update_info.h:31`) ;
   une lecture à un instantané ancien se reconstruit en partant du segment de base et en
   n'appliquant que les versions `<= startTS` (`iterateVectorInfo`, `update_info.cpp:130`). Fondre
   dans la base une version postérieure à un lecteur détruit ce qu'il devait lire.
3. **Les mises à jour et les suppressions de lignes validées s'écrivent dans le stockage
   partagé à l'instruction**, avec la version = ID de transaction (`NodeTable::update`
   `:768-779`, `delete_` `:849-860` ; `UpdateInfo::update`, `VersionInfo::delete_`). Le commit ne
   fait que les dater (`UndoBuffer::commit` → `UpdateInfo::commit`, `setInsert/DeleteCommitTS`).
   Seules les insertions passent par le stockage local, et s'ajoutent à la validation
   (`NodeGroupCollection::append` → `NodeGroup::append`, sous `chunkedGroups.lock()` et
   `inMemAppendMtx` : un bloc IN_MEMORY neuf si le dernier est plein ou ON_DISK, sinon le dernier
   s'allonge).
4. **Le tampon d'annulation pointe dans l'ancien bloc** : `VectorUpdateRecord` garde
   `UpdateInfo*` et `VectorUpdateInfo*` bruts (`undo_buffer.cpp:58-63`) ; `VersionRecord` garde
   `(nodeGroupIdx, startRow, numRows)` et se résout dans la **liste vivante**
   (`NodeGroup::applyFuncToChunkedGroups`, `node_group.cpp:950-973`), où `commitInsert`/`commitDelete`
   déréférencent `versionInfo` sans test (`chunked_node_group.cpp:591, 622`). Après un échange,
   un commit d'écrivain tomberait sur le bloc neuf, sans ses versions.
5. **Les versions ne se persistent pas** : `VersionInfo::serialize` exige NO_INSERTED ou
   ALWAYS_INSERTED (`version_info.cpp:337`) ; un TS de suppression par ligne est possible mais
   `checkpointVersionInfo` y écrit 0 (`node_group.cpp:690`) ; `UpdateInfo` n'est jamais
   sérialisé (`column_chunk.cpp:217-224`) et un `ColumnChunk` neuf (`sharedView`, `checkpoint`)
   n'en a pas et ne peut pas en recevoir (champ privé, `column_chunk.h:346`).
6. **Le CSR change les décalages** au point de reprise : suppressions retirées physiquement
   (`writeCSRListWithPersistentDeletions`), insertions en mémoire intercalées, régions
   redistribuées. Un numéro de ligne CSR n'est pas une identité à travers un point de reprise ;
   `PersistentVersionRecordHandler` adresse pourtant par décalage (`rel_table_data.cpp:29-38`).
7. **L'échange jette la liste entière** (`chunkedGroups.clear`, `finalizeCheckpoint`) : un bloc
   ajouté entre la prise de `groups` et l'échange partirait sans être fondu ; des lignes ajoutées
   au dernier bloc en mémoire après la copie (`scanAllInsertedAndVersions` copie `numRows` au
   moment de la copie) seraient perdues.
8. **Le journal** : un enregistrement CHECKPOINT est toujours le dernier (assert dans
   `dryReplay`), et `WAL::reset` supprime le fichier à la fin du point de reprise ; le rejeu
   repart du début jusqu'au dernier COMMIT. Une validation faite pendant l'écriture partirait
   avec le fichier.
9. **Le catalogue** se sérialise avec la dernière version validée de chaque entrée
   (`CatalogSet::serialize`, `getCommittedEntryNoLock`), pas à un horodatage ; les entrées sont
   pourtant versionnées par TS.
10. `oldestActiveSnapshotNoLock(committing)` (`transaction_manager.cpp:99-108`) donne déjà le
    plus vieux `snapshotFloorTS` des transactions actives.

## 3. La forme

Deux horodatages, pris sous le verrou public au départ du point de reprise, une fois les
écrivains actifs partis (voir 3.6) :

- **T** = `lastTimestamp` : tout ce qui est validé à T est au point de reprise, rien d'après.
- **F** = min(T, `snapshotFloorTS` des lecteurs actifs) : ce qui est validé à F est visible de
  tout le monde et **se fond** ; ce qui est validé entre F et T, ou pas encore validé, **se garde
  en versions** dans le bloc neuf.

### 3.1 Fondre jusqu'à F, porter le reste (NodeGroup)

Le point de reprise lit avec une transaction d'instantané F (un `Transaction` CHECKPOINT à
`startTS = F`, ID 0) là où il lisait DUMMY : `checkpointVersionInfo`, `hasAnyUpdates`,
`scanCommittedUpdatesForColumn`, `scanAllInsertedAndVersions`, `isDeleted`. Le segment neuf
porte donc les valeurs à F. Puis, **à l'échange, sous le verrou du groupe**, les versions
postérieures à F de l'ancien bloc passent au neuf :

- mises à jour : pour chaque colonne et chaque vecteur, les `VectorUpdateInfo` de version `> F`
  (TS dans ]F, T] ou ID d'une transaction en cours) sont **copiées** dans l'`UpdateInfo` du chunk
  neuf, même version, mêmes lignes, mêmes valeurs (une API `ColumnChunk::carryVersionsAfter(F,
  const ColumnChunk& old, mapping)`). Les positions sont stables dans un NodeGroup (persistant,
  puis insertions à `numPersistentRows`) : le mapping est l'identité.
- suppressions : `VersionInfo` du bloc neuf = suppressions validées à F fondues (TS 0, comme
  aujourd'hui, juste pour tout lecteur actif puisque F ≤ leur instantané), plus les suppressions
  de version `> F` recopiées avec leur TS ou leur ID (une pose directe, pas `delete_` qui lève
  un conflit au second ID).
- insertions : les lignes validées dans ]F, T] sont dans le bloc neuf (copiées par position) avec
  leur TS d'insertion gardé **en mémoire** (CHECK_VERSION) ; à la sérialisation elles s'écrivent
  ALWAYS_INSERTED (après une réouverture, aucune transaction ne les précède ; ce qui est au-delà
  de T n'est pas dans le point de reprise, il est au journal neuf). Les insertions non validées
  n'existent pas dans les blocs partagés : elles sont locales jusqu'à la validation.

### 3.2 Les blocs en mémoire après la prise

À la prise de `groups`, le dernier bloc en mémoire est **scellé** (plein par décision) : une
validation qui ajoute ensuite ouvre un bloc neuf. À l'échange, les blocs de la liste vivante qui
ne sont pas dans `groups` sont **gardés** derrière le bloc neuf (leur `startRowIdx` suit le compte
du bloc neuf, qui a fondu toutes les lignes des blocs pris). Rien n'est jeté qui n'ait été fondu.

### 3.3 Les écritures à l'instruction pendant la fenêtre d'un groupe

Entre la prise et l'échange, un écrivain peut écrire dans l'ancien bloc (3). Sa version est
portée par 3.1 (ID > F). Pour qu'aucune écriture ne tombe entre la copie des versions et
l'échange, le groupe a une garde : `update`/`delete_` la prennent en partagé le temps de trouver
le bloc et d'écrire ; la copie des versions et l'échange la prennent en exclusif. Courte (un
vecteur), jamais tenue avec le verrou public.

### 3.4 Le tampon d'annulation ne pointe plus

`VectorUpdateRecord` devient `(table, nodeGroupIdx, columnID, vectorIdx, ID)` : à la validation
ou à l'annulation, la version se retrouve **par son ID** dans la chaîne du chunk **vivant**
(`UpdateInfo::commitVersion(vectorIdx, ID, commitTS)`, `rollbackVersion`). Après un échange, la
copie de 3.1 porte le même ID : le commit la date dans le bloc neuf. `VersionRecord` se résout
déjà par position dans la liste vivante ; `commitInsert`/`commitDelete` créent le `versionInfo`
s'il manque au lieu de le déréférencer.

### 3.5 Le CSR : le mapping dans la réécriture

Les décalages changent (6) : la copie des versions se fait **dans la boucle de réécriture**, qui
connaît pour chaque ligne gardée son ancien et son nouveau décalage (`writeCSRList…`,
`writeInMemoryCSRInsertion`). Une ligne supprimée après F est **gardée physiquement** (le saut
physique ne vaut que pour `isDeleted` à F), écrite à son nouveau décalage avec sa marque de
suppression ; ses mises à jour `> F` sont copiées au nouveau décalage. `PersistentVersionRecordHandler`
et `CSRNodeGroup::update/delete_` en source PERSISTENT passent par la garde de 3.3 et lisent
`persistentChunkGroup` sous elle ; un commit qui adresse une ligne du bloc persistant par
décalage après l'échange adresse le **nouveau** décalage : le record d'annulation d'une
suppression ou mise à jour CSR porte l'ID, résolu par 3.4 dans le bloc vivant, pas un décalage.
La compaction des listes (`compactDeadRowsNoLock`, à `oldestActiveStartTS`) est déjà de cette
forme.

### 3.6 La porte, le journal, le catalogue, l'en-tête

- **Porte.** Le point de reprise attend encore le départ des **écrivains actifs** à son départ
  (borné comme aujourd'hui) : leurs instructions en cours écrivent dans les blocs, et V2-2 ne
  veut pas prouver en même temps le portage des versions et la fenêtre d'un écrivain déjà
  parti. Les lecteurs actifs ne sont **plus** attendus (F les couvre). Puis `checkpointWriting`
  ouvre la porte **à tout le monde** : un écrivain nouveau commence, écrit (3.3), valide (3.4,
  journal neuf) pendant l'écriture. V2-3 lèvera l'attente des écrivains actifs, sur la même
  mécanique.
- **Journal.** À T, sous le verrou du journal, le fichier courant est **scellé** et un journal
  neuf s'ouvre (`<base>.wal` devient `<base>.wal.1` ; les validations d'après T écrivent dans le
  neuf). À la fin, l'enregistrement CHECKPOINT va dans le scellé, qui est supprimé ; le neuf
  continue. Rejeu : un scellé sans CHECKPOINT final se rejoue puis le neuf ; un scellé fini par
  CHECKPOINT se termine (pages fantômes) puis le neuf se rejoue ; `dryReplay`, l'identité
  d'ouverture en lecture seule et `ReadersLock` lisent les deux fichiers. **Format 44** (le 43 est
  à hotfixs, l'état d'index de `create-concurrently-etat`, bascule du 26 octobre) : le journal
  porte le `FormatRecord`, un journal double est refusé par une lib d'avant, ce qui est voulu ;
  sa propre bascule, réglée par l'orchestration.
- **Catalogue et métadonnées à T.** `CatalogSet::serialize` prend l'entrée visible à T
  (`getEntryOfTimestamp`), pas la dernière validée ; une table créée après T n'est pas dans ce
  point de reprise, son stockage non plus (elle revient par le journal neuf). Les statistiques
  et le gestionnaire de pages se sérialisent à la fin, après l'échange de tous les groupes : les
  pages allouées après T par des écrivains (blocs scellés, journal) sont des pages que l'en-tête
  à T ne désigne pas, et `returnOwnerlessPages` les rend au rejeu comme aujourd'hui.
- **En-tête à T.** Rien de plus : il désigne catalogue et métadonnées écrits ci-dessus.

### 3.7 Ce que ça coûte

Les chaînes de versions `> F` sont copiées, pas partagées : le coût est proportionnel à ce qui a
été validé pendant la fenêtre d'un lecteur long, et se résorbe au point de reprise suivant (F
avance). Un lecteur très long (heures) fait grossir les chaînes : c'est le prix de PostgreSQL
aussi (le vacuum attend). Décision de l'orchestration (25 octobre au soir, soumise à Lucie) :
**pas de plafond à l'aveugle** ; d'abord la mesure (le lecteur long du scénario du banc, et le
témoin d'un lecteur long pendant un COPY forcé que prépare l'arbre principal), puis un plafond
nommé à la manière d'`old_snapshot_threshold` de PostgreSQL : un lecteur trop vieux reçoit
« snapshot too old » au lieu de bloquer le monde, **éteint par défaut** tant que la mesure ne dit
pas qu'il faut l'allumer. Lot à part.

## 4. L'ordre des pièces, et le témoin de chacune

| pièce | ce qu'elle change | témoin |
|---|---|---|
| V2-2a | 3.1, 3.2, 3.4 (NodeGroup) : fondre à F, porter `> F`, scellement, records par ID ; la porte n'attend plus les lecteurs | lecteur ouvert avant, non attendu, relit pareil avant/pendant/après (banc) ; `test/transaction` : versions portées à travers un point de reprise avec un lecteur à instantané ancien |
| V2-2b | 3.5 (CSR) | le même témoin sur une table de relations |
| V2-2c | 3.6 journal scellé, catalogue à T, porte ouverte aux écrivains nouveaux, 3.3 | `BeginDuringACheckpointDoesNotWait` vert ; écrivain tué après sa validation pendant l'écriture, ligne présente à la réouverture (banc) |
| V2-3 | l'attente des écrivains actifs levée | un écrivain parti avant, validé pendant, lu juste |

V2-2a d'abord parce qu'elle contient tout le mécanisme des versions et que son témoin n'a pas
besoin de la porte des écrivains ; V2-2c dernière parce qu'elle change le format du journal
(bascule) et touche `transaction_manager`, où hotfixs écrit aussi (ordre réglé par l'orchestration).
