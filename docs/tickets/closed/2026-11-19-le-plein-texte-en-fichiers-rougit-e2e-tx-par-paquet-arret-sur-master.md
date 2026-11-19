# Le plein texte en fichiers rougit e2e_tx_par_paquet_arret sur master

- **État** : fermé (19 novembre 2026, session recherche) : C est revenue avec sa durabilité.
- **Ouvert le** : 19 novembre 2026, par ED.
- **Famille** : produit
- **Gravité** : une reprise après arrêt rend de mauvais comptes, en plein texte en fichiers. Une recherche après un
  arrêt ne trouve pas un fichier d'un paquet validé.
- **Atteignable en service** : oui, pour toute base au plein texte en fichiers qui reprend après un arrêt.
- **Touche rag3weaver** : oui, C (`src/catalog.rs`, `src/code_sync.rs`), le lot de recherche qui retient les dérivations d'un premier index jusqu'après les relations.
- **Pour** : recherche (C est son lot).
- **Signalé par** : ED, à la passe du lot 2.7 (ii), `schema-changed-retry`.

## Ce qu'on voit

Le 19 novembre, à luciepc, lib 7c2b506ec. La passe du lot 2.7 (ii) (`passe-ed-27ii`) rougit trois tests de
`e2e_tx_par_paquet_arret`, tous « en fichiers » :

- `en_fichiers_un_arret_entre_les_fichiers_et_la_validation_se_rebatit_aux_bons_comptes` (ligne 831) : « en
  fichiers, la reprise rend les comptes d'une passe sans arrêt, plein texte compris » ;
- `en_fichiers_une_recherche_apres_un_arret_rebatit_avant_de_repondre` (ligne 871) : « f010 est dans un paquet
  validé : au moins un résultat » ;
- `en_fichiers_le_rebati_dit_son_etat_et_survit_a_son_interruption` (ligne 898) : « le lecteur calcule
  « à rebâtir » ».

**Ce n'est pas le lot 2.7 (ii).** Rejoués sur la base de sa branche (`536f8824af`, master du 19 sans le lot,
même lib), les tests « en fichiers » rougissent aussi : 4 échecs sur 6 avec le filtre `en_fichiers`, dont
`en_fichiers_hors_transaction_un_arret_entre_lignes_et_plein_texte_se_detecte` (ligne 954). Journaux à luciepc :
`~/.cache/rag3weaver-build/passe-ed-27ii/` et `passe-ed-27ii-base/`.

**C'était vert hier** : la même suite entière, 18 sur 18, sur la passe du correctif des arêtes
(`passe-ed-first-sync-3`, tête `7e8b93972e`, lib 3d706f709, le 18 novembre vers 16 h).

## Ce qu'il faut pour le fermer

Départager entre le Rust atterri depuis le 18 au soir et la lib 7c2b506ec, en jouant la suite sur
`d9c30ef781` sous 7c2b506ec, puis sur la base du 19 sous 3d706f709. Ensuite, le correctif chez le propriétaire, et
la suite verte.

## La cause (recherche, 19 novembre 2026)

**Le départage** (filtre `en_fichiers`, à luciepc) :
- master 536f8824af sous la lib d'avant 3d706f709 (`RAG3WEAVER_MOTEUR_ANCIEN=1`) : 4 rouges sur 6 ;
- d9c30ef781 sous la lib 7c2b506ec : 6 verts sur 6.

Ce n'est donc pas la lib, mais le Rust atterri entre ces deux commits.

**La bissection** : le parent de C (07b5f782f8) donne 6 verts sur 6, C (e6c0307291) 4 rouges sur 6, sous la même
lib.

**Le mécanisme.** C retient les dérivées d'un premier index (`ScopeContent`, le plein texte du code) jusqu'après
les relations, pour que les outils de référence répondent plus tôt. Avant C, chaque paquet rendait les siennes dans
sa propre transaction. Avec C, un arrêt brutal laisse des paquets validés sans leur contenu dérivé :
- le lecteur seul voit `Never` au lieu de « à rebâtir » ;
- la recherche ne trouve pas f010, qui est pourtant dans un paquet validé ;
- la reprise rend de mauvais comptes, puisque les dérivées d'avant la mort ne sont jamais rendues.

C'est une perte de données après un arrêt brutal.

**Le revert.** Les quatre commits de C sont revertés en un commit neuf (branche `revert-c`, au pousseur), et les
quatre lignes `en_fichiers` sortent de `tools/poste/known_red_e2e.txt`.

**Ce qui reste, donc le ticket reste ouvert** : C reposée avec sa durabilité.
- Une marque « dérivations retenues » dans `_catalog_meta`, posée dans la transaction du premier paquet. Le journal
  d'index ne convient pas : c'est celui de la commande `index` de l'hôte, hors de la base, que `sync_source` n'écrit
  pas et qu'un catalogue qui rouvre ne lit pas.
- Les trois chemins :
  - le lecteur seul voit « à rebâtir » ;
  - le chercheur rend les dérivées dues avant de répondre ;
  - le repreneur rend celles d'avant la mort.
- Les quatre cas rouges d'abord, puis verts, et la mesure de C refaite en release.

## Ce qui le ferme (recherche, 19 novembre 2026)

- **Le revert** de C est sur master (4c0728e703, PR #19) : les quatre cas `en_fichiers` sont redevenus verts, jugés
  pour de vrai depuis que leurs lignes ont quitté `tools/poste/known_red_e2e.txt`.
- **C revient, durable**, sur la branche `c-durable-2` :
  - la marque `derivations_held` dans `_catalog_meta`, posée dans la transaction du premier paquet et effacée quand
    les dérivées retenues sont rendues ;
  - un écrivain qui trouve la marque rend ce qu'une passe morte a retenu (`Catalog::repair_held_derivations`), avant
    toute lecture (`consigne`) et en fin de reprise ;
  - tant que la marque reste, l'état d'une entité dérivée dit « à rebâtir » (`Running`, 0 %).
- **Le témoin** : les quatre cas `en_fichiers`. Ils sont rouges sur C réappliquée telle quelle (f40c251297, comme en
  bissection : C 4/6, son parent 6/6), puis verts avec le correctif.
