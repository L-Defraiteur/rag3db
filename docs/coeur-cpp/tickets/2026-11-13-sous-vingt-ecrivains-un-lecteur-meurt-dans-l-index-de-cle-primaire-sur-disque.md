# Sous vingt écrivains, un lecteur meurt dans l'index de clé primaire sur disque

- **État** : ouvert (13 novembre 2026, cœur c++)
- **Famille** : moteur
- **Gravité** : plantage (SIGSEGV du processus entier)
- **Atteignable en service** : à établir. Le banc le produit seul, une fois sur trois ; un hôte qui
  lit pendant des écritures concurrentes, des DDL et un pool de tampons plein y ressemble.
- **Touche rag3weaver** : par le moteur (l'hôte meurt)
- **Signalé par** : cœur c++ — banc décrit dans ce ticket
- **Pour** : cœur c++

## Ce qu'on voit

`TwentyWriters.DdlEveryMinute` (`test/transaction/concurrence/twenty_writers_scenario_test.cpp`),
joué seul trois fois à luciepc le 13 novembre sur `d5aa49c49` (master `2ac8080ca` plus deux témoins
de test, sans code moteur en plus) :

| Tour | Verdict |
|---|---|
| 1 | rouge : la saturation du pool (ticket `2026-11-13-sous-vingt-ecrivains-le-pool-sature-et-ne-se-vide-plus.md`) |
| 2 | vert |
| 3 | SIGSEGV après 36 s |

Le fil fautif du tour 3 (core gardé par systemd-coredump à luciepc, pid 3911689, binaire
`~/git_workspaces/rag3db-colonnes/build/moteur/test/transaction/concurrence/concurrence_test`) :

```
#0 rag3db::storage::DiskArrayInternal::get(unsigned long, Transaction const*, std::span<std::byte>)
#1 rag3db::storage::HashIndex<long>::lookupInPersistentIndex(...)
#2 rag3db::storage::HashIndex<long>::lookupInternal(...)
#3 rag3db::storage::PrimaryKeyIndex::lookup(...)
#4 rag3db::storage::NodeTable::lookupPK(...)
#5 rag3db::processor::PrimaryKeyScanNodeTable::getNextTuplesInternal(...)
#7 rag3db::processor::ScanRelTable::getNextTuplesInternal(...)
#13 rag3db::processor::HashJoinBuild::executeInternal(...)
```

Une lecture par clé primaire, dans le côté construction d'une jointure, lit un tableau disque de
l'index de hachage (les pages d'emplacements ou leurs PIP) et sort de la mémoire. Juste avant, le
journal du banc dit « Checkpoint postponed: other transactions are open » (16 Mio de journal).

## Ce que dit le core (13 novembre, 22 h 15)

Le core du 13 novembre à 22 h 04 m 27 s (pid 3911689, gardé par systemd-coredump, extrait dans
`/tmp/core-ddl-3911689` à luciepc) : adresse fautive `0x8`, instruction `mov 0x8(%rdx,%rdi,4)` avec
`rdx = 0` et `rdi = 0`. `DiskArrayInternal::get` lit donc `pips[0]` dans un vecteur de PIP **vide**
(tampon nul). Ce n'est pas la famille du 8 novembre (un numéro de page au-delà du fichier, que la
garde de `9bae60786` change en erreur) : le tableau disque est cru non vide par un lecteur alors que
ses PIP ne sont pas encore dans `pips`.

Pour une transaction de lecture, `getAPPageIdxNoLock` (`src/storage/disk_array.cpp`) ne lit que
`pips` ; les PIP neufs restent dans `pipUpdates.newPIPs` jusqu'à ce que le point de reprise les y
verse. L'en-tête de l'index que lit le lecteur se lit hors du verrou du tableau. Deux voies : le
point de reprise V2, qui laisse lire pendant qu'il s'écrit, publie l'en-tête avant les PIP ; ou une
annulation laisse l'en-tête et les PIP incohérents.

## Les questions

1. Le tableau disque de l'index de clé primaire est-il lu pendant qu'un écrivain ou un point de
   reprise le change (ses PIP, `DiskArrayInternal::checkpoint`, les pages ajoutées par
   `pushBack`) ? Le cas V2 (lecteurs pendant le point de reprise) a-t-il couvert l'index de hachage
   comme les colonnes ?
2. Une page refusée par le pool plein pendant la lecture laisse-t-elle un état à moitié lu ?
3. Le lien avec le SIGSEGV de `TwentyWriters.KilledDuringTheCheckpoint` vu dans le banc entier du 9
   novembre (vert seul) : à départager.

## Témoin

`TwentyWriters.DdlEveryMinute` lui-même, une fois sur trois. Un témoin réduit reste à écrire : des
lectures par clé primaire en boucle pendant que des écrivains insèrent dans la même table et qu'un
point de reprise passe, sous ASAN.

## Pour le fermer

La cause sous ASAN ou par un témoin réduit rouge, le correctif, le témoin vert.
