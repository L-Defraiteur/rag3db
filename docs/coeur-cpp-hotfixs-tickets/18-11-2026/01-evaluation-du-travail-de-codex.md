# Évaluation du travail de Codex dans le périmètre hotfixs (18 novembre 2026)

Codex a tenu le rôle hotfixs du 16 novembre (11 h) au 17 (21 h 08). Il est reparti de
[la passation du 16](../16-11-2026/02-passation-pour-codex.md) et a laissé
`docs/coeur-cpp-hotfixs-tickets/17-11-2026/99-passation-pour-claude.md` sur la branche
`cx-hotfixs-rows-outside-graph-red-pilot-v2`. Cette page juge ce qu'il a produit, sur pièces, avant toute reprise.

**Ce qui a été vérifié** :
- `git log 927941a67a..origin/master` : 76 commits, tous `luciedefraiteur@gmail.com`, aucune attribution. Une seule
  ligne cite Codex : le corps d'un commit d'embarquement (« passation à Codex »), qui n'est pas une attribution.
- `git diff --name-only 927941a67a origin/master -- src test extension/vector tools/shell` est vide : aucun C++ du
  moteur n'a changé sur master pendant l'arrêt.
- `check_tickets.py` est VERT sur master (656 en-têtes), sur `cx-hotfixs-add-column-proof` et sur
  `cx-hotfixs-rows-outside-graph-red-pilot-v2`.
- Les branches, leurs bases et leurs diffs, les pages de preuve, et le témoin rouge de `rows_outside_graph`, tel que
  Codex l'a consigné.

## 1. Le tableau

### Les commits de master qui touchent mon périmètre élargi

Ce sont quatre commits de l'outillage du push et de la passe (`tools/poste/`). Ils appartiennent au rôle de G, et
sont jugés ici seulement pour nos règles.

| Commit | Ce qu'il fait | Verdict |
| --- | --- | --- |
| e271ee084 puis 68590270e | Témoin, puis correctif : `passe_epinglee.sh` épingle la lib réellement posée aux deux postes après la bascule (fin des passes sur 373b5477b) | **utile, à garder**. Le rouge précède le correctif, et la lib épinglée est la bonne (59ac9c86bc, celle des hôtes). |
| ce6aa9ab6 puis b74b85257 | Témoin, puis correctif : la garde juge tous les en-têtes du range-diff d'une pile, ou refuse | **utile, à garder**. Le rouge précède le correctif. Le jugement sur le fond revient à G. |

### Les branches `cx-hotfixs-*`

