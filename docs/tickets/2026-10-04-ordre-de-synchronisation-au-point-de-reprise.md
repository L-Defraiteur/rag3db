# L'ordre de synchronisation au point de reprise laisse peut-être des pages non durables

- **État** : corrigé le 4 octobre 2026, sans témoin — le doute tient (lu dans le code, banc)
- **Gravité** : perte (sur coupure de courant ou arrêt du noyau)
- **Atteignable en service** : inconnu ; il faudrait une coupure de courant ou un arrêt du noyau, pas la mort d'un processus
- **Touche rag3weaver** : oui si le doute est fondé (ses chargements passent par `COPY`)
- **Ouvert le** : 4 octobre 2026, session cœur C++ (lecture du code par un agent de repérage, non vérifiée)
- **Pour** : cœur C++ (journal, point de reprise)

## Ce que c'est

Un doute de durabilité, pas un défaut établi : le fichier de données ne serait synchronisé sur disque qu'après que l'enregistrement de point de reprise a été écrit et vidé dans le journal. Les pages qu'un `COPY` écrit directement dans le fichier de données pourraient alors ne pas être durables au moment où le journal dit que le point de reprise est fait.

## Recette minimale

Aucune : la mort d'un processus ne distingue pas ce cas (le cache du noyau survit). Il faut une coupure simulée — par exemple un système de fichiers d'essai qui abandonne les écritures non synchronisées.

## Témoin

Aucun.

## Cause

À lire avant tout : `src/storage/shadow_file.cpp` (vers la ligne 79, la synchronisation du fichier de données), `src/storage/checkpointer.cpp` (vers la ligne 158, l'enregistrement de point de reprise), `src/storage/wal/wal.cpp` (lignes 39-45). Non vérifié : qu'aucune autre écriture ne synchronise le fichier de données plus tôt.

## Correctif de l'amont

Ladybug `e2ada3f90` (6 août 2026) synchronise le dossier parent après le retrait du journal ; il ne traite pas ce point.

## Ce qu'il faut pour le fermer

Lire les trois endroits et dire si le doute tient ; s'il tient, synchroniser le fichier de données avant d'écrire l'enregistrement de point de reprise, et l'éprouver.

## Ce que la lecture a établi (4 octobre 2026, banc)

Le doute tient, pour une coupure seulement (une mort de processus laisse le cache du
noyau intact).

- Pendant `checkpointStorage`, les pages neuves des colonnes s'écrivent directement dans
  le fichier de données, sans page fantôme : `compression_flush_buffer.cpp`
  (`dataFH->writePagesToFile`, lignes 19, 47, 54, 110), et les débordements
  (`overflow_file.cpp:226`).
- Le catalogue, les métadonnées et l'en-tête qui désignent ces pages passent par le
  fichier fantôme (`InMemFileWriter::flush`, `Checkpointer::writeDatabaseHeader`) ; le
  fichier fantôme est synchronisé par `ShadowFile::flushAll`.
- La seule synchronisation du fichier de données d'un point de reprise était dans
  `ShadowFile::applyShadowPages`, **après** `WAL::logAndFlushCheckpoint`.

Une coupure entre la marque CHECKPOINT et cette synchronisation laisse un journal durable
clos par CHECKPOINT. La reprise rejoue l'en-tête et les métadonnées, qui désignent des
pages directes peut-être jamais écrites, puis supprime le journal : les transactions
validées depuis le point de reprise précédent sont perdues.

Les amonts ont le même ordre (Ladybug `ladybug-main-2026-08-31`, Vela
`vela-master-2026-09-03`, `logCheckpointAndApplyShadowPages`) : pas de correctif à lire.
Le chargement en masse journalisé n'en dépend pas : il journalise les lignes, et le
rejeu ignore les pages directes que le dernier point de reprise ne connaît pas (session
cœur C++).

## Correctif

`Checkpointer::logCheckpointAndApplyShadowPages` synchronise le fichier de données après
`flushAll` et avant la marque CHECKPOINT. La synchronisation d'`applyShadowPages` reste,
pour les pages recopiées. Le chronométrage (`RAG3DB_PROFILE_CHECKPOINT`) lui donne son
poste : `synchro-donnees`.

Coût mesuré (Release, base sur btrfs, cinq passes de chaque côté, médianes) :

| point de reprise | avec | sans | écart |
|---|---|---|---|
| après 200 000 lignes | 167 ms | 146 ms | + 22 ms (+ 15 %) |
| après une ligne (moyenne de 50) | 7,4 ms | 6,0 ms | + 1,4 ms (+ 23 %) |

## Ce qui manque pour le prouver

Un témoin demande un système de fichiers d'essai qui abandonne les écritures non
synchronisées (une coupure simulée), puis une coupure entre la marque CHECKPOINT et
l'application des pages fantômes. Le même crochet servirait au rejeu des pages fantômes
(ticket de durabilité).
