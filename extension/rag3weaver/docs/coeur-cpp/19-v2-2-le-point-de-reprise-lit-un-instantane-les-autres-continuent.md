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

## 3. La forme : les générations (écrite le 25 octobre au soir, après un premier essai ; sur master en f8c0c8df0)

Un premier essai a suivi la forme « fondre jusqu'à F, porter les versions postérieures dans le
bloc neuf » (branche `v22a-instantane`, 9101ec942) : elle marche en mémoire (le lecteur ouvert
avant relit son instantané, un lecteur nouveau voit l'état validé), mais **l'état sur disque est
alors à F**, et le journal se supprime à la fin du point de reprise : tout ce qui est validé entre
F et T est perdu à la réouverture (le témoin l'a vu à sa dernière ligne). Il aurait fallu garder
le journal à partir de la position de F, et le rejeu aurait réinséré des lignes déjà copiées. La
forme retenue est l'autre moitié de PostgreSQL : **la version ancienne reste là, à côté**.

**Deux horodatages** pris sous le verrou public au départ de l'écriture, une fois les écrivains
actifs partis : **T** = `lastTimestamp` (tout ce qui est validé est au point de reprise, comme
aujourd'hui) et **F** = le plus vieux instantané des lecteurs restés ouverts (T s'il n'y en a pas).
Le point de reprise **fond tout** comme avant : l'état sur disque est à T, le journal se supprime,
rien ne change à la durabilité ni au format.

**La génération précédente.** Si F < T, chaque groupe garde à l'échange sa liste des blocs
d'avant comme génération précédente (`NodeGroup::previousGenerations` : T et les blocs ;
`CSRNodeGroup::csrPreviousGenerations` : T, le bloc persistant, les blocs en mémoire et une copie
intacte de l'index des listes, le bâti travaillant sur une copie puisqu'il y invalide les lignes
supprimées). Une transaction d'instantané < T y lit ce qu'elle voyait, **versions comprises** :
l'ancienne liste porte encore les `VersionInfo` et `UpdateInfo` que le point de reprise a fondus
dans le bloc neuf (V2-1b ne mute rien de l'ancien bloc). `initializeScanState`,
`initScanForCommittedPersistent`, `scanCommittedInMem`, `isVisible`, `isDeleted`, `isInserted`
choisissent la génération par `groupsFor(startTS)` / `takeGenerationFor` : la plus ancienne
génération gardée dont le point de reprise est postérieur à l'instantané, sinon le vivant. Un
instantané égal à T lit le vivant, qui est exactement ce qu'il voit.

**Le retrait.** Une génération est retirée au point de reprise suivant quand plus aucune
transaction ne la précède (`checkpointTS ≤ F` du point de reprise suivant) ; ses blocs meurent
alors et rendent leurs pages (`retireStorage` les avait marquées, V2-1b (d)). Le coût est la
mémoire des anciens blocs des groupes changés, le temps d'un lecteur long : c'est le §3.7, mesuré
d'abord.

**Les écritures pendant la fenêtre d'un groupe** (V2-2c). Le dernier bloc en mémoire est scellé
à la prise de la liste (`ChunkedNodeGroup::seal`) ; ce qui s'ajoute ensuite ouvre un bloc neuf,
gardé derrière le bloc neuf à l'échange d'un groupe de nœuds. Pour un groupe CSR, un bloc ajouté
décalerait les numéros de lignes de l'index des listes : les écritures d'un groupe CSR entre la
prise et l'échange attendront une garde du groupe (partagée pour écrire, exclusive pour
l'échange), pas une liste. Les mises à jour et suppressions à l'instruction dans l'ancien bloc
pendant la fenêtre (versions non validées) passeraient à la génération précédente et non au bloc
neuf : la même garde les retient le temps de l'échange. Le tampon d'annulation pointe dans
l'ancien bloc : avec la garde, une version non validée n'est jamais dans un bloc remplacé.

**La porte, le journal, le catalogue** (V2-2c, inchangés par rapport à la première rédaction) :
le point de reprise attend encore les écrivains actifs à son départ ; `checkpointWriting` ouvre
ensuite la porte à tout le monde ; le journal scellé à T et un journal neuf (format 44) ; le
catalogue et l'en-tête à T.

### 3.7 Ce que ça coûte

Les anciens blocs des groupes changés restent en mémoire (et leurs pages sur disque) le temps
qu'un lecteur ouvert avant le point de reprise finisse, jusqu'au point de reprise suivant : c'est
le prix de PostgreSQL aussi (le vacuum attend le plus vieux instantané). Décision de
l'orchestration (25 octobre au soir, soumise à Lucie) : **pas de plafond à l'aveugle** ; d'abord
la mesure (le lecteur long du scénario du banc, et le témoin d'un lecteur long pendant un COPY
forcé que prépare l'arbre principal), puis un plafond nommé à la manière d'`old_snapshot_threshold`
de PostgreSQL : un lecteur trop vieux reçoit « snapshot too old » au lieu de bloquer le monde,
**éteint par défaut** tant que la mesure ne dit pas qu'il faut l'allumer. Lot à part.

## 4. L'ordre des pièces, et le témoin de chacune

| pièce | ce qu'elle change | témoin |
|---|---|---|
| V2-2a + 2b | les générations (nœuds et CSR), le scellement ; la porte n'attend plus les lecteurs | `checkpoint_open_reader_test.cpp` : lecteur ouvert avant, non attendu, relit pareil pendant et après, nœuds et relations ; banc : `CheckpointOpenReader.*` |
| V2-2c | journal scellé, catalogue à T, porte ouverte aux écrivains nouveaux, garde de groupe | `BeginDuringACheckpointDoesNotWait` vert ; écrivain tué après sa validation pendant l'écriture, ligne présente à la réouverture (banc) |
| V2-3 | l'attente des écrivains actifs levée | un écrivain parti avant, validé pendant, lu juste |

V2-2a et 2b ensemble parce qu'un lecteur ne choisit pas ses tables ; V2-2c dernière parce
qu'elle change le format du journal (bascule) et touche `transaction_manager`, où hotfixs écrit
aussi (ordre réglé par l'orchestration).
