# Le pas 2a′ : stocker les générations sans rien faire fuir

14 novembre 2026, everything declarative ; à relire par arbre principal **avant tout code**, puis
c7. Elle suit le plan de 2b-B (`01-plan-du-chargeur-durable-des-couches-2b-b.md`, § 7 bis, § 7 ter et
§ 8). Le code a été lu sur origin/master c0ebec3a6, relativement à `extension/rag3weaver/` ; rien n'a
été compilé ni joué. Le relevé des lecteurs a été fait par un agent, en lecture seule, et ses points
clés ont été vérifiés.

## 1. La question qui se pose avant le code : l'ancre a-t-elle des générations ?

- **Aujourd'hui, l'ancre est vivante.**
  - Une ligne `Scope` a pour uuid `hashsafe_uuid("Scope", [clé])` (`catalog.rs:2252-2273`,
    `uuid.rs:35`).
  - Une réindexation la **réécrit** sur place.
  - Le snapshot d'un fichier efface ce qui n'est plus marqué, au grain `{source, file_path}`
    (`code_sync::grain_du_fichier`, l. 52 ; `plan_snapshot_finish`).
- **Une couche épingle la génération de sa parente** (page 30, § 2 ; décision 4 : « le rebase
  suit »).
  - Si la parente est l'ancre et que l'ancre réécrit ses lignes, l'état de l'ancre au commit de base
    de la couche est **perdu** dès que l'ancre avance.
  - La vue de la couche montrerait alors, sous la couche, l'ancre d'aujourd'hui, et non celle de sa
    base. C'est exactement le mélange que le contrat refuse.
