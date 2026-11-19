# La campagne des correctifs passés d'Algareth : les cas jugés

La forme, le juge et la grille sont dans `docs/banc/19-11-2026/02-la-campagne-d-algareth-le-juge-et-la-grille.md`.
Une ligne par cas joué ; les détails suivent la table.

## La grille

| cas | verdict | endroit | taille (Algareth / nôtre) | coût | dégâts à côté | indices présents, suivis ? |
|---|---|---|---|---|---|---|
| tty 09, une demande écrite une fois (`b1163f64e1`) | **écarté** : latence de Vertex (jeu du 19 à 16 h 26) ; à rejouer | aucun diff | 0 / +36 −3 (tui.rs +7 −2, vue.rs +29 −1 ; hors docs) | 2 pas, 45 appels, 1 814 s de modèle sur 1 828 s ; l'appel le plus lent 385 s pour 8 695 jetons d'invite ; invite totale 1,34 M jetons, dont 88 % par le cache | aucun (diff vide) | la souche `tui::nom_garde -> None` ; non suivie (rien n'est écrit) |
| tty 09, jeu du 19 à 18 h 06 | **dégâts** : les deux témoins déclarés passent, mais un test vert au parent rougit, par une assertion qu'Algareth lui a ajoutée | **même** (tui.rs, vue.rs) et **autre** (boite.rs) | +46 −5 (dont +13 de tests) / +36 −3 | 5 pas, 165 appels, 1 181 s (1 155 s de modèle) ; invite totale 5,31 M jetons, dont 87 % par le cache ; sortie 2 647, raisonnement 92 592 | `vue::tests::the_view_keeps_the_last_agent_name_seen` vert au parent, rouge après | la souche `nom_garde` : **suivie** (remplie et branchée dans `/nom`) |

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
