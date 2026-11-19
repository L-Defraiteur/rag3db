# La campagne des correctifs passés d'Algareth : les cas jugés

La forme, le juge et la grille sont dans `docs/banc/19-11-2026/02-la-campagne-d-algareth-le-juge-et-la-grille.md`.
Une ligne par cas joué ; les détails suivent la table.

## La grille

| cas | verdict | endroit | taille (Algareth / nôtre) | coût | dégâts à côté | indices présents, suivis ? |
|---|---|---|---|---|---|---|
| tty 09, une demande écrite une fois (`b1163f64e1`) | **écarté** : latence de Vertex (jeu du 19 à 16 h 26) ; à rejouer | aucun diff | 0 / +36 −3 (tui.rs +7 −2, vue.rs +29 −1 ; hors docs) | 2 pas, 45 appels, 1 814 s de modèle sur 1 828 s ; l'appel le plus lent 385 s pour 8 695 jetons d'invite | aucun (diff vide) | la souche `tui::nom_garde -> None` ; non suivie (rien n'est écrit) |

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
- **Un point à éclaircir sur les compteurs** : la trace compte 156 794 jetons d'entrée et **1 186 786** jetons servis
  par le cache. Le cache dépasse l'entrée : `input_tokens` ne compte sans doute que la part non servie par le cache.
  Il faut le confirmer avant d'en tirer la colonne « coût » et la part servie par le cache (ticket R, P5).
- **À ne pas compter contre l'agent** : dans le bac à sable, la suite `tty` rend 3 rouges de plus, des tests de
  coquille qui ouvrent un pseudo-terminal refusé par le bac à sable. Le ticket est
  `docs/rag3weaver-archi/tickets/2026-11-19-le-bac-a-sable-refuse-le-pseudo-terminal.md`.
- Le premier jeu du même cas (`20261119-160735`) ne compte pas : cargo n'y trouvait pas la chaîne Rust.
