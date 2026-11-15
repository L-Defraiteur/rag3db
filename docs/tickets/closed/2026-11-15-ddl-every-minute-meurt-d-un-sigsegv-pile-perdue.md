# DdlEveryMinute meurt d'un SIGSEGV en relisant une colonne ajoutée sous écrivains (master)

- **État** : corrigé sur master `2c11a7995f` (15 novembre 2026, cœur c++) : le témoin `AddColumnRefusedMidway` rouge puis vert, `DdlEveryMinute` ×10 sans SIGSEGV (4 sur 12 avant)
- **Famille** : moteur
- **Gravité** : plantage (SIGSEGV du processus entier)
- **Atteignable en service** : à établir
- **Touche rag3weaver** : par le moteur (l'hôte meurt)
- **Signalé par** : cœur c++. Passe du lot du verrou de schéma (`alter-attend-les-ecrivains`) ;
  reproduit sur master `a2f9c9b65`, sans le lot.
- **Pour** : cœur c++

## Ce qu'on voit

`TwentyWriters.DdlEveryMinute`, tour 3 sur 3, joué à luciepc le 15 novembre sur `ed2b6ffe6`
(master `a2f9c9b65` plus le lot du verrou de schéma), bâti Release dans
`~/git_workspaces/rag3db-colonnes/build/moteur`. Le processus meurt d'un SIGSEGV à 09:33:29,
après le contrôle `index-creation-refuses-no-reader` (rouge connu) ; il porte le core
systemd-coredump du pid 1604744. Les tours 1 et 2 ont fini sans mourir.

**La pile est perdue.** Le départage qui a suivi a rebâti `concurrence_test` par-dessus le
binaire qui avait produit le core. `coredumpctl debug` ne donne que des adresses sans symboles.
Je n'attribue donc pas ce plantage au SIGSEGV du dictionnaire (ticket
`2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire-d-une-colonne-de-chaines`) :
ce serait sans preuve.

## La pile, puis master

Les tours suivants ont gardé leur binaire à côté de leur journal.

- **Sur le lot** (`b7251cb77`, tour 2 sur 3, core 2034048, 10:42:40, binaire
  `~/.cache/rag3db-moteur-notes/alter7/concurrence_test.garde`) :

```
#0 memcpy (libc)
#1 DefaultColumnReadWriter::readCompressedValuesToVector
#2 Column::scanSegment
#3 Column::scan
#4 ColumnChunk::scan
#5 ChunkedNodeGroup::scan
#6 NodeGroup::scan
#7 NodeTableScanState::scanNext
#8 ScanNodeTable::getNextTuplesInternal
   … Filter, Filter, Projection, SimpleAggregate
```

Un balayage filtré sous un `count`, après la passe, dans les relectures E10 : les comptes de `b`
ou de `rang` hors de leur défaut, deux colonnes ajoutées sous écrivains (minutes 2 et 6).

- **Sur master `a2f9c9b65`, sans le lot**, trois tours, binaire
  `~/.cache/rag3db-moteur-notes/ddl-base/concurrence_test.garde` :
  - tour 1, core 2159864 : le SIGSEGV du dictionnaire au point de reprise (`DictionaryChunk::appendString`
    sous `StringChunkData::finalize`, ticket
    `2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire-d-une-colonne-de-chaines`) ;
  - tour 2, core 2183505 : **la même pile que sur le lot**, `readCompressedValuesToVector` sous
    `NodeGroup::scan` ;
  - tour 3 : fini, avec le rouge connu `index-creation-refuses-no-reader`.

Le défaut est donc sur master, le lot ne le crée pas. Le core de 09:33 avait la même adresse dans
la libc : c'est très probablement le même plantage.

## La cause (15 novembre)

**Le fait.** Dans les quatre tours morts de `DdlEveryMinute` (alter5 t3, alter7 t2, ddl-base t1 et
t2), et eux seuls, « 2 ADD rang DEFAULT 0 » a été refusé par le pool plein (`Buffer manager
exception: Unable to allocate memory!`). Dans les deux tours sains, il a passé.

**Le mécanisme**, lu dans le code. `NodeTable::addColumn` pose la colonne dans `columns`, puis la
pose groupe par groupe (`NodeGroupCollection::addColumn` → `NodeGroup::addColumn` →
`ChunkedNodeGroup::addColumn`), et n'inscrit son annulation (`pushAddColumn`) qu'à la fin. Un refus
à mi-chemin laisse donc la colonne au stockage : dans `columns` et dans les groupes déjà faits.
Le catalogue, lui, l'annule. L'ADD suivant (b, minute 6) se range un cran trop loin dans les groupes
qui portent la morte, et sa relecture tombe dans le morceau de `rang`, un INT64 lu comme une chaîne :
c'est le `memcpy` de `readCompressedValuesToVector`. Le même trou existe pour une table de
relations, entre les deux sens.

**Le premier témoin, sans le pool plein, est vert.** `AddColumnAfterRollback` (la minute 6 seule,
sous huit écrivains, six tours) est vert 5 fois sur 5 : ce n'était ni l'annulation de l'ADD a ni
les écrivains. C'est ce qui a orienté vers l'ADD refusé.

**Le point de faute** `add-column-between-chunked-groups` (`TestFailPoint::ADD_COLUMN_BETWEEN_CHUNKED_GROUPS`,
dans `NodeGroup::addColumn`, après le premier groupe de lignes) refuse l'ADD à mi-chemin, comme le
pool plein.

**Le témoin** `AddColumnRefusedMidway` (`test/transaction/add_column_refused_midway_test.cpp`) : un
ADD refusé à mi-chemin, puis un ADD b avec défaut, qui doit se relire, se modifier et survivre à
un point de reprise et à une réouverture, pour une table de nœuds et une table de relations.

**Le correctif** : tout ou rien à chaque niveau. Le groupe de lignes ne pose la colonne qu'une fois
remplie. Le groupe de nœuds, la collection, la table et les deux sens d'une relation retirent ce
qu'ils ont posé avant de remonter.

Branche `colonne-ajoutee-se-relit` : point de faute `e444ee491`, témoin `db7eed64c`, correctif
`6d557d826`.

## Pour fermer

Un témoin qui reproduit la lecture courte hors du scénario, puis le correctif. DdlEveryMinute,
dix tours sans SIGSEGV de cette pile.

## Fermé (15 novembre 2026)

Sur master `2c11a7995f` (de `d59a56e9e` à `2c11a7995`, voie des suites C++ transférables). Rejeu
de la tête `68769674f` : le témoin 2/2, `transaction_test` entier (387), `e2e_test` (1 867 verts,
`DropNodeTableReclaim` seul, sous ticket). `DdlEveryMinute` ×10 sur `ee598e074` (même moteur) :
aucun SIGSEGV. Les tours qui s'arrêtent encore le font sur le pool plein (ticket du pool qui
sature), et un tour sur un refus sans fin (ticket
`2026-11-15-apres-un-checkpoint-les-ecrivains-sont-refuses-schema-changed-sans-fin.md`).
