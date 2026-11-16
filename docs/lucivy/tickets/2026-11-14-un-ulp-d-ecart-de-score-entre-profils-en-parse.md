# Un ulp d'écart de score entre profils en `parse`

- **État** : ouvert (14 novembre 2026).
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

## Reprise du 16 novembre après audit des diffs

Porteur lucivy. Branche neuve poussée `cx-lucivy-parse-sum-order`, tête
`1b9f47d4f87dddbe76cdf5535a95de6cdff11bdb`, identique au WIP de passation ;
worktree portable `/home/lucied/git_workspaces/cx-lucivy-parse-sum-order`.
Réservés : score_combiner.rs, boolean_query/block_wand.rs et le témoin
`test_parse_scores_identical_across_layouts.rs`. Pas de nouveau correctif.

Preuves anciennes relues : témoin rouge1échec puis vert1/0/0, lib1480/0/24ignorés,
clippyrc0, core interrompu faute de corpus. Aucune nouvelle suite exécutée.
Le noyau7.2 attendu (recette épinglée8d3ae59288f1) et les liens lucivy-cmp sont
absents aux deux postes, non retrouvés dans les emplacements documentés.
À luciepc existent `~/.cache/lucivy-corpora/linux-linux-2.6.0.tar.gz` (42Mo,
sourcekernel.org documentée dans playground/corpora.json) et le clone rag3db
`~/.cache/lucivy-tests/rag3db-clone`. Ce ne sont pas des preuves de présence
ni d’équivalence du corpus7.2. Ni extraction, substitution de corpus, symlink,
ni téléchargement effectué. ABSENT_OK ne sera pas utilisé pour peindre vert.

Reste : faire admettre une provenance de corpus et une recette exacte par
coordination, puis compléter core --no-fail-fast et les preuves manquantes.
Pendant fenêtre G/Arbre : aucun lourd. Entrée4 explorée indépendamment.
Voir `docs/lucivy/16-11-2026/03-reprise-ulp-et-corpus.md` pour la recette.
