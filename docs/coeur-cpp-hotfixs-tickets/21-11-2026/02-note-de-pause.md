# Note de pause (21 novembre 2026, 15 h 26) : la tablette redémarre

## Où j'en suis

- **C1 sous TSan** (au mot de c7) : 9 tours de `TwentyWriters.DdlEveryMinute` sur 10 sont joués, et le 10e tourne.
  Au tour 5, TSan nomme **une course neuve dans l'allocateur de pages**. Elle ne se montre dans aucun autre tour :
  `FreeSpaceManager::finalizeCheckpoint`, appelé par le point de reprise automatique **sans `mtx`**, contre
  `addUncheckpointedFreePages`, appelé par un fil de requête qui lâche le dernier segment retiré, **sous `mtx`**. Elle
  est peut-être la cause de C1, sans preuve : le cœur rejouera C1 sur la branche corrigée. Le ticket est déposé au
  pousseur (`ticket-fsm-finalize-race`) :
  `docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-21-le-point-de-reprise-vide-les-pages-libres-non-reprises-sans-verrou.md`.
- **Le correctif et le témoin sont écrits, mais pas appliqués** :
  - le patch est à luciepc, `~/.cache/rag3weaver-build/mesures-hotfixs/patch_fsm_race.py` (`… <arbre> test`, puis
    `… <arbre> fix`) ;
  - le correctif prend `mtx` dans `PageManager::finalizeCheckpoint` et dans `rollbackCheckpoint` ;
  - le témoin est `DataFileExtentTest.PagesFreedWhileACheckpointFinalizesAreNotLost` (`transaction_test`) : 4 fils
    rendent 16 000 pages réelles pendant des `finalizeCheckpoint` en boucle, puis aucune page ne doit manquer.
  - L'ordre des verrous est lu par le MCP : sous `mtx`, `evictPages` ne descend que dans `removePageFromFrame` du
    gestionnaire de tampons, comme le fait déjà `freeImmediatelyRewritablePageRange`, et aucun appelant de
    `freePageRange` ne passe par le gestionnaire de tampons.

## Ce qui tourne à luciepc

- `~/.cache/rag3weaver-build/mesures-hotfixs/c1-tsan/run.sh` (pid 3126301), dans l'arbre
  `~/git_workspaces/hotfixs/21-11-2026-c1-tsan` (master 4fcbe09fc3, détaché, `build/tsan`). Tour 10 en cours ; un
  journal par tour, `~/.cache/rag3weaver-build/mesures-hotfixs/c1-tsan/tour-N.log`. Le résumé des tours est la sortie
  du script ; le ssh qui le portait meurt avec la tablette, mais pas le script.

## Branches et dépôts

- Sur master aujourd'hui : `e22dede600` (rattrapage HNSW en fin de bâti), `3196d6f66f` (îlot d'anciens voisins après
  une suppression), `4fcbe09fc3` (la garde du dictionnaire).
- Au pousseur, en docs : `c1-reading-and-dictionary-guard` (7816f4a2d9) et `ticket-fsm-finalize-race`.
- Aucune branche de code ouverte : le correctif de la course n'est pas encore commité.

## Prochaine action

1. Quand le tour 10 a fini (`FIN` dans la sortie, ou plus de pid `run.sh`) : dans l'arbre `21-11-2026-c1-tsan`, créer
   la branche `fsm-finalize-lock` et appliquer le patch du témoin.
2. Jouer le témoin rouge **en Release et sous TSan**, comme c7 l'a demandé : bâtir `transaction_test` dans un
   `build/release` et dans `build/tsan`.
3. Appliquer le correctif, puis le voir vert dans les deux bâtis.
4. La passe de tête, puis le push par la pause du pousseur et le « à toi » de G.
5. Donner le hash au cœur, qui rejoue vingt tours de C1 dessus.
