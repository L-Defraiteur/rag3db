# Un index entier se fige pendant un checkout, et la mémoire de l'hôte monte

- **état** : ouvert (11 octobre 2026, vu sur luciepc, non reproduit)
- **gravité** : blocage (vecteurs figés un quart d'heure), mémoire (+0,5 Go par minute)
- **atteignable en service** : oui — un checkout, ou une lib partagée remplacée, pendant un index entier
- **touche rag3weaver** : oui

## Ce que c'est

Sur luciepc, pendant l'index entier de `code-lourd` (136 000 morceaux), le cœur a fait un
checkout de l'arbre indexé et remplacé la lib partagée sous l'hôte. Les vecteurs se sont
figés à 18 % pendant un quart d'heure : le fil d'indexation tournait sans avancer, et la
mémoire résidente de l'hôte est montée de 9,4 à 15,9 Go. L'hôte a été redémarré.

## Ce qu'on sait

- La montre, pendant un index entier, met chaque fichier changé **en reprise**
  (`code_sync::note_change`, registre clé par racine) ; la synchronisation le relit à son
  paquet ou à la fin. Ce registre est un ensemble de chemins : il n'explique pas des Go.
- La phase des vecteurs lit des **morceaux en base** et les envoie à l'embarqueur ; elle ne
  relit pas les fichiers. Un checkout seul ne devrait pas la figer.

## Les suspects, à départager

1. **La lib partagée réécrite en place** sous un processus qui l'a chargée (même inode) :
   les pages mappées changent sous lui, comportement indéfini. Un remplacement par
   renommage (inode neuf, ce que fait l'arbre principal) ne le fait pas.
2. **L'embarqueur** bloqué ou relancé pendant le checkout, et le fil qui réessaie sans
   avancer ; la mémoire monterait par la file des morceaux en attente.
3. **La reprise** : des milliers de fichiers en reprise pendant la phase des vecteurs.

## Pour le reproduire et le départager

Un index entier sur un arbre de plusieurs milliers de fichiers ; pendant les vecteurs, un
`git checkout` qui change 500 fichiers, **sans** toucher à la lib partagée ; puis la même
chose en remplaçant la lib par renommage, puis en place. Pendant le figement : la pile des
fils (`gdb -batch -ex "thread apply all bt" -p <pid>`), `index_state` (progression des
vecteurs, `watches`, `lag`), et la mémoire par `/proc/<pid>/smaps_rollup`.

## Pour le fermer

L'index finit proprement et la reprise rattrape les fichiers changés (ou, si la reprise
est la cause, l'index finit son paquet, le valide et s'arrête en le disant, la
réconciliation reprenant au démarrage) ; la mémoire ne monte pas sans progrès. Témoin :
index entier en cours, un checkout de 500 fichiers, l'index finit, `usages` voit les
changements.