- **Donc l'ancre doit avoir des générations elle aussi.** C'est ce qu'arbre principal a écrit
  (« les lignes de l'ancre reçoivent une génération »). Il reste à savoir comment, sans migrer les
  uuid de toutes les bases ni ralentir chaque lecture.

## 2. La proposition : des lignes immuables, une visibilité par deux colonnes

- **Une ligne `Scope` est immuable une fois publiée.**
  - Un scope inchangé garde sa ligne.
  - Un scope changé ne se réécrit pas : sa génération neuve écrit une **ligne neuve**, et la ligne
    d'avant est **retirée** de la génération neuve, sans être effacée.
  - L'effacement physique n'appartient qu'à la rétention (M5), sous son bail.
- **Deux colonnes sur `Scope`** : `_depuis` (le rang, dans sa chaîne, de la génération qui a posé la
  ligne) et `_jusqu_a` (le rang de la génération qui l'a retirée ; vide tant qu'elle est vivante).
  Plus `_couche`, la chaîne à laquelle la ligne appartient : l'ancre, ou une couche.
- **Une ligne est visible dans la génération de rang n de sa chaîne** si `_depuis ≤ n` et
  (`_jusqu_a` vide ou `_jusqu_a > n`). C'est un prédicat de deux colonnes, sans jointure ni ensemble
  d'identifiants à porter dans chaque requête : le filtre le moins cher possible pour tous les
  lecteurs sans vue.
- **L'uuid d'une ligne** est `hashsafe_uuid("Scope", [clé, couche, _depuis])`.
  - Les lignes **d'avant** ce lot gardent leur uuid (clé seule). Elles appartiennent à la génération
    0 de l'ancre (`_depuis` = 0), posée par l'estampille du format.
  - **Aucune base existante ne voit ses uuid changer** : ni migration d'uuid, ni arête à
    réécrire. Seules les lignes écrites après portent leur génération.
  - Deux générations d'un même scope ne se percutent donc plus sur `_uuid`. C'est la condition
    d'arbre principal, tenue sans toucher l'existant.
- **L'entité `Generation`** (décision 1) porte :
  - l'identité de la page 30 : la chaîne, le commit, la base, l'empreinte de la recette ;
  - son rang dans sa chaîne et sa parente épinglée (la chaîne et le rang de la parente) ;
  - ses tombstones de fichier.
  Les occurrences ne sont pas énumérées une à une : elles se lisent par `_depuis` et `_jusqu_a`.
  C'est l'ensemble d'identifiants de la décision, en compréhension.
- **Une couche** est une chaîne à elle. Sa vue à son rang g est :
  - ses lignes visibles à g ;
  - plus les lignes de la parente visibles au rang épinglé ;
  - moins ce que la couche retire (ses tombstones de scope par clé, et de fichier).

  C'est ce que compose le résolveur 2a, et c'est le pas 2b′. 2a′ ne fait que l'ancre et le stockage.
- **Sans vue, la lecture d'aujourd'hui devient** « l'ancre, à son rang courant » : `_couche` =
  ancre, et le prédicat au rang courant. Les lignes d'une couche n'y apparaissent jamais (le témoin 0).

## 2 bis. La relecture d'arbre principal (14 novembre 2026) : trois règles de plus

1. **Les arêtes portent aussi `_depuis` et `_jusqu_a`** (variante (a) d'arbre principal).
   - Avec des lignes immuables, un `f` changé prend une ligne neuve à uuid neuf. L'arête `A → f`
     d'un fichier inchangé visait l'ancienne ligne : sans règle, l'appel de `A` à `f` disparaîtrait de
     la vue courante à la première édition de `f`.
   - Quand une ligne neuve remplace une ligne vivante, ses **arêtes entrantes se recopient** vers la
     ligne neuve au rang n, et les anciennes se ferment à n. Les arêtes sortantes sont celles que
     l'analyse du fichier réécrit déjà. L'écriture paie, proportionnellement aux appelants ; la
     lecture reste un prédicat.
   - L'alternative (b), viser la clé stable et résoudre le bout à la lecture, coûterait une jointure
     par saut. Écartée.
   - **Le cas des scopes seulement déplacés.** Un scope dont le texte ne change pas mais dont les
     lignes bougent (une édition au-dessus) a une position neuve. Sa ligne est donc neuve elle aussi,
     et ses arêtes entrantes se recopient. C'est le cas des 72 % de déplacés de la mesure du 8
     novembre. Pour l'ancre, d'un commit au suivant, ce n'est que les fichiers touchés, mais **le coût
     d'écriture se mesure avant d'atterrir** (§ 5), sur une copie de code-lourd. Si c'est trop, la
     position sort de l'identité de la ligne vers un petit enregistrement par génération, et c'est
     alors à redécider.
2. **L'ancre a une couche de travail, et une génération par commit** (décision 2 de Lucie, page 30).
   - Une base de Lucie indexe l'arbre vivant, non commité compris, et la montre réindexe à chaque
     sauvegarde. Si chaque réindexation était une génération, la chaîne croîtrait à chaque frappe.
   - Donc, au-dessus du dernier commit indexé (rang n), les lignes de l'ancre au rang **n+1 en
     attente** sont sa couche de travail : réécrites sur place par la montre, et retirées sur place
     si l'édition revient en arrière. Au commit, le rang n+1 se fige et se publie, et un rang n+2 en
     attente s'ouvre.
   - **Sans vue**, la lecture prend l'ancre au rang en attente : c'est l'arbre vivant, comme
     aujourd'hui. Une couche épingle un rang figé, jamais le rang en attente.
3. **`File` reçoit les deux colonnes, `Symbol` reste partagé.**
   - `File` se réécrit sur place (`content_hash`, `lines_of_code`, `includes`), et la visibilité en
     dépend (`includes`). Une génération d'avant doit voir son `File` d'alors : `File` suit la même
     règle que `Scope`, et ses arêtes aussi.
   - `Symbol` est par nom et idempotent : partagé entre générations, sans colonnes.

Le témoin de plus (arbre principal) : `A` dans `a.rs` appelle `f` dans `b.rs` ; `f` change ;
`usages(f)` rend `A` dans la génération courante, et dans celle d'avant.

**L'ordre avec 1b** : 1b vise la forme du binaire. Si 2a′ atterrit avant que 1b joue sur les bases de
Lucie, sa migration triviale (`_depuis` = 0, `_couche` = ancre, la génération 0, sur `Scope`, `File`
et leurs arêtes) s'ajoute à la fin de 1b, sans deuxième passe.

## 3. Ce que le relevé dit des lecteurs, et où le filtre entre

**L'écriture** (deux points, qui empêchent les générations de s'écraser) :
- le **grain du snapshot** (`code_sync::grain_du_fichier`) prend `_couche`. Retirer, c'est poser
  `_jusqu_a`, et non effacer. Sinon, réindexer une branche efface les lignes de master ;
- **l'identité** (`uuid_for_in`) prend `[clé, couche, _depuis]` pour les lignes neuves (§ 2) ;
- l'ingest et `code_sync` (`reingest_file_retenu`, `synchroniser`, `remove_file`,
  `oublier_les_aretes_du_fichier`, `source_deja_indexee`) écrivent dans la chaîne et au rang de la
  génération en cours ;
