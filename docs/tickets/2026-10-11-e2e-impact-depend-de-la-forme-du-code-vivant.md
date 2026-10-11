# e2e_impact dépend de la forme du code vivant qu'il indexe

- **État** : ouvert (11 octobre 2026). À faire par codeparsers ou par l'arbre principal, après la
  migration (a) de la marche 1 bis, pas maintenant (orchestration).
- **Gravité** : test fragile. Un rouge qui ne dit rien du produit, et qui pousse à contorsionner le code.
- **Atteignable en service** : non (c'est le témoin).
- **Touche rag3weaver** : oui (`tests/e2e_impact.rs`).

## Ce que c'est

`e2e_impact::impact_nomme_les_tests_que_des_changements_reels_ont_casses` indexe ce dépôt et attend que
l'impact d'un changement de `split_unchanged` nomme le test
`une_premiere_ingestion_passe_par_la_masse_et_la_seconde_par_le_merge`. L'attente porte donc sur un
**chemin précis** à travers une fonction vivante du produit (`Catalog::ingest_entities_jusqu_a`).
Le pas b de la marche 1 bis (`bac552c22`) a déplacé le corps de l'ingestion dans une fonction à part :
un saut de plus, l'impact ne remontait plus jusqu'au test, et le témoin a rougi sur master sans que
rien ne soit faux dans l'impact. Le correctif (`f9dc86564`) a rendu au code sa profondeur, en faisant
s'appeler l'ingestion elle-même dans sa cellule : du code tordu pour un test.

## Pour le fermer

La règle du dépôt depuis `e2e_code` et le banc : **un module-corpus à part, stable, choisi pour ça**,
plutôt qu'un fichier vivant. L'attente d'impact exacte (tel changement nomme tel test, à telle
profondeur) porte sur ce module. Le test sur le code vivant ne vérifie que des propriétés qui survivent
à une extraction de fonction : le test est atteint, à une profondeur inférieure ou égale à N, jamais par
un chemin exact. Ensuite, `ingest_entities_jusqu_a` peut reprendre la forme la plus lisible.
