# En fichiers, le plein texte compte un document de moins que de lignes

- **État** : ouvert (20 novembre 2026, 05 h), en cours (session lucivy).
- **Gravité** : à établir. Si les bases réelles sont touchées, une base saine rebâtit son plein texte à chaque
  réouverture.
- **Atteignable en service** : à établir. Le stockage du plein texte en fichiers est celui des hôtes.
- **Touche rag3weaver** : oui.
- **Famille** : produit
- **Pour** : lucivy

## Ce qui se passe

Sur master nu (d5b7ce14f7, lib 6e2476d41), trois tests en fichiers d'`e2e_tx_par_paquet_arret` rougissent. Le lot
de recherche qui les porte était vert la veille sous la même lib.
- `en_fichiers_une_base_saine_se_rouvre_sans_rebatir` (l. 850) : « une base saine ne se rebâtit pas ». Le journal
  dit « plein texte de Scope : 300 documents pour 301 lignes — index rebâti depuis les lignes ».
- `en_fichiers_le_rebati_dit_son_etat_et_survit_a_son_interruption` (l. 923) : « plus rien à rebâtir ».
- `la_garde_des_comptes_retire_un_document_sans_ligne_en_fichiers` (l. 1003) : « avant le fantôme, un document par
  ligne », 300 contre 301, juste après une première synchronisation, sans arrêt brutal.

Le pointeur lucivy 4.3.12 (c42baa7c0f) est entré entre-temps, avec 96 autres commits.

## En cours

Départage à luciepc, sous la même lib : les tests « fichiers » sur `c42baa7c0f^` puis sur `c42baa7c0f`. Si le
pointeur est en cause, le changement de la 4.3.12 sur le chemin d'écriture est l'époque de la table des créations
en attente, que le commit des shards fait maintenant tourner (`flush_workers`).
