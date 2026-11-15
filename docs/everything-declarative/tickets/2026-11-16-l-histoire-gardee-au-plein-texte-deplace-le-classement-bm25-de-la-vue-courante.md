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

## La cause première : une base rouverte se croyait « tout visible » (ED, 16 novembre, 0 h 15)

`Visibilite::tout_visible` était posé à vrai à toute ouverture d'une base à générations (`catalog.rs`, à la pose
des générations). Seuls l'écrivain, dans son processus, et un témoin le baissaient. Une base qui a de l'histoire,
rouverte par un autre hôte, partait donc **sans masque** pour tout lecteur sans filtre (`read_view.rs`,
`compile_filter_for_vector_in` et `parser_le_filtre`). C'est le cas du juge de banc, qui rouvre la copie du lot après
l'index : sa recherche « sans filtre » cherchait sur les 136 181 morceaux, avec N, les fréquences et la longueur moyenne
de toute l'histoire, et les versions fermées pour candidates.

C'est un défaut de la lecture sale sous versions : un lecteur sans masque sur une base à histoire. Il est corrigé dans
le lot de l'écrivain (`ecrivain-2a-prime-2-pile-3`, 5081d2281) : « tout visible » se lit sur la base à l'ouverture.
Une ligne retirée, d'une autre couche ou posée après le rang lu le rend faux. Le témoin 21
(`code::tests_de_l_ecrivain::une_base_rouverte_sait_qu_elle_a_une_histoire`) est rouge sur 6b3579115 et vert sur
5081d2281.

**Ce qui reste à juger** : banc rejoue ses 30 cas sur la tête corrigée. Si les rangs reviennent, ce ticket ne garde que
la longueur moyenne du champ, et peut-être rien ; lucivy attend ce rejeu avant de dessiner (c7).

## Pas de perte sur search_code : la visibilité descend en pré-filtre (ED, 16 novembre, 0 h 50)

Une sonde dans `BM25SearchNode` (branche `sonde-masque-lexical`, hors push) le montre. Sur une base à générations,
la visibilité arrive au nœud **en condition de filtre explicite** : `Must[Scope._couche = backend, _depuis <= rang,
_jusqu_a nul ou > rang, _uuid non nul]`. `allowed_ids_in` la résout, et lucivy reçoit l'ensemble avant le top-k. La
perte supposée, où un filtre après coup ôterait sans promouvoir, n'a donc pas lieu sur ce chemin.

Le témoin
`backend::tests::une_recherche_lexicale_sans_filtre_ne_perd_pas_la_ligne_visible` le garde : trois versions fermées de
`depart` mieux classées, k = 1, et la ligne ouverte rendue.

Le correctif de « tout visible » (cause première ci-dessus) reste juste pour les lecteurs sans filtre qui ne passent
pas par ce chemin. Mais le classement de `search_code` avait déjà son masque. **L'écart des 23 cas vient donc de la
longueur moyenne du champ**, comptée sur le corpus : lucivy mesure 31,6 M tokens avec l'histoire contre 9,4 M visibles.
Sa moyenne sur le corpus donne 11 classements différents sur 15, et sa moyenne sur les admis 4 sur 15, à un écart de
score d'au plus 2,6 %.

## Ce que le code fait déjà, et ce qui reste

- **Le masque est un pré-filtre, pas un filtre après coup.** `search_filtered` de lucivy 4.3.7 prend les k premiers
  parmi les documents admis, et compte N et les fréquences de document dans la vue. Le témoin en est
  `fts_handle::tests::une_recherche_filtree_classe_comme_sa_vue`. Une vue ne perd donc pas un résultat au profit d'un
  morceau d'une autre génération, **si le masque lui est passé**.
- **Mais la longueur moyenne du champ reste celle du corpus** : le même témoin le dit (« la longueur moyenne, gardée
  exprès sur le corpus, n'entre pas en jeu »). Avec l'histoire, le corpus compte les morceaux des versions fermées, et la
  normalisation de longueur de BM25 change pour tous. C'est la cause la plus probable des rangs déplacés, à prouver.
- **Le masque sans filtre** n'est imposé que si une ligne de la table n'est pas visible (`parser_le_filtre`,
  `Visibilite::tout_visible`). Ce drapeau était vrai à toute ouverture : c'est la cause première, ci-dessus.
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
