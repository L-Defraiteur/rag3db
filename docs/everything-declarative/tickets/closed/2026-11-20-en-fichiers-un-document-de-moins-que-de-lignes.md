# En fichiers, le plein texte compte un document de moins que de lignes

- **État** : fermé le 20 novembre 2026 par la session everything declarative, dans le lot du correctif (`fts-count-guard-candidates`). Voir « Fermeture ».
- **Gravité** : à établir. Si les bases réelles sont touchées, une base saine rebâtit son plein texte à chaque
  réouverture.
- **Atteignable en service** : à établir. Le stockage du plein texte en fichiers est celui des hôtes.
- **Touche rag3weaver** : oui.
- **Famille** : produit
- **Pour** : everything declarative (ouvert par lucivy, passé à ED sur ordre de c7)

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

## Fermeture (20 novembre 2026, session everything declarative)

**La cause : 2.12, pas lucivy.** G l'a départagé par la répétition : baccb2ea6e, avec lucivy 4.3.12 mais sans 2.12, ne
montre aucun écart ; 1f22743e17, avec 2.12, rebâtit à la réouverture (« 67 610 documents pour 75 246 lignes ») et
finit l'index sur un écart (« 94 469 documents pour 103 999 lignes »).

2.12 laisse sans document de plein texte, par construction, un conteneur replié qui n'est pas candidat à la recherche.
Sa ligne reste dans la table, avec ses arêtes. Or la garde des comptes (`Catalog::check_fts_counts`, et
`comptes_du_plein_texte` qui alimente l'état) comparait les documents à **toutes** les lignes de la table : chaque
conteneur écarté faisait un document de moins, et l'index se rebâtissait à chaque ouverture. Le corpus des tests en
fichiers a un module replié, d'où « 300 pour 301 ». Mes témoins de 2.12 ne rouvraient jamais une base : c'est le trou
qui l'a laissé passer.

**Le correctif.**
- La garde compare les documents aux lignes **candidates** : toutes, moins celles que la règle des non-candidats
  écarte.
- Les lignes écartées sont lues par une seule fonction (`Catalog::non_candidate_uuids`), partagée par la garde et par
  le geste de 2.12. Elle lit aussi les champs de structure et de langue que la règle juge : le geste ne les lisait pas,
  si bien qu'il jugeait sans la règle de structure, et autrement que l'insertion.
- Une base indexée avant 2.12 a encore les documents de ses conteneurs à sa première ouverture : la garde y voit un
  écart et rebâtit une fois depuis les lignes, sans eux. Les ouvertures suivantes ne rebâtissent rien.

**Les témoins.**
- Les trois tests en fichiers d'`e2e_tx_par_paquet_arret` : rouges sur master 877019bdf2 (« 300 documents pour 301
  lignes »), verts avec le correctif.
- `backend::tests::a_base_with_containers_set_aside_keeps_its_full_text_at_its_reopening` : une base de code dont les
  namespaces repliés sont écartés. Rouge sans le correctif (13 documents pour 26 lignes), vert avec : aucun écart, et la
  garde de réouverture ne rebâtit rien.