- les lectures faites pendant l'ingest pour résoudre les arêtes (`materialiser_les_symboles`,
  `Declarants::charger_par`, `parents_et_fichiers`, `charger_les_types_ecrits`) lisent la vue de la
  génération en cours.

**Les points communs**, où le filtre entre une fois pour beaucoup de lecteurs :
1. **La traduction IR → dialecte** (`SchemaDialect::select`, `hop`, `count`, `dialect.rs:337-351`) :
   une clause de visibilité imposée quand la table est `Scope`, ou quand un `Hop` arrive sur `Scope`.
   Elle couvre `usages`, `impact`, `callees`, `impact_fichier`, les liens et la cohésion, les plans
   (`outline`, la navigation des symboles), `graph_walk`, `lock_order`,
   `mentions_sans_definisseur_visible` et les lectures de résolution de `code.rs`.
2. **`admitted_parent_uuids_for`** (`generic_search_nodes.rs:3011`), à côté de la condition de
   cellule : BM25, dense et creux sur `Scope`, donc `search_code`. Le repli sans condition
   d'`occurrence_roots_for` (l. 2975, `select_all`) se ferme en même temps. Le masque des versions en
   découle.
3. **`compile_filter_for_vector_in`** et **`resoudre_le_filtre`** (`read_view.rs:340`, 397) : les
   recherches sans projection. La visibilité y devient une clause imposée, puisque `resoudre_le_filtre`
   rend `None` sans clause.
4. **`read_for_tool`** (`catalog_read.rs:272`) : c'est là que la vue d'un outil se posera, en 2b-C.
   En 2a′, la vue est l'ancre au rang courant.

**À toucher un par un**, car ils ne passent par aucun point commun :
- `ReadView::find_by_field` et `get`, `Catalog::find_by_field` (SQL du dialecte à la main) : ils
  servent `read_file` et `grep_files` par `code_tools::scopes_of` ;
- les `Hop` à arrivée non typée (`code_tools::voisins_des_scopes` l. 1124, `code.rs::linked_from_many_where`
  l. 3448) : le prédicat doit s'appliquer au nœud atteint quand il est un `Scope` ;
- `occurrence_roots_for` (`select_all`), `derived_references`, `kb_gather_content` : filtrés par
  l'amont, à vérifier un par un ;
- `catalog.rs::enregistrer_l_entite` (l. 2108, 2148), `progress.rs`, `rebuild_fts_step`, et la chaîne
  du snapshot dans `catalog/synchronisation.rs`.

**La maintenance** : décider, entrée par entrée, si elle compte une génération ou toutes.
- La dette d'embarquement est par contenu partagé : elle reste commune.
- Les comptes d'état (`index_states_du_code`, `progress.rs`) disent l'ancre au rang courant.
- Le plein texte indexe `ScopeContent`, partagé : il reste commun, et sa visibilité vient du masque.
- La garde de `enregistrer_l_entite` compte toutes les lignes.

**Les arêtes Scope → Scope** (CONSUMES et les suivantes) ne traversent pas les générations : une
arête relie deux lignes de la même chaîne, ou une ligne de couche à une ligne de sa parente épinglée
(2b′). En 2a′, avec la clause sur le nœud atteint (point 1), un saut parti d'une ligne visible
n'atteint que des lignes visibles.

