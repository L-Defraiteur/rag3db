# Cas de campagne : un ulp d'écart de score en `parse`, d'une disposition de segments à l'autre

Session lucivy, 19 novembre 2026, pour la campagne des correctifs passés (chantier de Lucie). C'est un cas court
de justesse numérique, en Rust, dans le moteur de lucivy. Il se joue sans la lib rag3db, en quelques minutes de
`cargo test`.

## Le point de départ

- **Dépôt** : lucivy (`github.com/L-Defraiteur/lucivy`), pas rag3db.
- **Commit parent** : `e3c9753f7fe5` (16 novembre 2026). Le correctif n'y est pas. **Le témoin n'y est pas non plus** :
  il est entré dans le même commit que le correctif. Il faut le copier depuis main dans l'arbre d'Algareth, tel
  quel, avant de lui donner le cas.
- **Arbre** : le workspace lucivy entier (le moteur `ld-lucivy` à la racine, `lucivy_core` pour le témoin).

## Le signalement, tel qu'il était

C'est le ticket du 14 novembre (`docs/lucivy/tickets/2026-11-14-un-ulp-d-ecart-de-score-entre-profils-en-parse.md`,
dans rag3db). Il est repris sans sa partie « Ce qu'on suppose », qui nommait la cause et le correctif. Sa partie
« Comment le rejouer » est remplacée par le témoin.

> Une même requête `parse` sur les mêmes 89 894 documents, réindexés dans trois profils (positions, sans positions,
> dictionnaire par segment), rend les mêmes documents. Mais pour 2 à 4 documents du top 20, le score diffère d'un
> ulp : au plus 1,9e-6 sur un score d'environ 27, soit 7e-8 en relatif. Les requêtes `contains` n'ont aucun écart.
>
> La doc de `shared_dictionary` (`lucivy_core/src/query.rs`) dit pourtant « counts, spans and scores are identical
> either way », et celle de `positions` dit « same answers ». Le juge de banc de rag3db exige la parité au bit près.

## Le témoin

- **Filtre** : `parse_scores_are_identical_across_segment_layouts`, dans
  `lucivy_core/tests/test_parse_scores_identical_across_layouts.rs` : le même corpus, commité en une fois ou en
  plusieurs, donc des segments disposés autrement. Le fichier est identique sur main (vérifié le 19 nov. sur 792f514).
- **Commande** : `cargo test -p lucivy-core --test test_parse_scores_identical_across_layouts`.
- **Rouge au parent** (et sur main avant le correctif, a1c0dfd, le 16 novembre) : la dernière assertion échoue. 4 scores
  sur 41 diffèrent d'un ulp, par exemple 0,1596082 contre 0,15960819. Tous les documents sont trouvés des deux côtés.
- **Suite** : `cargo test` du workspace lucivy, celle de sa CI. Un correctif qui change la somme peut déplacer d'un ulp
  un score qu'un test d'avant compare exactement : la suite entière le dit.

## Les indices présents dans l'arbre

- `src/query/score_combiner.rs` : les combinateurs de scores d'un OU.
- `src/query/union/buffered_union.rs` : l'union des clauses, qui retire une clause épuisée.
- `src/query/boolean_query/block_wand.rs` : le chemin top-k du OU, qui somme aussi.

## Notre correctif (caché à Algareth, pour le juge)

`3933431475aa`, « fix(ld-lucivy): les scores d'un OU se somment en f64, arrondis une fois » :
`src/query/score_combiner.rs` +13 −6, `src/query/boolean_query/block_wand.rs` +4 −3 (et le témoin, +81). L'union
retire une clause épuisée par `swap_remove`, et l'ordre de la somme en f32 suivait la disposition des segments.
