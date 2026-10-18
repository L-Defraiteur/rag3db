# La montre perdait un fichier créé pendant sa liste (le « racy git »)

- **État** : fermé (18 octobre 2026, la nuit), dans le commit qui l'ouvre
- **Gravité** : index faux en silence — un fichier créé pouvait rester absent de l'index jusqu'au
  filet de relecture (60 s), sans que rien ne le dise
- **Atteignable en service** : oui, dès qu'un fichier est créé pendant qu'un balayage lit la liste
  (un checkout, une génération, plusieurs fichiers écrits d'affilée)
- **Touche rag3weaver** : oui (`watch.rs`)
- **Ouvert le** : 18 octobre 2026, everything declarative, en départageant le témoin instable
  `backend::tests::the_watch_does_not_index_what_the_index_excludes`
  (ticket `2026-10-18-trois-temoins-rougissent-sous-charge.md`)
- **Pour** : everything declarative

## Ce qu'on a vu

Le témoin seul, sous charge (la lib entière en fond et seize boucles CPU), vingt passes, lib
87a362e30, avec un diagnostic en cas de rouge : **3 rouges sur 20**. Chaque fois :

- `admis.rs` sur le disque, **absent de la base** ;
- la montre à `lists: 1` pour 17 balayages ;
- 1 à 3 exclus vus sur 3 : la liste avait été lue au milieu des créations.

Ce n'était pas une famine de la réaction : la montre avait perdu le fichier.

## La cause

`Scanner::remember_dirs` lisait la date des dossiers **après** la liste de la source. Un fichier
créé entre les deux changeait la date du dossier sans être dans la liste. Aux tours suivants,
`dirs_moved` trouvait la date inchangée, et la liste n'était plus relue avant le filet
(`relist_ms`, 60 s). C'est le « racy git » : une date qui ne prouve rien parce qu'elle n'est pas
plus vieille que l'instant où l'on a regardé. La granularité de la date d'un inode (un tic du
noyau) en donne une seconde forme : un fichier créé dans le même tic qu'une lecture ne la change
pas.

## Le correctif

Une date de dossier qui n'est pas plus vieille que le début de la liste moins `DATE_DOUTEUSE`
(2 s) est tenue pour douteuse (`None`), et le dossier se relit au tour suivant, jusqu'à ce que sa
date ait vieilli. La marge couvre la course et la granularité ; elle ne coûte que des relectures
pendant les deux secondes qui suivent un changement de dossier.

## Les témoins

- `watch::tests::a_file_created_between_the_list_and_the_dates_is_not_lost` : déterministe, un
  crochet de test (`apres_la_liste`) crée le fichier exactement entre la liste et les dates.
  Rouge sans le filtre des dates douteuses, vert avec.
- `watch::tests::a_quiet_scan_does_not_relist_the_source` part d'un arbre vieilli et compte la
  relecture que fait une date douteuse.
- Le témoin d'origine sous la même charge : **3 rouges sur 20 avant, 0 sur 20 après.**
