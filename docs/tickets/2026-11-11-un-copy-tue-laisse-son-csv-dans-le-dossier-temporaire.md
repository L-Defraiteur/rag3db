# Un COPY tué laisse son CSV dans le dossier temporaire

- **État** : ouvert (11 novembre 2026, session recherche), résidu intermittent observé dans la passe du lot arret-du-backend.
- **Famille** : produit
- **Gravité** : fichiers temporaires abandonnés après un arrêt brutal ; le témoin des résidus rougit.
- **Atteignable en service** : oui, si le processus est tué entre la création du CSV et son effacement.
- **Touche rag3weaver** : oui (`src/dataflow/record_nodes.rs`, `tests/e2e_tx_par_paquet_arret.rs`).

## Observation et preuve

La première passe e2e entière du lot arret-du-backend à luciepc laisse trois CSV de 12 Ko dans le TMPDIR de `e2e_tx_par_paquet_arret` :

- `rag3weaver-lignes-1369958-42-File-1794234704051.csv` ;
- `rag3weaver-lignes-1375887-42-File-1794234724462.csv` ;
- `rag3weaver-lignes-1377772-42-File-1794234731332.csv`.

Preuve relue le 11 novembre : `~/.cache/rag3weaver-build/journaux-arret/e2e-entier-1.log`, lignes 8457–8459, à luciepc. Le second e2e entier ne laisse pas ces CSV, selon le rapport de reprise. Ces résultats sont historiques ; aucune nouvelle passe n'est revendiquée par ce ticket.

## Cause

`fichier_csv` crée les fichiers dans `std::env::temp_dir()`. Les chemins des lignes et des liens ne sont effacés qu'après le retour de `conn.execute` du COPY. La suite d'arrêt brutal tue volontairement ses processus par SIGKILL : l'effacement ne peut alors pas s'exécuter. Ce défaut est distinct des fils du backend qui écrivent après son Drop.

La suite est déclarée comme résidu intermittent connu avec ce ticket ; cela ne corrige pas la cause et ne dispense pas des autres assertions de la suite.

## Pour fermer

Donner aux CSV de COPY un dossier dont le nettoyage survit à la mort du producteur (par exemple un espace de travail lié à la base, nettoyé à la reprise sans toucher aux processus vivants). Rejouer le témoin d'arrêt brutal et prouver plusieurs passes propres, puis retirer la suite de la liste des résidus connus dans le même lot.