## 4. Les témoins de 2a′, rouges d'abord

0. **La non-fuite** : une génération de couche publiée à côté de l'ancre ne change **aucune** réponse
   sans vue, dans `search_code` (les trois signaux), `usages`, `impact`, `callees`, `outline`,
   `read_file` et `grep_files`.
1. **L'ancre garde son passé** : après une réindexation qui change `f`, la génération d'avant rend
   l'ancien `f`, et la courante le neuf. Aucune ligne n'est effacée.
2. **Aucune base ne change d'uuid** : une base de la forme d'aujourd'hui garde tous ses uuid. Ses
   lignes sont la génération 0.
3. **Une branche réindexée n'efface rien de master** (le grain du snapshot).
4. **Le prédicat** : une ligne retirée au rang n est visible à n-1 et pas à n.
5. **Les arêtes entrantes suivent** : `A` (`a.rs`) appelle `f` (`b.rs`) ; `f` change ; `usages(f)`
   rend `A` à la génération courante et à celle d'avant.
6. **La couche de travail de l'ancre** : dix sauvegardes du même fichier ne publient aucune
   génération ; un commit en publie une. Une lecture **sous une vue** de l'ancre à son rang publié n
   ne voit pas les sauvegardes en attente : c'est le cas qu'une couche épingle, et c'est lui qui
   prouve « jamais un rang en attente » (arbre principal).

**La mesure des déplacés, bornée pour trancher** (arbre principal) :
- **ce qu'elle compte** : sur les 50 derniers commits de master de code-lourd, rejoués sur une copie,
  le nombre d'arêtes entrantes recopiées par commit, et la part qui vient des seuls déplacés ;
- **le seuil** : si les déplacés font plus de la moitié des recopies **et** qu'un commit médian
  dépasse quelques milliers d'arêtes, la position sort de l'identité de la ligne ; sinon, on garde ;
- dans un fichier modifié, la couche de travail réécrit sur place : seul le commit qui publie paie,
  une fois par scope déplacé, jamais à chaque sauvegarde.

## 5. Ce qui se décide avant le code

- **Ce format, et l'estampille qui le porte** : `code_scope_form` reçoit une valeur neuve (par exemple
  `code-scope-v1/content/generations`). Une base de la forme d'avant reste lisible et « à migrer ». La
  migration est triviale : poser `_depuis` = 0, `_couche` = ancre, et la génération 0. Elle s'ajoute
  à 1b.
- **Le coût des deux colonnes** : un prédicat de plus sur chaque lecture de `Scope`. C'est à mesurer
  sur code-lourd, sur une copie, avant d'atterrir (la règle « mesurer une règle de résolution sur une
  vraie base »).
- **La croissance** : les lignes retirées s'accumulent jusqu'à la rétention. C'est le budget de 30 Go
  par dépôt, et la rétention de la page 30.
- **L'alternative**, si arbre principal la préfère : une table d'appartenance (génération →
  occurrences) au lieu des deux colonnes. C'est plus général (une génération y est un ensemble
  quelconque), mais une jointure de plus à chaque lecture. Je recommande les deux colonnes : une
  chaîne est un ordre total, et ce prédicat suffit à l'exprimer.

## 6. La mesure du coût du prédicat (14 novembre 2026, sur une copie de code-lourd)

**Le banc.**
- La copie reflink de l'instantané de G (`instantanes/avant-0c41f9fe6/code-lourd`, pris hôte arrêté le
  14 novembre à 13 h 37), son `.extensions` repointé sur la lib 0c41f9fe6. La base en service n'est
  jamais ouverte.
- Le contenu : 82 855 `Scope`, 7 278 `File`, 151 744 `CONSUMES`, conforme au relevé de G du
  8 novembre.
- La sonde `tests/sonde_cout_du_predicat.rs` (branche `generations-de-l-ancre`), seule sous
  `poste mesure`, à luciepc ; la médiane de 20 jeux, en ms. Les colonnes sont ajoutées par `ALTER`
  sur la copie seulement, avec leurs valeurs par défaut (toutes les lignes au rang 0, vivantes).

