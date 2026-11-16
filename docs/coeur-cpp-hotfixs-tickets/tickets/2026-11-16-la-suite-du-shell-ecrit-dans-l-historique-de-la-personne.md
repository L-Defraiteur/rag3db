# La suite de tests du shell écrit dans l'historique de la personne

- **État** : ouvert (16 novembre 2026, hotfixs)
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
