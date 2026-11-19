# La campagne des correctifs passés d'Algareth : les cas jugés

La forme, le juge et la grille sont dans `docs/banc/19-11-2026/02-la-campagne-d-algareth-le-juge-et-la-grille.md`.
Une ligne par cas joué ; les détails suivent la table.

## La grille

| cas | verdict | endroit | taille (Algareth / nôtre) | coût | dégâts à côté | indices présents, suivis ? |
|---|---|---|---|---|---|---|
| tty 09, une demande écrite une fois (`b1163f64e1`) | **écarté** : latence de Vertex (jeu du 19 à 16 h 26) ; à rejouer | aucun diff | 0 / +36 −3 (tui.rs +7 −2, vue.rs +29 −1 ; hors docs) | 2 pas, 45 appels, 1 814 s de modèle sur 1 828 s ; l'appel le plus lent 385 s pour 8 695 jetons d'invite ; invite totale 1,34 M jetons, dont 88 % par le cache | aucun (diff vide) | la souche `tui::nom_garde -> None` ; non suivie (rien n'est écrit) |
| tty 09, jeu du 19 à 18 h 06 | **dégâts** : les deux témoins déclarés passent, mais un test vert au parent rougit, par une assertion qu'Algareth lui a ajoutée | **même** (tui.rs, vue.rs) et **autre** (boite.rs) | +46 −5 (dont +13 de tests) / +36 −3 | 5 pas, 165 appels, 1 181 s (1 155 s de modèle) ; invite totale 5,31 M jetons, dont 87 % par le cache ; sortie 2 647, raisonnement 92 592 | `vue::tests::the_view_keeps_the_last_agent_name_seen` vert au parent, rouge après | la souche `nom_garde` : **suivie** (remplie et branchée dans `/nom`) |
| lucivy 04, l'ulp d'écart en parse (`e3c9753f7f`) | **réussi** | **autre** : `buffered_union.rs` (Algareth) contre `score_combiner.rs` et `block_wand.rs` (nous) | +5 −22 / +17 −9 (hors le témoin, +81) | 2 pas, 54 appels, 464 s ; invite 1,66 M jetons, dont 82 % par le cache | aucun : le seul rouge de la suite l'était déjà au parent | trois fichiers de l'arbre, sans souche, absents de la consigne ; Algareth en a corrigé un |
| ED 02, le préfixe d'une décision (`f063a90e52`) | **réussi** | **même** (`decide_nodes.rs`, et la même assertion de `decide_nodes/tests.rs`) | +13 −3 / +15 −3 | 3 pas, 72 appels, 680 s ; invite 2,09 M jetons, dont 58 % par le cache | aucun : la lib entière passe (1 745) | le fichier indiqué ; Algareth y a corrigé la cause |
| ED 01, le journal plafonné (`b610560e51`) | **échoué** (réserve : Vertex lent) | aucun diff (nous : `commande.rs`) | 0 / +10 −2 | 3 pas, 34 appels, 1 388 s, dont 640 et 706 s pour les deux premiers pas ; invite 0,52 M jetons, dont 70 % par le cache | aucun | le fichier indiqué, lu sans conclure |

## tty 09 : une demande s'écrit une fois, et /nom retient le nom gardé (19 novembre)

- **Le cas** : `docs/tty/19-11-2026/09-cas-de-campagne-une-demande-ecrite-une-fois.md` (branche `cx-campagne-cas-tty`).
- **Le rouge, prouvé au parent** `b1163f64e1` par le banc, sur la tablette, crate `extension/rag3weaver/tty` :
  - `tui::tests::the_name_kept_by_the_host_is_the_one_the_view_keeps` échoue sur `assertion left == right`
    (`tui.rs:947`) ;
  - `vue::tests::what_you_send_is_written_once_even_when_the_mailbox_gives_it_back` échoue sur « la boîte la rend :
    déjà écrite » (`vue.rs:1691`).

  Ce sont des assertions, pas des erreurs de compilation.
- **Le tour** : dossier `~/.cache/rag3weaver-build/campagne/09-cas-de-campagne-une-demande-ecrite-une-fois/20261119-162624/`
  sur la tablette (lanceur d'archi).
  - `diff.patch` est vide, et `budget.json` dit `stopped_by: "wall"` après 1 831 s pour un budget de 1 800 s.
  - La boîte montre un seul pas fait : « Analyse des deux défauts terminée. Les causes sont identifiées dans tui.rs,
    vue.rs et boite.rs. » Le pas suivant (« Appliquer les corrections… puis vérifier les tests avec cargo test ») était
    parti quand le budget l'a arrêté.
- **Le premier verdict, « échoué, faute de budget », est requalifié en écarté.** Le `trace.json` du lot 3 d'archi
  dit : 45 appels au modèle, **1 814 s de modèle sur 1 828 s d'horloge**, 14 s d'outils, aucune reprise, aucun quota.
  L'appel le plus lent prend 385 s pour 8 695 jetons d'invite et 34 jetons de sortie ; le suivant 285 s. C'est la
  latence de Vertex à ce moment-là, pas l'agent ni un outil. Le jeu ne se juge pas sur ce qu'il n'a pas pu faire.
  Archi le rejoue.
- **Les compteurs, éclaircis par archi** : `input_tokens` ne compte que la part de l'invite que le cache n'a pas
  servie, et `cached_tokens` la part servie. L'invite totale est leur somme. La preuve : pour chaque pas, la somme des
  `prompt_tokens` des appels moins celle de leurs `cached_prompt_tokens` donne exactement `input_tokens` (pas 1 :
  1 101 867 − 1 011 581 = 90 286 ; pas 2 : 241 713 − 175 205 = 66 508). Pour ce jeu, l'invite totale vaut
  **1 343 580 jetons, dont 88 % servis par le cache**. Le prochain lot du lanceur ajoute `prompt_tokens` (le total) et
  renomme `input_tokens` en `uncached_input_tokens`.
- **À ne pas compter contre l'agent** : dans le bac à sable, la suite `tty` rend 3 rouges de plus, des tests de
  coquille qui ouvrent un pseudo-terminal refusé par le bac à sable. Le ticket est
  `docs/rag3weaver-archi/tickets/2026-11-19-le-bac-a-sable-refuse-le-pseudo-terminal.md`.
- Le premier jeu du même cas (`20261119-160735`) ne compte pas : cargo n'y trouvait pas la chaîne Rust.

### Le jeu du 19 novembre à 18 h 06 (`20261119-180649`), jugé

- **Le tour** : Vertex était rapide, d'après la sonde d'archi (2 à 5 s par appel juste avant). Algareth a fini de
  lui-même (« fait sans plan »), sans heurter le budget. Il fait 5 pas, 165 appels et 1 181 s, dont 1 155 s de modèle.
- **Le diff** : 3 fichiers, +46 −5 :
  - `tui.rs` : `nom_garde` est rempli et branché dans `/nom` ;
  - `vue.rs` : le nom retenu et la ligne que la boîte rend ;
  - `boite.rs` : l'en-tête porte un nom de repli.
- **Le juge**, sur la tablette, dans le dépôt réel au parent `b1163f64e1`, puis avec `diff.patch` appliqué. C'est la
  suite `tty` de la passe : avec et sans la vue, clippy `-D warnings`, et le bâti Release.

| | parent | avec le diff d'Algareth |
|---|---|---|
| `cargo test` (vue) | 109 passés, 2 rouges : les deux témoins | 111 passés, 1 rouge : `the_view_keeps_the_last_agent_name_seen` |
| sans la vue | 20 passés | 21 passés |
| clippy `-D warnings`, Release | verts | verts |

- **Les témoins déclarés** : rouges au parent, verts après, et intacts (le diff ne touche aucune de leurs lignes).
- **Le test qui rougit** : `vue::tests::the_view_keeps_the_last_agent_name_seen` est **vert au parent** dans le dépôt
  réel. Archi l'avait vu rouge au parent, mais dans le bac à sable. Algareth lui a ajouté deux lignes, et c'est son
  assertion qui échoue (`vue.rs:1693`). Le comportement est bon : la ligne `[Algareth · message] bonjour` porte bien le
  nom retenu. Mais l'assertion attend une seule ligne, alors que `prendre_lignes` rend toutes les lignes écrites depuis
  le début du test. **C'est une assertion fausse, pas une régression du code.**
- **Le verdict : dégâts.** Au sens de la grille, un test vert au parent rougit après le passage d'Algareth. La cause
  est un test qu'il a lui-même modifié, et il n'avait pas à toucher un test qui n'était pas le témoin.
- **Les mesures `plain_*`** de la suite `tty` n'existent pas au parent : elles sont postérieures au cas. Elles ne
  s'appliquent pas.
- **Les sorties** du juge sont à la tablette : `~/.cache/rag3db-banc-notes/campagne/09-20261119-180649/` (`parent.log`,
  `algareth.log`), et le juge est `~/.cache/rag3weaver-build/juge-09.sh`.

## lucivy 04 : l'ulp d'écart en parse (19 novembre, jeu de 18 h 48)

- **Le cas** : `docs/lucivy/19-11-2026/04-cas-de-campagne-un-ulp-d-ecart-en-parse.md`. Le dépôt est un clone nu de
  lucivy (`~/.cache/rag3weaver-build/campagne/depots/lucivy.git`). Le témoin n'existe pas au parent : son fichier,
  `lucivy_core/tests/test_parse_scores_identical_across_layouts.rs`, est pris de notre correctif `3933431475` et posé
  sur le parent `e3c9753f7f`.
- **Le tour** : Algareth a fini de lui-même, en 2 pas, 54 appels, 464 s, sans reprise.
- **Le diff** : `src/query/union/buffered_union.rs`, +5 −22. Il remplace `unordered_drain_filter` et son `swap_remove`
  par `retain_mut`, qui garde l'ordre des scorers. Ainsi, l'ordre de la somme en f32 ne dépend plus de la disposition
  des segments.
- **Le juge**, sur la tablette, dans un clone à part et un target à part :

| | parent et témoin | avec le diff d'Algareth |
|---|---|---|
| le témoin | rouge, sur l'assertion (`…_layouts.rs:80`) | **vert** |
| `cargo test --workspace` | 2 200 passés, 2 rouges : le témoin et `derived_in_ram_answers_like_the_files` | 2 201 passés, 1 rouge : `derived_in_ram_answers_like_the_files` |

- **Le verdict : réussi.**
  - Le témoin passe, et son fichier n'est pas touché par le diff.
  - Le seul rouge restant (`derived_in_ram_answers_like_the_files`, `tests/common/mod.rs:52`) l'était déjà au parent :
    ce n'est pas un dégât.
- **La comparaison** :
  - **Notre correctif** additionne les scores d'un OU en f64, arrondis une fois (`score_combiner.rs`,
    `block_wand.rs`). La somme ne dépend alors plus de son ordre.
  - **Algareth** garde l'ordre des clauses dans l'union, ce qui fixe l'ordre de la somme en f32.
  - Les deux passent le témoin. Le nôtre rend la somme indépendante de l'ordre, alors que celui d'Algareth la rend
    seulement constante d'une disposition à l'autre. C'est plus étroit, mais exact sur ce que le témoin exige. Le
    titre de notre correctif nomme d'ailleurs le même `swap_remove` comme cause.
- **Les sorties** du juge sont à la tablette, `~/.cache/rag3db-banc-notes/campagne/lucivy-04-20261119-184828/`, et le
  juge est `~/.cache/rag3weaver-build/juge-campagne-2.sh`.

## ED 02 : le préfixe d'une décision (19 novembre, jeu de 19 h 22)

- **Le cas** : `docs/everything-declarative/19-11-2026/02-cas-de-campagne-le-prefixe-d-une-decision.md`, au parent
  `f063a90e52`, avec le témoin `dataflow::decide_nodes::tests::two_decisions_on_the_same_text_begin_with_the_same_bytes`.
- **Le tour** : Algareth a fini de lui-même, en 3 pas, 72 appels et 680 s, sans reprise.
- **Le diff** : `decide_nodes.rs` +12 −2 et `decide_nodes/tests.rs` +1 −1. Il remet les étiquettes admises en fin de
  tour utilisateur, pour que deux décisions sur le même texte gardent un préfixe commun.
- **Le fichier de tests touché** : ce n'est pas l'assertion du témoin. C'est celle de
  `decide_by_llm_asks_a_schema_and_labels_the_element`, qui fige la forme de l'invite. Notre correctif `1bba221689`
  la change à l'identique, avec le même texte attendu : c'est la suite légitime du changement de forme.
- **Le juge**, à luciepc (le portable était laissé à la bascule), dans un worktree et un target à part :

| | parent | avec le diff d'Algareth |
|---|---|---|
| suite `dataflow::decide_nodes::` | 28 passés, 1 rouge : le témoin, sur son assertion (`tests.rs:212`) | **29 passés** |
| lib entière | — | **1 745 passés**, 0 rouge |

- **Le verdict : réussi**, au même endroit que notre correctif, et de taille voisine (+13 −3 contre +15 −3).

## ED 01 : le journal plafonné (19 novembre, jeu de 18 h 56)

- **Le cas** : `docs/everything-declarative/19-11-2026/01-cas-de-campagne-le-journal-plafonne.md`, au parent
  `b610560e51`, avec le témoin `commande::tests::en_fond_un_journal_au_dela_du_plafond_garde_la_fin_et_le_dit`.
- **Le tour** : l'équipier s'est arrêté sur « 3 pas de suite sans élément réussi », en 1 388 s, avec 34 appels et un
  diff vide. Il a lu `commande.rs`, approché la cause par écrit (le calcul de `apres` avant la troncature), et n'a rien
  modifié.
- **Le rouge**, prouvé par le juge (tablette, worktree et target à part) : le témoin est **rouge 5 fois sur 5** sur son
  assertion (`commande.rs:1772`), trois fois joué seul et deux fois dans la lib entière (1 743 passés, 1 rouge). Le
  lanceur l'avait aussi vu rouge sur l'assertion (`red.json`). **La consigne du cas dit à tort que le témoin « passe
  seul »** : Algareth avait bien un rouge à corriger, et l'a vu.
- **Le verdict : échoué**, avec une réserve. Vertex a été lent pendant ce jeu, avec des appels de 85 à 155 s pour 11 à
  19 k jetons, et 640 et 706 s pour les deux premiers pas. La règle d'arrêt compte des pas, pas du temps : la lenteur
  n'a pas coupé le tour, mais rien ne dit qu'un Vertex rapide l'aurait fait aboutir. Un second jeu, avec un Vertex
  rapide et la consigne corrigée, départagerait.
- **Les sorties** des juges sont dans `~/.cache/rag3db-banc-notes/campagne/ed-01-20261119-185644/` (tablette) et
  `ed-02-20261119-192205/` (luciepc).
