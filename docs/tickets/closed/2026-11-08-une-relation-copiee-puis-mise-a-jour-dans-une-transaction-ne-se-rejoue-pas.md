# Une relation copiée puis mise à jour dans une transaction ne se rejoue pas

- **État** : corrigé (lot `rejeu-relations-copiees`, 8 novembre 2026, session hotfixs)
- **Gravité** : perte — une base dont le journal porte cette suite ne se rouvre plus
  (code-rag3db-depot, 8 novembre 2026 à 9 h 05)
- **Atteignable en service** : oui — vu en service. La réindexation de la montre supprime une
  arête, la recrée par un COPY de relations journalisé, puis la met à jour dans la même
  transaction.
- **Touche rag3weaver** : par le moteur (la base de l'hôte ne s'ouvre plus)

## Ce que c'est

Un COPY de relations journalisé écrit ses relations dans les listes CSR, à leurs décalages
définitifs, et les inscrit au journal au milieu de la transaction (TABLE_INSERTION). Une mise à
jour ou une suppression de ces relations dans la même transaction les trouve dans les listes, et
s'inscrit au journal avec ce décalage définitif (REL_UPDATE, REL_DELETION).

Au rejeu, `WALReplayer::replayRelTableInsertRecord` rejoue chaque relation par `RelTable::insert`,
c'est-à-dire dans le stockage local de la transaction de reprise, à un décalage local. La mise à
jour qui suit cherche le décalage définitif dans les listes et ne le trouve pas. La garde du sens
unique (`RelTableData::update`) refuse alors, et la base ne s'ouvre plus :

> Relationship 187668 of table CONSUMED_BY was not found in the forward direction, among the
> relationships of node 10769. Its update is refused: it would have reached one direction only.

Le journal, lui, est cohérent. Le cœur l'a lu enregistrement par enregistrement
(`WalReader.PrintAFile`) : #3295, suppression de l'ancienne arête 10769 → 77441 ; #13705, insertion
de 187668 dans les deux tables jumelles, au milieu de la transaction ; #14045, sa mise à jour.

Le défaut précède les lots du 7 novembre : le témoin rougit aussi entre nœuds validés, sans nœud
local ni versement. La lib dadbf0041, en service de 8 h 31 à 9 h 05, a seulement rendu la suite
fréquente.

## La correction

Au rejeu d'une insertion de relations, `WALReplayer` garde, par table et le temps d'une transaction
rejouée (vidée au BEGIN et au COMMIT), la correspondance du décalage journalisé vers le décalage
local que l'insertion vient de donner. Le rejeu d'une REL_UPDATE et d'une REL_DELETION y traduit son
identifiant (`translateReplayedRelID`), puis prend le chemin local de `RelTable::update` et de
`RelTable::delete_`, comme en service.

## Les témoins

- `Cases/RelCopyThenUpdateReplay.TheReplayFindsTheCopiedRelToUpdate`
  (`test/transaction/concurrence/provisional_offsets_test.cpp`) : une arête validée est supprimée,
  recréée par un COPY de relations, puis mise à jour, dans une transaction ; le processus meurt base
  ouverte, et le rejeu doit rendre la relation, mise à jour, dans les deux sens. Deux cas : entre
  nœuds validés, et avec des nœuds locaux versés. Rouges avant, avec le message de la base de
  Lucie.
- **Le journal réel.** La base de code-rag3db-depot de 8 h 45 et son .wal de 9 h 03 (preuve mise de
  côté par G dans `~/.local/share/rag3weaver/preuves/2026-11-08-code-rag3db-depot-wal-dadbf0041/`),
  rejoués sur une copie avec la lib du lot (luciepc, md5 de `librag3db.so` e7571835c64f) : la base
  s'ouvre, l'extension se charge. Le relevé ancré de CONSUMED_BY donne 106 612 relations dans
  chaque sens et 0 écart. Sous 17a094d12 et dadbf0041, le même journal est refusé.

## Lié

- [Le SIGSEGV dans optimisticRead](../2026-11-08-un-sigsegv-dans-optimisticread-sous-lecture-concurrente-pendant-une-indexation.md),
  qui a précédé la mort et le rejeu ; c'est un défaut distinct, encore ouvert.
- Un second défaut, trouvé par le témoin du double versement : G1 sous un second écrivain donne des
  relations aux nœuds de l'autre après le rejeu. Il a son lot à part.
