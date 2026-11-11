# Sous vingt écrivains, un pool plein laisse une annulation inachevée

- **État** : ouvert (9 novembre 2026, cœur c++)
- **Famille** : moteur
- **Gravité** : indisponibilité — la base refuse toute transaction jusqu'à sa réouverture
- **Atteignable en service** : à établir. Le banc le reproduit seul, sur master avant et après
  29a7bc52a ; un hôte sous un pool de tampons serré et des DDL concurrents y ressemble.
- **Touche rag3weaver** : par le moteur (l'hôte devrait fermer et rouvrir la base)
- **Pour** : cœur c++

## Ce que c'est

`TwentyWriters.DdlEveryMinute` (`test/transaction/concurrence/twenty_writers_scenario_test.cpp`)
rougit seul sous poste lourd, sur 813d7eda8 (master sans le correctif des deux sens) comme sur
52d73a301 (avec). La lib du banc est bâtie de 9bae60786 et suivants, en Release, à luciepc.

Ce que dit son journal (`~/.cache/rag3db-moteur-notes/csr/dep-base.log` à luciepc) :

| Message | Nombre |
|---|---|
| `BEGIN refusé : Buffer manager exception: Unable to allocate memory! The buffer pool is full and no memory could be freed!` | 9 |
| `BEGIN refusé : A rollback of this database could not complete, so it must be closed and reopened before it is used again` | 318 434 |

Le contrôle qui échoue est `index-creation-refuses-no-reader` (8 refus de lecteurs), puis une
exception non rattrapée (`unknown file: Failure`). Les fenêtres « 5 CHECKPOINT », « 6 ADD b
DEFAULT » et « 6 migration annulée » ne voient plus que des refus.

**Lecture** : un pool plein fait échouer une annulation, et la base se met d'elle-même en état
« à rouvrir » (la garde posée pour une annulation inachevée). Tout ce qui suit est refusé. La
question est de savoir si l'annulation peut échouer faute de mémoire, ou si elle devrait réserver
ce qu'il lui faut avant de commencer.

`TwentyWriters.DdlEveryFifteenSeconds` montrait les mêmes « buffer pool is full » dans le banc
entier, mais passe seul des deux côtés.

## Les seuils de temps notés ici

Deux cas de coût rougissent seuls, sur les deux mêmes versions. Ce sont des seuils à revoir avant
d'en faire un défaut :
- `Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize/SetInBatchesOf64` ;
- `Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize/SetInBatchesOf64WithRelation` : 7,95 ms
  par ligne à 15 000 lignes pour 5,86 permises, contre 1,93 ms à 1 500.

`TwentyWriters.KilledDuringTheCheckpoint` (enfant mort de SIGSEGV avant le point de reprise) n'a
été vu que dans le banc entier ; il passe seul des deux côtés. À rejouer sous charge, sans ticket.

## Témoin

`TwentyWriters.DdlEveryMinute` lui-même, rouge seul. Un témoin réduit reste à écrire : une
annulation qui échoue faute de pool, puis un BEGIN.

## Pour le fermer

La cause de l'annulation inachevée, le correctif, le témoin vert.

## Reprise du 11 novembre — témoin réduit rouge

Le témoin initial `6e7f11680` ne reproduit pas : `FullBufferRollback.*`, 3/3 verts
à luciepc (3,325 s), journal `~/.cache/rag3db-moteur-notes/annulation/rouge.log`
(le nom du fichier ne dit pas son verdict). La variante CREATE à clé STRING courte
passe aussi (4/4 au total, `cle-string.log`) : ses lignes sont encore locales.

Le COPY atteint au contraire `INSERT_INFO` puis `rollbackPKIndexInsert`. Le cas
existant INT64 et sa variante STRING courte passent (2/2, `copy-string.log`).
Avec 20 000 clés STRING préfixées de 128 caractères `k`, le défaut est reproduit :
`CopyRollbackUnderAFullBuffer.StringKeysRemainUsableAfterRollback`, témoin sauvegardé
sur `annulation-sous-pool-plein`, commit `65486c02d`.

Recette : pool des tests de 64 Mio, point de reprise automatique coupé, COPY dans
Doc sous transaction A ; B insère et valide dans Other, par lots depuis 5 000 avec
division par deux après refus. B valide 155 077 lignes et subit 7 refus.
Le premier ROLLBACK de A échoue explicitement avec « buffer pool is full », le second
et le BEGIN suivant exigent de rouvrir la base, COUNT est refusé. Le fils ferme sans
signal. Le voisin INT64 reste vert. Preuve : luciepc
`~/.cache/rag3db-moteur-notes/annulation/copy-long-string.log` (1 vert, 1 rouge).

Aucun correctif à ce stade. Prochain pas : tracer l’allocation durant le retrait des
clés longues, puis comparer les stratégies des moteurs avant proposition à l’orchestration.

### Cause tracée et suite

La pile de `annulation/trace.log` confirme : `rollbackPKIndexInsert` →
`scanIndexColumns` → `StringVector::reserveString` →
`InMemOverflowBuffer::allocateNewBlock` → `MemoryManager::allocateBuffer` →
`BufferManager::pin`, qui refuse. La copie des clés longues vers le vecteur de
lecture demande le nouveau bloc ; ce n’est pas une preuve d’allocation dans le
retrait de la clé elle-même. La trace reproduit le rouge, voisin INT64 vert.

Proposition transmise à l’orchestration avant tout correctif : lire les clés depuis
leur stockage existant pour l’annulation, sans recopier les chaînes dans un nouveau
tampon ; vérifier les verrous et la durée de vie de la vue empruntée.

Lectures de référence : PostgreSQL réserve 8 Kio dans
[ErrorContext](https://github.com/postgres/postgres/blob/master/src/backend/utils/mmgr/mcxt.c)
pour traiter les erreurs ; cela ne prouve pas une réserve pour tout rollback.
[SQLite](https://sqlite.org/lang_transaction.html) peut annuler toute la transaction
après SQLITE_NOMEM. [DuckDB](https://duckdb.org/docs/2.0/guides/troubleshooting/crashes)
invalide la base après une erreur interne. Ces politiques ne démontrent pas que
notre annulation exige une réserve forfaitaire.

Branche de reprise désormais `cx-coeur-annulation`, publiée, worktree
`~/git_workspaces/cx-coeur-annulation`, même commit `65486c02d`.
L’ancien worktree `rag3db-csr` reste sur disque, détaché, sans suppression.

### Correctif en essai, non validé

Commit `0511dece7` sur `cx-coeur-annulation` : les slots de l’index local détiennent
déjà les clés et leurs décalages. `discardLocalRange(first,count)` retire celles
qui appartiennent à la plage réservée par l’undo, sous verrou exclusif local,
sans lecture des colonnes ni modification du persistant ou des tombstones.
Les clés conservées sont déplacées dans les slots existants, en maintenant le
préfixe compact de chaque chaîne. La comparaison par soustraction évite de faire
déborder `first+count`. Le code impose un déplacement `noexcept`. Cela concerne
ce retrait de clés, pas une preuve que toute annulation du moteur n’alloue rien.

Compilation et tests ciblés en file à luciepc (`annulation/vert.log`) : aucun
verdict sur ce correctif à ce point. Les suites entières et la mesure de coût
restent à jouer ; le ticket reste ouvert.
