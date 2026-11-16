# La suite de tests du shell écrit dans l'historique de la personne

- **État** : fermé (16 novembre 2026, hotfixs) : le lot `shell-tests-own-history-2` (8d404496d) donne à tout `ShellTest` son propre historique.
- **Famille** : moteur
- **Gravité** : une suite de tests qui modifie un fichier de la personne (`~/.rag3db/history.txt`) sur le poste où elle
  tourne. Et un test peut à nouveau dépendre de ce qu'y ont laissé les tests d'avant, comme
  `test_prev_history_multiline` (ticket `2026-11-15-test-prev-history-multiline-du-shell-expire-par-intermittence.md`).
- **Atteignable en service** : non (tests du shell du moteur)
- **Touche rag3weaver** : non
- **Pour** : hotfixs (lot suivant de celui des fixtures)

## Ce qu'on voit

Sans `-p`, le shell prend `history.txt` dans le dossier courant s'il y en a un, sinon `~/.rag3db/history.txt`
(`tools/shell/shell_runner.cpp:134-150`), et il y ajoute chaque requête. Le lot `shell-test-isolated-history` donne
un historique à soi aux fixtures `multiline` et `singleline`. Restent environ 130 `ShellTest()` lancés sans `-p`,
surtout dans `test_shell_basics.py` (20), `test_shell_commands.py` (51) et `test_shell_flags.py` (56). Après une
passe de la suite le 16 nov. à luciepc, `~/.rag3db/history.txt` avait changé.

## Pour fermer

1. `ShellTest` reçoit par défaut un dossier d'historique temporaire, neuf par instance, sauf si le test en donne un
   (`-p`). Les tests qui lisent `history.txt` gardent le leur.
2. Un témoin : la suite entière laisse `~/.rag3db/history.txt` intact (même empreinte avant et après), sous un `HOME`
   de test.

## Fermeture (16 nov.)

Le lot `shell-tests-own-history-2` est passé par le mode « tests seuls » de la garde (eeb1ba8db5). Ses commits :

- **55aaada11** : le témoin, une fixture de session qui compare les historiques partagés avant et après la suite.
- **187fec001** : le témoin regarde aussi `history.txt` du dossier courant. Un test de la suite l'y crée, et tout shell
  sans `-p` le préfère alors à `~/.rag3db` (`shell_runner.cpp:139`). Sous un `HOME` temporaire, le témoin est rouge :
  « the suite changed …/tools/shell/test/history.txt », 245 passés et 1 erreur.
- **8d404496d** : le correctif. Tout `ShellTest` sans `-p` reçoit un `TemporaryDirectory` tenu par l'instance, posé
  juste après l'exécutable, pour que les tests de drapeau sans valeur (`-m`, `--max_db_size`…) finissent toujours sur
  leur drapeau.

La passe, à luciepc, avec un shell bâti de 8d404496d (sha256 2297d8da…) : 245 passés, le témoin vert, aucun dossier
laissé sous `TMPDIR`, aucun `history.txt` dans l'arbre, aucun `~/.rag3db` créé.