**Les chiffres** (deux jeux ; les écarts entre jeux sont de quelques pour cent) :

| requête | sans | avec le prédicat | nœuds seuls | arête seule | par appartenance |
|---|---|---|---|---|---|
| outline (un fichier, 1 275 lignes) | 7,0 | 7,4 (×1,05) | — | — | 9,0 (×1,28) |
| usages (un saut, 780 lignes) | 4,8 | 35,2 (×7,3) | 5,9 (×1,24) | 4,6 (×0,95) | 6,1 (×1,35) |
| impact (1 à 3 sauts) | 7,4 | 16,5 (×2,2) | 16,3 (×2,2) | — | — |
| racines d'une source (82 855) | 15,7 | 17,4 (×1,11) | — | — | 18,8 (×1,20) |
| compte des scopes | 0,18 | 1,9 (×10) | — | — | 0,45 (×2,4) |
| fichiers d'une source (7 278) | 1,4 | 1,7 (×1,20) | — | — | — |

Les `ALTER` coûtent 6 ms (`Scope`), 2 ms (`File`) et 17 ms (`CONSUMES`). Peupler la table
d'appartenance de la génération 0 coûte 646 ms, pour 82 855 arêtes.

**Ce que ça dit.**
- **Sur les nœuds, le prédicat ne coûte presque rien** : de 5 à 20 %.
- **Sur un saut, le surcoût n'est pas celui de l'arête.** L'arête seule est gratuite (×0,95), et les
  nœuds seuls coûtent ×1,24. C'est **leur conjonction dans une même clause** qui fait déraper le
  plan (×7,3). La traduction IR doit donc écrire la clause de l'arête à part de celle des nœuds, sous
  une forme que le planificateur ne combine pas mal, et la sonde le vérifiera dans 2a′-1. Si aucune
  forme ne l'évite, c'est un ticket pour le cœur (le planificateur de rag3db).
- **Le parcours récursif** (`impact`, filtre sur chaque nœud traversé) coûte ×2,2, soit 9 ms de plus
  en valeur absolue.
- **La table d'appartenance** coûte moins sur les sauts (×1,35) mais autant sur les nœuds (×1,2 à
  ×1,3). Elle demande une arête par occurrence et par génération : 82 855 de plus, et 646 ms, à
  chaque génération publiée. Une génération est un ordre total sur sa chaîne : les deux colonnes
  l'expriment sans ce coût.

**La décision proposée** : les deux colonnes, confirmées. Avant le code, la sonde de 2a′-1 cherche la
forme de clause qui rend `usages` à moins de ×1,3 avec le prédicat complet (nœuds et arête). Le
prédicat récursif d'`impact` (×2,2, quelques ms) est accepté tel quel.

## 7. Ce que 2a′-1 a posé, et ce qui reste à 2a′-2 (14 novembre 2026)

**Ce que 2a′-1 pose** (branche `generations-de-l-ancre-2`) :
- l'estampille `code-scope-v1/content/generations` sur une base neuve dont le dialecte sait les
  générations (`SchemaDialect::sait_les_generations` : rag3db seulement, postgres a son ticket) ;
- les trois colonnes `_couche` (défaut `'backend'`), `_depuis` (0) et `_jusqu_a` (vide) sur `Scope`
  et `File` ;
- l'entité `Generation`, en table interne `_generation` (comme `_catalog_meta` : pas une entité des
  outils, donc aucune réponse de `schema` ne change). La génération 0 de la couche y est posée à la
  première ouverture, et le rang lu est le dernier publié ;
- une base d'avant se lit comme avant, et `index_state` la dit « forme sans couches ».

**Où le prédicat entre** (`Visibilite`, `src/generations.rs`) :
- les plans, par `run_read_plan` (catalogue et vue) ;
- les sauts, par `graph_walk::run_hop`, bornés au nœud atteint ;
- les lecteurs qui traduisent l'IR eux-mêmes, par `ReadCatalog::traduire_select`, `traduire_hop` et
  `traduire_count` : `code.rs` (les noms, les parents, `linked_from_many_where`, le compte des
  arêtes ambiguës, `mentions_sans_definisseur_visible`), `code_tools::voisins_des_scopes`,
  `graph_walk::neighbors`, `lock_order`, `links_nodes`, `neighborhood_nodes`, `react_nodes`,
  `composable_results` ;
