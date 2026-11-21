# Des identifiants en français à renommer avant le multilingue

- **État** : ouvert (21 novembre 2026, 11 h, orchestration).
- **Gravité** : dette de nommage — aucun défaut de comportement.
- **Atteignable en service** : non (les noms ne changent rien à ce que font les outils).
- **Touche rag3weaver** : oui (routes de l'hôte, fichiers d'état, fonctions, commandes).
- **Famille** : produit
- **Pour** : à attribuer par l'orchestration, en un seul lot de refacto, avant toute livraison multilingue.
- **Signalé par** : Lucie, 21 novembre 2026, 11 h : « je préférerais le code en anglais partout dans un premier
  temps, mais tant pis ; le truc c'est que tu te fais influencer par mes demandes qui sont en français ; on fera une
  refacto donc obligatoire le jour où on livrera le produit vers le multilingue ».

## Ce que c'est

La règle (6 septembre puis 16 novembre 2026) veut tout identifiant en anglais : routes, fichiers, clés, fonctions,
commandes, outils, tests ; la prose, les commits et les tickets restent en français. Les demandes de Lucie, et les
consignes de l'orchestration qui les reprenaient, étant en français, des identifiants français sont entrés sur master.
Depuis le 21 novembre à 11 h, tout identifiant neuf est en anglais ; ce ticket recense l'existant.

## Relevé sur master (21 novembre 2026, 0f6cb9b7e7)

Routes de l'hôte : `/depots/resoudre`, `/depots/retirer`, `/dossiers/ouvrir`, `/etat`, `/ide/bilans`, `/ide/etat`,
`/ide/ouvrir`, `/ide/poste`, `/ide/poste/etat`, `/poste`, `/ide/reactions`, `/ide/runs/arreter`, `/ide/runs/epingler`,
`/ide/taches`, `/ide/taches/fusionner`, `/ide/taches/rejeter`. À relever au moment du lot : la route des diffs des
vues (en passe), les fichiers d'état (`sans-index.json`, `atterrissages.json`, `dossiers-des-sessions.json`…), les
fonctions et modules en français (`ouvrir_un_dossier`, `poser_l_atelier`, `fermer_une_tache`, `taches.rs`…), et les
clés de manifeste.

## Ce qu'il faut

Un lot de refacto : chaque identifiant renommé en anglais, avec l'ancien nom accepté en alias le temps d'une version
pour les routes et les commandes (les clients et les scripts des postes s'en servent), et les fichiers d'état lus sous
les deux noms puis réécrits sous le nouveau. Les textes affichés à l'écran ne sont pas des identifiants : ils relèvent
de la graine de la langue parlée.

## Témoin

Un test qui parcourt les routes déclarées de l'hôte et refuse un segment hors de l'ASCII anglais ou figurant dans une
liste de mots français connus ; rouge sur master, vert après le lot.
