# Le bm25 seul de la migration reste à prouver égal sous 4.3.9

- **État** : ouvert (16 novembre 2026), à prouver. Ce n'est pas un défaut constaté sous 4.3.9 : il manque la preuve
  que la 4.3.9 corrige ce que le banc a mesuré sous 4.3.8.
- **Gravité** : moyenne. Si l'égalité ne vient pas, une base qui garde son histoire (les générations d'ED) ne classe
  pas comme un index neuf des mêmes rangées visibles, et la recherche de code sur une base migrée diffère.
- **Atteignable en service** : oui, sur toute base dont le plein texte garde des versions fermées.
- **Touche rag3weaver** : oui, par le pointeur lucivy de `extension/rag3weaver/Cargo.toml`.
- **Famille** : produit
- **Pour** : lucivy (la mesure est au banc)

## Ce qui se passe

Le banc a comparé A, une base migrée qui garde l'histoire (113 570 `ScopeContent_Chunk`), à B′, un index neuf des
mêmes rangées visibles (96 884). Sous lucivy 4.3.8, le **bm25 seul** donne un rappel à 10 de 270/279, avec
14 classements égaux sur 30 (`docs/banc/16-11-2026/01-la-parite-de-la-migration.md`, § 3). La requête et l'arbre
sont les mêmes.

La cause pour lucivy : sous un `allowed_ids`, la longueur moyenne d'un champ venait du corpus entier, histoire
comprise. La 4.3.9 la lit sur les documents admis (`docs/lucivy/16-11-2026/01-bm25-au-rang-visible.md`, témoin
`lucivy_core/tests/test_filtered_scores_ignore_hidden_lengths.rs`). Mêmes rangs exigés, scores à 5e-3 relatif près.

## Ce qui ferme ce ticket

Le banc rejoue le bm25 seul sur les mêmes copies A et B′, une fois le pointeur 4.3.9 sur master de rag3db. Il
rejoue ensuite tous les signaux.

- **Égalité de classement sur les 30 cas** : ce ticket se ferme, avec le résumé du banc en preuve.
- **Sinon, c'est à lucivy de départager** avant toute autre hypothèse :
  1. Prendre une copie reflink de la table des morceaux dans chacun des deux `.fts` (le dossier `Lucivy_<entité>` qui porte `ScopeContent_Chunk` ; le banc dit lequel), et le fichier des
     `_node_id` que A passe en `allowed_ids` (l'ensemble visible lu par l'hôte, demandé à ED ou au banc).
  2. Lancer
     `LUCIVY_CMP_A=<copie B′> LUCIVY_CMP_B=<copie A> LUCIVY_CMP_QUERIES=<requêtes du banc en QueryConfig> LUCIVY_CMP_B_IDS=<ids visibles> cargo test --release -p lucivy-core --test test_compare_two_indexes compare_two_indexes -- --ignored --nocapture`.
     Le test compare les contenus, N, les tokens par champ, puis le top 10 par empreinte du contenu.
  3. Si les contenus, N et les tokens sont égaux mais que le top 10 diffère, le défaut est dans lucivy : un témoin
     rouge réduit à ce cas dans `test_filtered_scores_ignore_hidden_lengths.rs`, puis le correctif.
  4. Si les contenus diffèrent, le défaut n'est pas dans lucivy : écrire un ticket pour ED (l'écrivain) ou pour
     recherche (le découpage), avec ce relevé.
