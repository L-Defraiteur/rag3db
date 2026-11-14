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