- les recherches, par `resoudre_le_filtre` et le filtre vectoriel (`compile_filter_for_vector_in`).
  `admitted_parent_uuids_for` y passe, et le repli `select_all` d'`occurrence_roots_for` se ferme
  avec lui ;
- `get` et `find_by_field` (catalogue et vue) lisent les trois colonnes et ne gardent que les lignes
  visibles.

**Le drapeau `tout_visible`** : sans filtre, « aucune clause » veut dire « toutes les lignes ». Tant
qu'aucune ligne n'est invisible, ce qui est le cas de toute base de 2a′-1, un lecteur sans filtre
n'ajoute pas de clause. La latence de `search_code` sans filtre ne bouge donc pas. L'écrivain qui
pose une seconde génération appelle `Catalog::marquer_des_lignes_invisibles` (2a′-2), et la clause
s'impose alors à tous.

**La mesure avant le drapeau** (condition de c7, 14 novembre 2026) : sans filtre, la clause fait
résoudre tous les offsets visibles de `Scope` à chaque `search_code`. Ce coût **se mesure dans 2a′-2,
sur la base de code-lourd (une copie), avant** que `marquer_des_lignes_invisibles` impose la clause.
C'est une mesure du lot, et le lot ne pose pas le drapeau sans elle.

**Les témoins** :
- 4 (`code::tests_des_generations`) : rouge sur 7072aa733 (`Cannot find property _couche`), vert ;
- l'entité `Generation` : rouge sur ee8b4aab4 (`Table _generation does not exist`), vert ;
- 0 (`backend::tests::une_generation_invisible_ne_change_aucune_reponse_des_outils`) : chaque ligne
  de `Scope` et de `File` est recopiée avec ses arêtes, une fois dans une autre couche et une fois
  retirée au rang lu.
  - **Le rouge** (branche `temoin-0-rouge`, le noyau seul sans les lecteurs un par un) : trois
    réponses changent, `read_file` sur deux fichiers et `search_code`.
  - **Le vert** : avec les lecteurs bornés.
  - **Un lecteur hors du témoin de non-fuite : `outline`.** Son plan
    (`@builtin:backends/code/plans/outline.json`) ne se charge pas dans le gabarit du témoin 0, qui
    ne déclare pas `Scope` au chargement du manifeste. Il passe par `run_read_plan`, que le témoin 4
    tient. 2a′-2 l'ajoute au témoin 0 si son plan ou ce gabarit change, et dans tous les cas le
    témoin à générations publiées de 2a′-2 le joue.

**La borne des arêtes, pour 2a′-2** (condition de c7) : en 2a′-1, aucune arête ne porte de
prédicat. C'est juste tant qu'aucune ligne n'est retirée alors que ses deux bouts restent visibles.
2a′-2, qui rend la ligne immuable et recopie les arêtes entrantes, doit :
- poser les colonnes sur les relations entre lignes à générations (`CONSUMES` et les suivantes, et
  `File`–`Scope`) ;
- borner l'arête d'un saut dans la **forme C** du § 6 : le bout filtré dans le `MATCH`, puis l'arête
  et l'autre bout après un `WITH`. Une clause unique nœuds et arête coûte ×7,3 (ticket
  `2026-11-14-un-predicat-sur-les-noeuds-et-l-arete-d-un-saut-fait-balayer-toute-la-table`) ;
- borner `Count::Edges` (le degré), que `borner_count` laisse aujourd'hui tel quel ;
- borner les sauts à arrivée non typée (`end: None`), que `borner_hop` laisse passer ;
- donner la vue de la génération en cours aux lectures de l'ingest, qui prennent encore le dialecte
  nu : `Declarants::charger_par`, `Inclusions::charger_par`, `materialiser_les_symboles`,
  `parents_et_fichiers` ;
