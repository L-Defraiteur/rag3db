# La recherche à versions masque toutes les occurrences quand rien ne filtre

- **État** : fermé (16 novembre 2026) : corrigé sur master par 00e28eee67 (le court-circuit de `occurrence_mask_options` quand aucune orpheline) puis 7a2765baf1 (le masque en prédicat, sans liste `IN`) ; témoins `tests/e2e_masque_sans_filtre.rs` ; mesure 406 s → 12 s sur 24 requêtes, réponses identiques 24/24 (`docs/orchestration/14-11-2026/01-rapport-de-session-c7.md`).
- **Famille** : produit
- **Gravité** : la recherche est 10 à 25 fois plus lente sur toute base à versions (neuve depuis
  e62151904, ou migrée par 1b) ; elle bloque la bascule des bases de Lucie.
- **Atteignable en service** : oui, toute recherche sans filtre sur une base à versions.
- **Touche rag3weaver** : oui (`src/dataflow/generic_search_nodes.rs` : `occurrence_mask_options`,
  `occurrence_roots_for` ; `src/search.rs` : `project_occurrence_results`).
- **Pour** : arbre principal (c'est le lot 1a).
- **Signalé par** : arbre principal (le jeu fixe de requêtes de 1b sur la copie de `code-rag3db-depot`)
  et everything declarative (la mesure étage par étage), 14 novembre 2026.

## Ce que c'est

Sans filtre, `occurrence_roots_for` admet **toutes** les occurrences de la cellule (`select_all`,
79 662 sur la copie). `occurrence_mask_options` lit leurs versions par la relation canonique, puis
passe `_uuid IN [72 652 versions]` à chaque signal. Le lexical et le dense refont le masque chacun.

Mesure d'ED, étage par étage : search_code passe de 965 ms à 7 032 ms. Le masque
(`derived_references`) coûte 902 ms ; la conversion du `IN` en offsets par `resolve_filter_to_ids`
coûte **5 001 ms**. Par le produit (le jeu fixe de requêtes), c'est 7,5 s par requête en bm25 et 25 s
en bm25+vector, contre environ 1 s sur la copie d'avant.

## Le correctif (décision de c7)

Sans filtre utilisateur et sans vue, toutes les versions qui ont au moins une occurrence sont
admises : on ne masque rien, et seule reste la remontée version → occurrences des résultats retenus.
**Seulement si la base n'a aucune version orpheline** (`NOT EXISTS` sur la relation canonique,
compté une fois par appel). Sans masque, une orpheline entrerait dans le top-k, puis la projection la
retirerait : le rappel baisserait en silence. S'il y a des orphelines, le masque d'aujourd'hui reste.

## Dépendance

**Aujourd'hui, une base vivante fait des orphelines** (ED, 14 novembre). Sous 2a′-1, une édition ou une
suppression de fichier passe encore par l'effacement (`apply_snapshot_finish`, `delete_jusqu_a`), et
les versions à identité de contenu sont exclues de l'effacement. Le court-circuit ne vaut donc que pour
une base sans orpheline : fraîchement migrée, ou neuve et non éditée. **C'est 2a′-2 qui le ferme**
(l'ancre immuable : retirer, c'est poser `_jusqu_a`, et la ligne garde son arête). Le test du
court-circuit devient alors « aucune version sans occurrence **visible** au rang lu », et la
projection ne garde que les occurrences visibles. C'est le point de composition avec 2a′-2 ; ED
l'inscrit dans sa page.

## Les témoins

`tests/e2e_masque_sans_filtre.rs` :
- `une_recherche_sans_filtre_ne_lit_pas_toutes_les_occurrences` : 200 fonctions, k = 3. Aucune requête
  de la recherche ne lit la moitié des occurrences, en plein texte comme en dense. Rouge : 200 lignes
  lues pour 200 occurrences.
- `une_version_orpheline_ne_prend_pas_la_place_d_une_occurrence` : un scope édité laisse sa version
  orpheline, et la recherche rend quand même k occurrences. C'est un garde-fou, vert avant et après.

## Ce qui le fermerait

Les deux témoins verts, puis le jeu fixe des 24 requêtes sur la copie migrée revenu vers 1 s par
requête.
