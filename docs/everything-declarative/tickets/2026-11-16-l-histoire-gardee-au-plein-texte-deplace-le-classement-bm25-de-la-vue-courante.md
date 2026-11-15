# L'histoire gardée au plein texte déplace le classement BM25 de la vue courante

- **État** : ouvert (16 novembre 2026)
- **Famille** : produit (générations et recherche lexicale)
- **Gravité** : défaut de classement. Sur une base qui garde son histoire, une recherche lexicale au rang courant ne classe
  pas comme un index neuf du même arbre. La bascule des bases de Lucie attend ce ticket ou une mesure sans perte (c7)
- **Atteignable en service** : oui, dès qu'une base à générations a une version fermée. Les bases d'aujourd'hui n'en ont pas
- **Touche rag3weaver** : oui (`src/fts_handle.rs`, `search_hits_with_text` ; `src/read_view.rs`, `parser_le_filtre` ;
  `src/search.rs`, `Admission`) et **lucivy** (`search_filtered`, les statistiques de la vue)
- **Pour** : lucivy (les statistiques au rang visible), avec ED (le masque passé par rag3weaver)
- **Signalé par** : banc, le juge de l'écrivain 794b14232 contre un index neuf (16 novembre 2026, 0 h) ; décision de c7 (0 h 2x)

## Ce que c'est

Banc a comparé l'écrivain 794b14232 à un index neuf du parent f10336d3f, sur la même source (1de1f86d88). Le lot a
repassé P0-3 vers cette source.

- **Le graphe est égal**, à une arête près : un scope inchangé garde `MENTIONS gzip` là où l'index neuf pose
  `USES_LIBRARY gzip`. Miroir 0/0, doublons 0.
- **Le classement ne l'est pas** : 23 cas sur 30 sont rouges au régime `reponses`. Ce sont de vrais rangs, pas des ex
  aequo. « PortDef » sans filtre : PortDef passe du rang 4 au rang 3. « json » sans filtre : basic_json entre au rang 8
  et json_of en sort.
- **Le plein texte du lot porte l'histoire** : 136 181 `ScopeContent_CHUNKED_FROM`, contre 114 854 à l'index neuf.
  Le dessin le prévoyait : « le plein texte indexe `ScopeContent`, partagé : il reste commun, et sa visibilité vient du
  masque » (page 03 de 2a′, du 14 novembre). La page 05 passe ce masque à lucivy en `allowed_ids`.

## Ce que le code fait déjà, et ce qui reste

- **Le masque est un pré-filtre, pas un filtre après coup.** `search_filtered` de lucivy 4.3.7 prend les k premiers
  parmi les documents admis, et compte N et les fréquences de document dans la vue. Le témoin en est
  `fts_handle::tests::une_recherche_filtree_classe_comme_sa_vue`. Une vue ne perd donc pas un résultat au profit d'un
  morceau d'une autre génération, **si le masque lui est passé**.
- **Mais la longueur moyenne du champ reste celle du corpus** : le même témoin le dit (« la longueur moyenne, gardée
  exprès sur le corpus, n'entre pas en jeu »). Avec l'histoire, le corpus compte les morceaux des versions fermées, et la
  normalisation de longueur de BM25 change pour tous. C'est la cause la plus probable des rangs déplacés, à prouver.
- **Le masque sans filtre** n'est imposé que si une ligne de la table n'est pas visible (`parser_le_filtre`,
  `Visibilite::tout_visible`). Il reste à vérifier sur la copie du lot, rouverte par un hôte neuf, que ce drapeau est
  bien faux : sinon la recherche partirait sans masque.
- **La remontée version → occurrences** (`Admission`) n'ôte après coup que si un filtre, une cellule ou des versions
  orphelines restreignent. Il reste à vérifier qu'une version dont toutes les occurrences sont fermées est ôtée **par
  le masque d'avant**, et pas par cette remontée. Sinon elle prendrait une place dans les k premiers puis tomberait :
  ce serait une perte, et non un rang.

## Décision (c7, 16 novembre 2026)

- **(c)** : des statistiques BM25 au rang visible, dans lucivy, y compris la longueur moyenne. lucivy donne l'estimation
  et le dessin.
- **Pas (b)**, qui retirerait du plein texte les versions fermées : une vue épinglée à un rang ancien est un worktree en
  retard sur master, et elle perdrait sa recherche lexicale.
- L'écrivain atterrit sans attendre : les bases d'aujourd'hui n'ont pas d'histoire.
- La bascule des bases de Lucie attend (c), ou bien une mesure de banc sans perte sur les 30 cas (P1 et rappel). Dans ce
  second cas, elle part en (a) le temps que (c) arrive.

## La preuve à faire (banc)

Sur la copie du lot, jamais une base de service : effacer du plein texte les morceaux des versions fermées, puis rejouer
les 30 cas. Si PortDef et json reviennent à leurs rangs, la cause est l'histoire au corpus. Pour séparer la longueur
moyenne des fréquences, journaliser pour un cas la longueur moyenne vue par la recherche filtrée, au lot et à l'index neuf.

## Comment le fermer

Par (c) posé dans lucivy et repris par rag3weaver, avec un témoin rouge d'abord : deux index, l'un avec des morceaux
fermés, l'autre sans, une recherche filtrée par la vue, et le même classement au bit près. Et les 30 cas de banc verts
au régime `reponses` contre l'index neuf.
