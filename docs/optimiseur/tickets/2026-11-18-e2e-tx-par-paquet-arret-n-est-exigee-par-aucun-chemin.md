# `e2e_tx_par_paquet_arret` n'est exigée par aucun chemin

- **État** : ouvert (18 novembre 2026) — attend le commit fautif trouvé par ED
- **Famille** : produit (outillage : `tools/poste/required_suites.txt`)
- **Gravité** : un rouge de master non vu. 11 tables d'arêtes diffèrent à la reprise d'un gros paquet défait.
- **Atteignable en service** : oui (la reprise après un arrêt pendant un gros paquet)
- **Touche rag3weaver** : non (la table des suites exigées)
- **Pour** : G (optimiseur)
- **Signalé par** : c7, 18 novembre 2026, après le rouge vu à luciepc (`passe-garde-115633/e2e_tx_par_paquet_arret.log`, 12 h 15)

## Ce qu'on voit

`e2e_tx_par_paquet_arret` était verte le 16 novembre à 14 h 22 (journaux de recherche, `fts-existing-canon-v1`), et rouge
le 18 à 12 h 15 et à 12 h 30 à luciepc. `tools/poste/required_suites.txt` n'exige que `e2e_code_sync`. Aucune passe de
lot ne jouait cette suite.

## Ce qu'il faudrait, après le commit fautif d'ED

Une ligne `e2e_tx_par_paquet_arret` dans `tools/poste/required_suites.txt`, sur les chemins de l'écrivain et de la
reprise : `extension/rag3weaver/src/code.rs`, `code_sync.rs`, `generations.rs`, `catalog.rs` et `catalog/*`, et le
fichier de la suite elle-même. Les chemins de la reprise sont à relever dans le commit fautif et dans les sources que la
suite exerce. Témoin dans `tools/poste/test_push_garde.sh` : un lot qui touche l'un de ces chemins sans avoir joué la
suite est arrêté ; rouge sur la table d'avant.
