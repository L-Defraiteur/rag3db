# Le filtre `_uuid IN` par la clé primaire

Session recherche, 14 novembre 2026. Lot sur master en 9fd946ec3, rang 26 de la pile, posé sur le
crochet d'arbre principal (d1578cd56).

## Ce qui coûtait

`resolve_filter_to_ids` traduisait un filtre `_uuid` en liste par
`MATCH (n:T) WHERE n._uuid IN $liste`. Le moteur balaie alors toute la table et teste chaque ligne
contre la liste : le coût est la table fois la liste. Sur une base à versions, le masque des versions
(`generic_search_nodes.rs`, `FilterCondition::Field { _uuid, List[versions] }`) pose exactement ce
filtre à chaque recherche. ED l'a mesuré : 5 008 ms pour 72 652 versions, et environ 113 ms par la
clé écrite à la main.

## Ce que le lot fait

`resoudre_le_filtre` (src/read_view.rs) extrait la liste d'uuids :
- d'un filtre `_uuid` en liste ou en `In`, seul ;
- ou d'un `Must` dont c'est exactement un terme.

Il la passe en paramètre (`$_cles_du_filtre`), puis le dialecte écrit
`UNWIND $_cles_du_filtre AS _cle MATCH (n:T {_uuid: _cle})`. Le reste du filtre et la clause des
générations ne portent que sur les lignes trouvées. La variante à morceaux (le creux) suit avec
`WITH DISTINCT n._uuid AS _parent`. Le tri et le dédoublonnage d'après la requête couvrent les
doublons de la liste.

Les formes suivantes **gardent le balayage** :
- **l'indirection** (le filtre porte sur l'entité titre, alias `t`) : la clé n'est pas celle de la
  table résolue ;
- **Postgres** : `filter_resolve_offsets_by_keys` rend `None` par défaut et seul le dialecte rag3db
  la dit. `_uuid = ANY($liste)` y passe déjà par l'index de la clé ;
- **deux listes d'uuids dans un même `Must`**, et un uuid sous `Should` ou `MustNot`.

Un ticket moteur demande que le planificateur fasse cette réécriture de lui-même.

## Les preuves

- **Témoin** : `e2e_invariant_des_vecteurs::un_filtre_par_uuid_se_resout_par_la_cle_primaire`, avec
  30 000 uuids, des doublons, une liste vide, un `IN` combiné et un `IN` hors de la clé. Il était
  rouge en 08aa781db : 1,10 s pour la seule résolution. Il est vert en e0aaa2d1f : 0,28 s pour le
  test entier, création des nœuds comprise. Lib 373b5477b, à luciepc.
- **Rejeu court** sous 373b5477b, tout vert :

  | suite | résultat |
  |---|---|
  | e2e_invariant_des_vecteurs | 13/13 |
  | e2e_code | 36/36 |
  | e2e_postgres | 36/36 |
  | check_features | 8/8, avec la preuve |
  | lib | 2 400 |
  | témoins navigateur | 45 |

- **Parité au bit près** : jouée par banc, juge indépendant du lot. Il a comparé e0aaa2d1f à son
  parent 08aa781db, chacun sur une copie neuve de la base migrée (forme à versions), avec le jeu
  commun de 10 requêtes × 3 formes et k = 10. Les 30 cas sont identiques : même `_uuid`, même ordre,
  même score. Les relevés sont à luciepc, dans
  `~/.cache/rag3weaver-build/banc-parite/sorties/filtre-in/`.
- **Temps d'appel** relevés par banc : un appel par requête et par forme, sans chauffe. C'est une
  indication, pas une mesure. Médianes, parent → lot :

  | forme | parent | lot |
  |---|---|---|
  | sans filtre | 623 ms | 624 ms |
  | chemin `src/include/` | 2 272 ms | 1 176 ms |
  | langage python | 1 508 ms | 910 ms |

  Les formes filtrées gagnent environ deux fois ; la forme sans filtre ne bouge pas sur ce jeu.
- **La sonde d'ED** (`tests/sonde_in_par_cle.rs`, l'étape 3 attendue sous 200 ms) : sur une copie de
  la base migrée de 1b, à jouer sous `poste mesure`. Son résultat sera ajouté ici.
