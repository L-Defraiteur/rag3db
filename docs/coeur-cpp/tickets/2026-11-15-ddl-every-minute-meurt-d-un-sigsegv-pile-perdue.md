# DdlEveryMinute meurt d'un SIGSEGV en relisant une colonne ajoutée sous écrivains (master)

- **État** : ouvert (15 novembre 2026, cœur c++)
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

## La piste

La lecture d'un segment d'une colonne ajoutée, dont la taille ne vaut pas celle de son groupe de
nœuds. Un ADD COLUMN posé pendant qu'un écrivain remplit un groupe, sans que le défaut de la
colonne neuve soit étendu au groupe en cours (lecture de c7, 15 novembre).

## Pour fermer

Un témoin qui reproduit la lecture courte hors du scénario, puis le correctif. DdlEveryMinute,
dix tours sans SIGSEGV de cette pile.
