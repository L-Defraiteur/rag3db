# Une recherche qui croise une lecture d'état répond par balayage

- **État** : fermé (14 novembre 2026) par le lot `recherche-lit-le-catalogue` ; relevé par archi,
  lot à part sur le mot de c7
- **Famille** : produit
- **Gravité** : faux résultat — une recherche faite pendant qu'une lecture d'état tient le
  catalogue quelques millisecondes répond par le balayage des fichiers (mots exacts, sans score
  ni signaux), alors que l'index est prêt ; la page ne relance pas, le balayage reste affiché
- **Atteignable en service** : oui — `/ide/etat` lit `index_states` à chaque chargement de page de
  l'IDE ; une recherche qui tombe au même instant balaye
- **Touche rag3weaver** : oui (`src/dataflow/generic_search_nodes.rs`)

## Ce que c'est

En mode auto, `SearchSourceNode` lit l'état de l'index de sa cible (et résout sa cible) par un
`catalog.try_lock()` nu : un catalogue tenu, même un instant, donne « index : occupé ou
illisible » et le balayage. `index_states`, lui, prend le même catalogue par
`catalog_read::lock_for_read`, qui attend un verrou court jusqu'à `READ_WAIT` (1 s). Deux lectures
qui se croisent : celle de l'état passe, la recherche balaye.

Trouvé par le témoin `les_vues_de_l_ide_se_rendent_dans_un_vrai_navigateur` sous le lot des thèmes
de l'IDE : une requête de plus au chargement de chaque vue (`/ide/themes.json`) décalait l'ordre
des requêtes, et `search_code` tombait à coup sûr sur le `/ide/etat` du rechargement. Bissection du
13 novembre au soir : vert sur master nu, sur le seul Rust du lot, et sur le bundle sans la
requête ; rouge avec la requête seule.

## Ce qui est attendu

La recherche prend le catalogue par `lock_for_read`, comme `index_states` : une tenue courte
s'attend, bornée à `READ_WAIT`. Au-delà, une indexation entière, le balayage reste (on ne
l'attend jamais), et la réponse dit pourquoi (« catalogue tenu plus de 1 s ») au lieu de
« occupé ou illisible » sans suite.

Témoin : `backend::tests::a_short_hold_of_the_catalog_does_not_turn_a_search_into_a_scan` (le
catalogue tenu 100 ms par un autre fil, la réponse vient de l'index ; tenu 1,5 s, le repli se dit).
