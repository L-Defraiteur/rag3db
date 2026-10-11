# e2e_impact dépend de la forme du code vivant qu'il indexe

- **État** : ouvert (11 octobre 2026). À faire par codeparsers ou par l'arbre principal, après la
  migration (a) de la marche 1 bis, pas maintenant (orchestration).
- **Gravité** : test fragile. Un rouge qui ne dit rien du produit, et qui pousse à contorsionner le code.
- **Atteignable en service** : non (c'est le témoin).
- **Touche rag3weaver** : oui (`tests/e2e_impact.rs`, `tests/e2e_usages.rs`).

## Ce que c'est

`e2e_impact::impact_nomme_les_tests_que_des_changements_reels_ont_casses` indexe ce dépôt et attend que
l'impact d'un changement de `split_unchanged` nomme le test
`une_premiere_ingestion_passe_par_la_masse_et_la_seconde_par_le_merge`. L'attente porte donc sur un
**chemin précis** à travers une fonction vivante du produit (`Catalog::ingest_entities_jusqu_a`).
Le pas b de la marche 1 bis (`bac552c22`) a déplacé le corps de l'ingestion dans une fonction à part :
un saut de plus, l'impact ne remontait plus jusqu'au test, et le témoin a rougi sur master sans que
rien ne soit faux dans l'impact. Le correctif (`f9dc86564`) a rendu au code sa profondeur, en faisant
s'appeler l'ingestion elle-même dans sa cellule : du code tordu pour un test.

## Le même mal dans e2e_usages (11 octobre 2026, 17 h)

`e2e_usages::usages_rend_la_definition_et_les_appels_releves_a_la_main` indexe `src/dataflow` et
attend les fichiers qui appellent `merge_port_values`, relevés à la main : {`port.rs`, `runtime.rs`}.
Deux vrais appels sont arrivés par des lots sans rapport, `node.rs:481` (le fan-in d'une entrée,
`9521ab63c`) et `concurrent.rs:341` (l'exécuteur concurrent, `439ca3c9c`). Le témoin a rougi sur
master à 16 h 34, pour toutes les sessions qui le rejouaient, alors que l'outil disait vrai. Le relevé
a été mis à jour à la main (`6a5c19b03`), avec un commentaire qui dit où relire : c'est un pansement,
le prochain appel ajouté dans `src/dataflow` le rougira de nouveau.

La même forme ferme les deux : les comptes exacts sur le module-corpus stable, et sur le code vivant
des propriétés qui survivent (la définition est trouvée ; les appels relevés à la main en sont **un
sous-ensemble** ; un fichier qui ne nomme la fonction que dans une chaîne ou un commentaire n'en est
jamais).

## Pour le fermer

La règle du dépôt depuis `e2e_code` et le banc : **un module-corpus à part, stable, choisi pour ça**,
plutôt qu'un fichier vivant. L'attente d'impact exacte (tel changement nomme tel test, à telle
profondeur) porte sur ce module. Le test sur le code vivant ne vérifie que des propriétés qui survivent
à une extraction de fonction : le test est atteint, à une profondeur inférieure ou égale à N, jamais par
un chemin exact. Ensuite, `ingest_entities_jusqu_a` peut reprendre la forme la plus lisible.
