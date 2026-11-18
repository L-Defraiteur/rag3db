# Un ulp d'écart de score entre profils en `parse`

- **État** : corrigé (18 novembre 2026), dans lucivy main 3933431 (PR #43), à publier avec la 4.3.10 (au mot de Lucie).
  La cause : l'union retirait une clause épuisée par `swap_remove`, ce qui changeait l'ordre des clauses restantes ;
  la somme f32 d'un document suivait donc la disposition des segments. Le correctif somme en f64 et arrondit une
  fois (`SumCombiner`, `DisjunctionMaxCombiner`, `block_wand`). Témoin `test_parse_scores_identical_across_layouts` :
  rouge sur main (4 scores sur 41 à un ulp), vert ensuite. `-p lucivy-core` vert sur le corpus rag3db et linux v7.2.
- **Gravité** : faible. Mêmes documents, et un écart d'un ulp de f32 sur le score. Mais la doc de lucivy promet
  des scores identiques, et le juge de banc de rag3db exige la parité au bit près.
- **Atteignable en service** : seulement si un index change de profil (`positions`, `shared_dictionary`), ce que
  rag3weaver ne fait pas aujourd'hui.
- **Touche rag3weaver** : non (lucivy seule).
- **Famille** : produit
- **Pour** : lucivy

## Ce qui se passe

Une même requête `parse` sur les mêmes 89 894 documents, réindexés dans trois profils (positions, sans positions,
dictionnaire par segment), rend les mêmes documents. Mais pour 2 à 4 documents du top 20, le score diffère d'un ulp :
au plus 1,9e-6 sur un score d'environ 27, soit 7e-8 en relatif. Mesuré à luciepc le 14 novembre 2026, deux fois, sur
« shared dictionary » et « open fts handle reason » en `parse`. Les requêtes `contains` n'ont aucun écart.

La doc de `shared_dictionary` (`lucivy_core/src/query.rs`) dit pourtant « counts, spans and scores are identical
either way », et celle de `positions` dit « same answers ».

## Ce qu'on suppose (à vérifier)

`parse` sur une valeur simple devient un OU de `contains` par mot et par champ. Les scores de ses clauses
s'additionnent en f32, et l'ordre de la somme change si les clauses ne répondent pas dans le même ordre selon la
disposition des segments ou des dictionnaires. La vérification consiste à sommer en f64, ou dans un ordre fixé par
clause, puis à rejouer le banc.

## Comment le rejouer

`lucivy_core/tests/test_light_profile.rs`, branche lucivy `mesure-profil-leger` (f89e6df), avec un fichier de
requêtes réduit aux `parse` : la ligne `SCORES DIFFER … largest gap` donne l'écart.
