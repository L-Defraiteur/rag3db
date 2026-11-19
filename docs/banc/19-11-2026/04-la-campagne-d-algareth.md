# La campagne des correctifs passés d'Algareth : les cas jugés

La forme, le juge et la grille sont dans `docs/banc/19-11-2026/02-la-campagne-d-algareth-le-juge-et-la-grille.md`.
Une ligne par cas joué ; les détails suivent la table.

## La grille

| cas | verdict | endroit | taille (Algareth / nôtre) | coût | dégâts à côté | indices présents, suivis ? |
|---|---|---|---|---|---|---|
| tty 09, une demande écrite une fois (`b1163f64e1`) | **échoué, faute de budget** (`stopped_by: wall`) | aucun diff | 0 / +36 −3 (tui.rs +7 −2, vue.rs +29 −1 ; hors docs) | 1 pas en 1 831 s, budget de 30 min ; appels et jetons sans `trace.json` (lot 3 d'archi) | aucun (diff vide) | la souche `tui::nom_garde -> None` ; non suivie (rien n'est écrit) |

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
- **Le verdict** : **échoué, faute de budget**. Avec un diff vide, le témoin reste au parent, donc rouge. Il n'y avait
  rien à rejouer.
- **Ce qui reste à départager** avant d'en conclure quoi que ce soit sur Algareth : un seul pas en 30 minutes est
  lent. Le lot 3 d'archi mesurera le temps du modèle par appel (`trace.json`).
- **À ne pas compter contre l'agent** : dans le bac à sable, la suite `tty` rend 3 rouges de plus, des tests de
  coquille qui ouvrent un pseudo-terminal refusé par le bac à sable. Le ticket est
  `docs/rag3weaver-archi/tickets/2026-11-19-le-bac-a-sable-refuse-le-pseudo-terminal.md`.
- Le premier jeu du même cas (`20261119-160735`) ne compte pas : cargo n'y trouvait pas la chaîne Rust.