| Branche | Contenu | Verdict |
| --- | --- | --- |
| `cx-hotfixs-add-column-proof` (c92d557bd, 1 commit, docs) | Ferme mon entrée 1 par un `git mv` vers `docs/tickets/closed/`. L'État cite le correctif du cœur (101ff50a6), et le témoin du cœur sur master, `AlterWaitsForWriters.AnAddWaitsForTheOpenWriter` (`test/transaction/alter_waits_for_writers_test.cpp:80`), rouge avant le verrou de schéma (l'ADD finissait à 1,1 ms, avant le COMMIT à 405 ms). Il annote aussi ma page du 16. | **utile, à garder**. La fermeture est juste, et elle s'appuie sur un témoin existant au lieu d'en écrire un second. Elle n'est pas sur master : à faire atterrir en docs seules. |
| `cx-hotfixs-rows-outside-graph-red-pilot-v2` (e4ea441ac, puis 2d1c8f93e, 23 commits) : **le témoin** `test/transaction/vector_index_diagnostics_test.cpp` (quatre cas) et la correction de fixture 71fbe786f | `ACompleteIndexReportsAKnownZero` est rouge à luciepc sur le BUILD_ID 59ac9c86bc, la lib courante. Ses prémisses (extension chargée, table, index READY) passent, puis il tombe sur `Binder exception: Variable rows_outside_graph is not in scope`. Les trois autres cas sont compilés, mais non joués. | **utile, à garder**. C'est le bon premier rouge, même s'il ne prouve encore que l'absence de la colonne. Les trois cas non joués restent rouges au sens de nos règles, tant qu'ils ne sont pas joués. |
| même branche : **les constats** (`16-11-2026/10-…` et `11-rows-membership-storage-findings.md`) | Le moteur ne garde pas, pour un index vectoriel, l'ensemble des lignes qui sont dans le graphe : `numCheckpointedNodes` est un repère, pas un ensemble. Sans cet ensemble, un compte exact « lignes visibles hors du graphe à l'instantané de l'appelant » n'est pas lisible. | **utile, à garder**. C'est un vrai constat, qui corrige la forme que proposait mon ticket (« au-delà de `numCheckpointedNodes`, plus ce que le dernier bâti n'a pas pris »). |
| même branche : **la conception** `17-11-2026/01-…`, `02-…`, `03-membership-relational-contract.md` | Une table de relations interne de « membership », à côté des deux graphes ANN, avec ses invariants. Elle suppose une rétention sous les vieilles transactions, un changement de format, une montée de version du journal, et l'idempotence de tous les mutateurs. Rien n'est codé. | **utile mais à requalifier**. C'est une conception de format que Lucie n'a pas décidée : elle va dans « ce qui attend Lucie », comme page, et non dans du code. La passation de Codex le dit elle-même (« aucun GO code »). |
| même branche : **le pilote** (`16-11-2026/rows-red/*.py`, `launch.sh`, manifestes, coupons « GO ») et les paquets de preuve | Un outillage ad hoc de lancement, de reçus et de hachage, pour jouer un seul test. 3 300 lignes ajoutées au total sur la branche. | **inutile pour la suite**. Il double `poste` et `passe_epinglee.sh`, et la procédure ne le connaît pas. Les preuves restent en archive sur la branche. Ne pas le faire atterrir, ni le réutiliser. |
| `cx-hotfixs-rows-outside-graph`, `-df2`, `-fixture-fix`, `-red-pilot` | Les étapes intermédiaires du même chantier, toutes contenues dans `-red-pilot-v2`. | **inutile, à laisser** : remplacées. Aucun retrait à faire, ce sont des branches. |
| `cx-hotfixs-temoin-quarantaine` (e289163bf) et `cx-hotfixs-recette-quarantaine` (32fc739a8) : 12 novembre, une passation Codex plus ancienne | Un diagnostic de mon entrée 5 (`test/transaction/quarantine_released_diagnostic.cpp`). Après l'annulation partielle de TA, BEGIN et CHECKPOINT publics sont bien refusés, mais le COMMIT de TB, déjà ouverte, **atteint la fabrique interne du point de reprise**. | **utile mais à requalifier**. Le constat confirme l'arc que décrit mon ticket. Mais il a été joué sur une source moteur privée (0511dece7), avec une sentinelle, sans point de reprise réel. À rejouer sur master sous forme de témoin ordinaire, puis à corriger (la garde à l'entrée de `commit`). |
| `cx-hotfixs-fts-global-ro` (7b9d9a8ec, 71 commits) | Du Rust du plein texte de rag3weaver (`catalog.rs`, `code_sync.rs`, `fts_existing.rs`…) et des revues pour la session recherche. | **utile mais à requalifier**, hors de mon périmètre malgré son nom. C'est à la session recherche de l'évaluer et de le reprendre sous son nom. Je n'en fais rien atterrir. |
| `cx-hotfixs-passation-week-end-20261113` | La passation du week-end du 13 novembre. | **inutile, à laisser** : remplacée par les pages du 16 et du 17. |

## 2. Le verdict d'ensemble

1. **Rien de nuisible.** Aucun C++ du moteur n'a changé sur master, l'identité et l'absence d'attribution sont tenues,
   les tickets ont bougé proprement, et rien n'a été poussé de force.
2. **Peu de livré.** Une fermeture de ticket juste (l'ADD COLUMN), un bon premier rouge, et un constat qui corrige mon
   propre ticket. Aucun correctif.
3. **Un écart de cadre.** L'entrée 4 (une lecture de 4 à 6 h) est devenue une conception de format de stockage. Elle
   n'a pas été soumise à Lucie comme une décision.
4. **Un coût de cérémonie disproportionné.** Coupons, pilotes, manifestes et hachages sur deux postes, pour jouer un
   seul test rouge. Notre procédure (`poste lourd`, `passe_epinglee.sh`) suffisait.
5. **Les douze autres entrées** de ma liste sont restées intactes : la liste reprend où elle était, entrée 1 mise à
   part.

## 3. Ce que ça change à ma page du 16

- **Entrée 1 (l'ADD COLUMN)** : faite par Codex. À faire atterrir depuis `cx-hotfixs-add-column-proof`, en docs seules.
- **Entrée 4 (`rows_outside_graph`)** : réécrite. Le compte exact demande un ensemble de membership que le moteur n'a
  pas. Deux voies, à soumettre à Lucie :
  - **(a) une borne supérieure**, sans changement de format : les lignes au-delà de `numCheckpointedNodes`, plus un
    compteur en mémoire de ce que le dernier bâti a sauté. Elle est nommée comme telle (`rows_outside_graph_at_most`),
    et suffit pour le seuil de reconstruction du produit (N* ≈ 13 400) ;
  - **(b) le compte exact**, par la conception de Codex, un changement de format.

  Le témoin de Codex se garde dans les deux cas : sa colonne, son nom et sa valeur attendue s'ajusteront à la voie
  choisie.
- **Entrée 5 (la quarantaine)** : un pas en avance. Il reste à écrire le témoin ordinaire sur master, à partir du
  diagnostic de Codex.
- **« Ce qui attend Lucie »** reçoit une ligne : le choix (a) ou (b) pour `rows_outside_graph`.

## 4. La lisibilité de sa passation

- **La forme** : une prose très dense, des nombres collés aux mots (« ONE_ONE/FWD impose AU PLUS1 », « rehash2postes0écart »),
  du jargon de pilote (« coupons », « END », « GO ») qui n'existe pas dans nos procédures. Elle est difficile à lire
  sans l'avoir écrite.
- **Le fond** : elle est honnête et complète. Elle dit ce qui n'est pas fait (« AUCUN correctif moteur »), ce qui n'est
  pas prouvé, et ce qu'il ne faut pas relancer. Elle donne les SHA, les chemins des preuves sur les deux postes, et
  l'inventaire des branches, qui a servi à cette évaluation.
- **À reprendre** : la conclusion de la page tient en trois lignes (un rouge juste, un constat, une conception à
  décider). Elle méritait d'être en tête.
