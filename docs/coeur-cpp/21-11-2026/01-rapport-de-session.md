# Rapport de session du cœur c++ — note de pause (21 novembre 2026, 15 h 30)

Pause demandée par c7 : redémarrage de la tablette. Rien de moi ne tourne à la tablette.

## Où j'en suis : C1, le dictionnaire au point de reprise

Ticket : `docs/coeur-cpp/tickets/2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire-d-une-colonne-de-chaines.md`.

**La sonde a déclenché à 15 h 25 (luciepc), au premier tour**, au point de la relecture (`DictionaryColumn::scan`,
appelée par `Column::checkpointSegment`) :

```
[sonde C1 scan] dictionnaire relu incohérent : offset[1] = 6928 (précédent 25970), octets 106496, offsets 8938
DATA numValues avant l'agrandissement 106496, après le scanSegment 106496 ; OFFSET numValues 8938 ;
capacité des octets avant 0, après 131072 ; numValuesPerPage de l'état DATA 4096, OFFSET 1927
DATA : numValues 106496, pages 2434..+26, compression 0, min 32, max 116
OFFSET : numValues 8938, pages 2460..+5, compression 1, min 0, max 106483
offset[8934..8937] = 2073, 41990, 95027, 84897
```

Lecture : les **métadonnées** des deux enfants sont cohérentes entre elles (106 496 octets, 8 938 offsets, un maximum
annoncé de 106 483, sous les octets). Ce sont les **pages d'offsets sur disque** (2460..+5) qui ne portent pas des
offsets : les valeurs relues ne croissent pas. Une page vivante a donc été réécrite par autre chose : la forme d'une
page rendue libre puis redonnée alors qu'un segment la tenait encore. Cela rejoint la course que hotfixs a trouvée
sous TSan le 21 (tour 5) : `FreeSpaceManager::finalizeCheckpoint` vide `uncheckpointedFreePageRanges` sans `mtx`
pendant qu'un fil qui détruit un vieux morceau y fait `push_back` sous `mtx`. Pas encore prouvé.

Core : pid 4013013 (SIGABRT, systemd-coredump, luciepc) ; binaire
`~/git_workspaces/coeur/20-11-2026-bati/build/rwdi/test/transaction/concurrence/concurrence_test` (437e8a841f).
Journal : luciepc `~/.cache/rag3db-moteur-notes/c1-scan/` (`passe.out`, `tour-1.log`).

## Ce qui tourne à luciepc

Rien : la série (`~/.cache/rag3db-moteur-notes/c1-scan-luciepc.sh`) s'est arrêtée d'elle-même au premier
déclenchement (FIN à 15 h 26).

## Branches et têtes

- `c1-scan-probe` = 437e8a841f : les deux sondes (relecture et avant write), jamais pour master.
- `c1-probe` = ba5ccce329 : l'ancienne sonde dans `StringChunkData::write`, close.
- Atterri : le train du 20 (master 44c14e1c38, la course des groupes en 4ad394f4d2) ; le ticket de la clé primaire
  fermé (f33426d941) ; `coeur-c1-ticket-trace` (74e788ced2) déposé au pousseur le 21 au matin.

## Ce qui attend

- Rien au pousseur de ma part, sauf `coeur-c1-ticket-trace` s'il n'est pas encore passé.
- La branche de hotfixs qui verrouille `FreeSpaceManager::finalizeCheckpoint` : son hash, quand il me le donne.

## Prochaine action

1. Écrire au ticket C1 le déclenchement de 15 h 25 (cette note), par le pousseur.
2. Lire le core 4013013 : qui d'autre a tenu ou réécrit les pages 2460..2464 (les plages libres du
   `FreeSpaceManager` dans le core).
3. Rejouer la série sondée, vingt tours, sur la branche de hotfixs avec le verrou, dans le même arbre de bâti : si
   la sonde ne déclenche plus là où elle déclenche au premier tour sans le verrou, le lien est fait.

Worktrees tenus : `coeur/19-11-2026-docs` (tablette, docs) ; `coeur/20-11-2026-bati` (luciepc, arbre de bâti,
build/rwdi sur 437e8a841f).
