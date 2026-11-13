# L'état des couches relance git à chaque lecture de l'état, et l'ouverture d'une tâche tient le catalogue pendant git

- **État** : ouvert (13 novembre 2026).
- **Famille** : produit
- **Gravité** : lenteur, grandissante — O(suivies²) processus git par lecture de l'état ; le verrou du catalogue tenu pendant des appels non bornés
- **Atteignable en service** : oui, à chaque lecture de `index_states` (la barre de l'IDE, `/etat`, les outils d'état) d'un backend dont un dépôt a des branches suivies déclarées ; et à chaque ouverture de tâche
- **Touche rag3weaver** : oui (`extension/rag3weaver/src/backend.rs`, le bloc `layers` d'`index_states` ; `src/couches.rs` : `etat_des_branches`, `rapport_des_couches` ; `src/daemon_host/taches.rs` : la déclaration de la couche dans `ouvrir_une_tache`)
- **Pour** : arbre principal (la liaison branche → couche)
- **Signalé par** : everything declarative, relecture de `arbre-principal/branches-suivies-2` (598eeb9a1), 13 novembre 2026 au soir, à la demande de c7 ; lignes de cette branche

## Ce que c'est

1. `index_states` appelle `rapport_des_couches` pour chaque racine du backend, à chaque lecture.
   `etat_des_branches` y lance, pour chaque branche suivie et chaque autre candidate, un
   `git merge-base` et plusieurs `git merge-base --is-ancestor`, plus `for-each-ref`,
   `rev-parse` et `rev-list`. C'est peu à trois ou quatre suivies ; c'est O(suivies²) processus par
   lecture, sur une route que l'IDE lit souvent.
2. `ouvrir_une_tache` prend le verrou du catalogue (`b.catalog.lock()`), puis lance, pour chaque
   volet, `git rev-list`, `git rev-parse` et `git for-each-ref`, sans borne de durée. Une lecture ou
   une écriture du catalogue attend ces appels.

## Le témoin à écrire

- Un dépôt de dix branches suivies : deux lectures de l'état de suite, dont la seconde ne lance aucun
  processus git quand aucune pointe n'a bougé. Elle se compte par un compteur d'appels, pas par une
  durée.
- L'ouverture d'une tâche ne tient pas le verrou du catalogue pendant ses appels git : une lecture
  concurrente du catalogue n'attend pas un `git` ralenti par un faux exécutable.

## Ce qui le fermerait

- L'état des couches mémorisé par backend et par ensemble de pointes (`for-each-ref`, un seul
  appel), recalculé seulement si une pointe ou la déclaration change ; ou les fourches de toutes
  les suivies en une seule commande git (`merge-base --all`, ou `for-each-ref` avec
  `--merged`/`--contains`) au lieu d'un processus par paire (c7). Mesurer d'abord : le coût à dix
  suivies, sur un vrai dépôt.
- Les appels git de l'ouverture d'une tâche faits avant de prendre le verrou ; le verrou pris
  seulement pour écrire les clés.
- Les témoins verts.
