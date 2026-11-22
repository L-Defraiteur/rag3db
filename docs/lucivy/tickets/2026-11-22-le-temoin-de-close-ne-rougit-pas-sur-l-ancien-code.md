# Le témoin de close() ne rougit pas sur l'ancien code

- **État** : ouvert (22 novembre 2026).
- **Gravité** : un témoin qui ne prouve rien. Le correctif de lucivy 4.5.0 (`pending_merge_tasks` devient un
  compteur de lots de fusions) est confirmé par la reproduction du résidu, pas par son témoin.
- **Atteignable en service** : non : le défaut est corrigé dans lucivy 4.5.0 ; ce ticket ne porte que sur sa preuve.
- **Touche rag3weaver** : non (lucivy).
- **Famille** : moteur
- **Pour** : lucivy
- **Signalé par** : lucivy, en relisant les passes du correctif (22 novembre 2026)

## Ce qu'on a vu

- `tests/test_close_is_quiet.rs` (`nothing_writes_in_the_directory_after_close`) ferme un index non vidé, puis exige
  que son dossier ne change pas pendant une seconde. Il est vert sur le code d'avant le correctif (b306ef3), trois
  passages sur trois : il ne rejoue pas la course.
- La reproduction qui, elle, rougit : `test_fuzzy_monotonicity` joué sous un `TMPDIR` neuf laisse
  `lucivy-test-test_monotonicity_segments-*` (tous les fichiers d'un segment fusionné) aux passages 1 et 2 sur `main`
  a4289f4 ; avec le correctif, 0 résidu en 20 passages (luciepc, 22 novembre).

## À faire

Réécrire le témoin pour qu'il rejoue la séquence du résidu : construire l'index comme `build_index` de
`test_fuzzy_monotonicity` (même nombre de documents, un commit tous les 200), le fermer, supprimer aussitôt son dossier,
attendre, puis exiger qu'il n'existe plus, sur plusieurs tours. Le voir rouge sur b306ef3 avant de fermer ce ticket.
