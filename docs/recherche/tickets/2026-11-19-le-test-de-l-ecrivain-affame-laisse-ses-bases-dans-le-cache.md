# Le test de l'écrivain affamé laisse ses bases dans ~/.cache/rag3weaver-build

- **État** : ouvert (19 novembre 2026, session recherche, relevé en fermant le ticket des dossiers temporaires).
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