- décider, entrée par entrée, pour la maintenance (`code_sync::avant_la_regle`, les `select` internes
  de `catalog.rs`, la chaîne du snapshot dans `catalog/synchronisation.rs`) : une génération, ou
  toutes (§ 3) ;
- mesurer la clause sans filtre sur code-lourd, puis appeler `marquer_des_lignes_invisibles` à la
  première écriture d'une seconde génération ;
- mettre `outline` dans le témoin de non-fuite, ou dire pourquoi il n'y est toujours pas.

## 8. Ce que `_couche` porte : un identifiant stable de couche (14 novembre 2026, 22 h 15)

Décidé avec arbre principal (voie (b)). Le détail est dans
`docs/arbre-principal/14-11-2026/02-les-suivies-tracked.md`, section « La décision » (master 6ba1427e4).

- **`_couche` porte l'identifiant de la couche, jamais le nom d'une ref.** L'identifiant entre dans
  l'uuid des lignes neuves (`[clé, couche, _depuis]`, § 2) : un renommage de branche ne change donc
  aucun uuid.
- **L'identifiant se dérive de la naissance de la couche** : hash(dépôt, nom complet, pointe à la
  première vue de la ref). Pour une couche de travail, c'est le chemin du worktree et le commit
  extrait. Il est gardé dans la fiche `fiche_couche:<dépôt>:<id>`, avec l'index
  `couche_nom:<dépôt>:<nom complet>`. Il est recalculable tant que la ref n'a pas bougé depuis sa
  naissance ; au-delà, la fiche fait foi.
- **La couche du backend garde l'identifiant `backend`** (`COUCHE_DU_BACKEND`). C'est la seule
  exception, pour que rien de 2a′-1 ne bouge.
- **Un identifiant ne se réutilise jamais pour une autre lignée.** Une ref effacée reçoit `fin`. Une
  ref recréée sous le même nom après sa fin reçoit un identifiant neuf.
- **Un renommage ne termine plus une couche** (ce qui retire la règle de la veille). Une ref ne se dit
  renommée que si, dans une même résolution, une seule ref a disparu et une seule est apparue sur la
  pointe vue en dernier, et qu'aucune autre ref vivante ne la porte. Sinon, c'est une fin et une
  naissance.
- Sous 2a′-3, l'écrivain lit l'identifiant par la suivie résolue, jamais par le nom de la ref.

## 9. La position sortira de l'identité avec 2a′-3 (14 novembre 2026, 22 h 55)

**La décision** (c7, avec arbre principal) : la position d'un scope reste dans l'identité de sa ligne
pour 2a′-2, et **en sort avec 2a′-3**, en une seule migration.

**Pourquoi pas maintenant** : ce n'est pas le coût. La recopie des arêtes entrantes se paie au commit,
quelques centaines de ms. Mais sortir la position de l'identité change les uuid de toutes les lignes
posées, donc une migration de plus, alors que 2a′-3 (les générations par les objets git, où la
génération devient l'arbre du commit) en impose déjà une. Les deux se font ensemble.

**La justification, écrite dès maintenant** : la mesure bornée des déplacés (page 06, § 9 ; sonde
`tests/sonde_deplaces.rs`, branche mesure-deplaces, 03f6fc2c9). Sur les 50 derniers commits de
master, 44 849 arêtes entrantes seraient recopiées, **dont 86 % par des scopes seulement déplacés**,
qui n'achètent qu'un numéro de ligne. Le commit de code médian recopie 1 500 arêtes, le pire 12 444.

**Ses deux réserves, qui vont avec la décision** :
1. les arêtes entrantes sont lues sur une seule base (la base d'avant les versions), et non à chaque
   commit : c'est un ordre de grandeur ;
2. un fichier `.md` compte comme un fichier de code, avec ses scopes de texte, mais sans arête : il ne
   pèse pas dans les recopies.

**D'ici 2a′-3** : l'écrivain de 2a′-2 dit à chaque publication le compte des recopies et la part des
déplacés, et un commit réel au-delà de 20 000 recopies rouvre la question.
