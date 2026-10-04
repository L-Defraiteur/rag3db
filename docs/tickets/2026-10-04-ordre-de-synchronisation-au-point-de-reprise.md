# L'ordre de synchronisation au point de reprise laisse peut-être des pages non durables

- **État** : ouvert — **non vérifié, à éprouver par un arrêt**
- **Gravité** : perte (si le doute est fondé)
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
