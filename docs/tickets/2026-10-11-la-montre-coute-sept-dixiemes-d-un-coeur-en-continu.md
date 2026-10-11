# La montre de fichiers coûte sept dixièmes d'un cœur en continu

- **état** : ouvert (11 octobre 2026, mesuré sur l'hôte de Lucie, `code-rag3db`)
- **gravité** : coût (un cœur pris en permanence sur le poste de travail)
- **atteignable en service** : oui, dès qu'un manifeste déclare `watches` (le gabarit `code` le fait)
- **touche rag3weaver** : oui (et seulement)

## Ce que c'est

Sur l'hôte de Lucie (pid 107223, binaire e9387d545), le fil `montre-code` prend
**69 tops d'horloge en 10 s, soit environ 69 % d'un cœur**, en continu, au repos : aucun
appel, aucune édition. La montre balaie les 7 310 fichiers du workspace
(`every_ms: 1000`, `settle_ms: 300`, ce que le gabarit déclare) : un parcours complet du
disque, règles d'exclusion comprises, chaque seconde.

Ce n'est pas un blocage : le balayage ne tient pas le verrou du backend (`/describe`
reste à 3–10 ms pendant qu'il tourne), et une édition est vue en 4,7 à 6 s. C'est un coût,
sur un poste où la compilation et les passes sont déjà ce qui gêne.

## Ce qu'il faut pour le fermer

Une période mesurée, pas posée : le coût d'un balayage pour N fichiers, et la période qui
garde une édition vue en quelques secondes pour un coût négligeable — ou un balayage moins
cher (ne relire les règles d'exclusion qu'à leur changement, `stat` seul hors des
fichiers qui ont bougé), ou les notifications du noyau derrière la même interface, que la
doc de `watch.rs` prévoit « si une mesure le demande ». Cette mesure-ci le demande. Témoin :
le CPU du fil sur 60 s au repos, sous un seuil dit, sur un workspace de la taille de
celui-ci. Chez le proto (la montre est à lui).
