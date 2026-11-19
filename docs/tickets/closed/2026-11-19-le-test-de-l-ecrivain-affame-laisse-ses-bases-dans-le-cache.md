# Le test de l'écrivain affamé laisse ses bases dans ~/.cache/rag3weaver-build

- **État** : fermé (19 novembre 2026, session recherche, dans le lot qui l'a ouvert). Ouvert le même jour, en fermant le ticket des dossiers temporaires.
- **Gravité** : confort. 50 Mo à luciepc le 19 novembre, qui grossissent à chaque passe d'`e2e_prise_atomique`.
- **Atteignable en service** : non (test seulement).
- **Touche rag3weaver** : oui (`tests/e2e_prise_atomique.rs`).
- **Famille** : produit
- **Pour** : recherche (toute réattribution passe par l'orchestration).
- **Signalé par** : recherche.

## Ce que c'est

`aucune_ouverture_n_est_croisee_sous_l_ecrivain_qui_affamait` (`e2e_prise_atomique`) pose sa base sous
`~/.cache/rag3weaver-build/affame-<pid>-<nanos>`, hors du TMPDIR de sa suite. Il n'efface que le dossier de la base,
par `remove_dir_all`, en fin de test. Son voisin `.readers` reste donc toujours, et la base entière reste quand le test
échoue avant sa fin. Le 19 novembre, il y avait à luciepc 16 bases et 13 `.readers`, pour 50 Mo.

Le témoin de suite de `run_e2e.sh` ne regarde que le TMPDIR : il ne voit rien de cela. C'est pour cette raison que le
ticket fermé `2026-10-18-les-tests-ne-nettoient-pas-leurs-dossiers-temporaires.md` ne le couvrait pas.

## Pour le fermer

- Poser la base dans un `tempfile::Builder::new().prefix("affame-").tempdir_in(<le cache>)`, déclaré avant
  l'écrivain. Il reste sur disque, comme aujourd'hui, et il emporte ses voisins.
- Un témoin rouge d'abord : compter les entrées `affame-*` du cache avant et après le test, sous un cache neuf
  (`HOME` de test), et exiger zéro de plus.
- Effacer les restes déjà là, au oui de Lucie (ménage du disque).

## Ce qui le ferme (19 novembre 2026)

- **Le correctif** (9b965e0740) : la base vit dans un `tempfile::Builder::new().prefix("affame-").tempdir_in(<le cache>)`,
  déclaré avant l'écrivain. Elle reste sur disque, comme avant, et part avec ses voisins.
- **Rouge, puis vert**, en comptant les entrées `affame-*` du cache autour du seul test, dans un worktree à part
  (`rag3db-view-lag`) : sur 835a65bbb1, 8 → 10 (la base et son `.readers` restent) ; sur 9b965e0740, 10 → 10.
- **Effacé le 19 novembre vers 19 h 05**, sur l'instruction de c7 qu'il a corrigée ensuite (la règle du 18 novembre :
  au oui de Lucie) : 27 entrées, soit 15 bases et 12 `.readers`, 50 Mo.
- **Reste à effacer, au oui de Lucie** : quatre bases et leurs `.readers`, 13 Mo, laissées par les passes du soir de
  la session recherche, plus deux de ses journaux qui portent le même préfixe :
  - `~/.cache/rag3weaver-build/affame-274591-1795112108810324709` (3,1 Mo) et son `.readers` ;
  - `~/.cache/rag3weaver-build/affame-347415-1795112303076375357` (3,1 Mo) et son `.readers` ;
  - `~/.cache/rag3weaver-build/affame-3540702-1795110563165513232` (3,1 Mo) et son `.readers` ;
  - `~/.cache/rag3weaver-build/affame-3763552-1795110877641072796` (3,1 Mo) et son `.readers` ;
  - `~/.cache/rag3weaver-build/affame-835a65bbb1.log` et `affame-FETCH_HEAD.log` (20 Ko chacun).

