# Le banc d'étage meurt sur la garde du changement d'index public

- **État** : ouvert (18 novembre 2026), arbre principal.
- **Famille** : produit
- **Gravité** : un banc de mesure qui ne joue plus depuis le 11 novembre ; ses mesures M1 à M3 (le poids du titre, le
  cosinus exact, le HNSW brut) ne sont plus relevées.
- **Atteignable en service** : non (un test `#[ignore]` de mesure).
- **Touche rag3weaver** : oui (`tests/e2e_banc_etage.rs`, `banc_etage_qui_perd`, ~l. 791-801).
- **Pour** : arbre principal.
- **Signalé par** : arbre principal, en jouant A1 le 18 novembre (`e2e_banc_etage banc_etage_qui_perd`,
  `RAG3WEAVER_BANC_MODELE=granite-278m`, par 7878, lib 59ac9c86bc).

## Ce que c'est

Le banc ré-enregistre `Scope` avec un autre poids de titre (`scope_au_boost`, `cat.register_entity(SCOPE, config)`)
sur un catalogue déjà peuplé. Depuis 8a931f78df (11 novembre, « Garder la représentation publique et le rattrapage
dans leur cellule »), le catalogue refuse ce changement : « Scope : changement d'index public interdit sur une entité
peuplée sans migration déclarée ». Le test meurt là, après 664 s, avant ses mesures.

Ce n'est pas le lot A1 : A1 ne touche ce fichier qu'à partir de la l. ~975, que le test n'atteint plus.

## Le correctif attendu

Le banc ne change plus l'index public d'une entité peuplée. Il pose chaque poids de titre sur un catalogue neuf, ou
déclare la migration que la garde demande, si une telle déclaration existe pour un poids de champ. La garde du
catalogue ne se touche pas.

## Ce qu'il faut pour le fermer

`./run_e2e.sh --test e2e_banc_etage banc_etage_qui_perd` vert à luciepc avec `RAG3WEAVER_BANC_MODELE=granite-278m`,
ses mesures M1 à M3 relevées, dont celle de la table des morceaux de scope lue par `code::scope_chunks` (A1).
