# Un test du shell expire par intermittence : test_prev_history_multiline[multiline]

- **État** : fermé (16 novembre 2026, hotfixs) : le correctif est sur master (9b932226e, 08643d923), et la CI de master est verte depuis.
- **Famille** : moteur
- **Gravité** : test instable. Un rouge sans défaut sur un commit qui ne touche pas le shell ; il apprend à passer
  outre un rouge du job shell test.
- **Atteignable en service** : non (test du shell du moteur, en CI)
- **Touche rag3weaver** : non
- **Pour** : hotfixs

## Ce qu'on voit

Le run 43748194127 du « CI Workflow » sur 76f1d44ed (18 h 41 UTC) est rouge sur un seul test, 1 échec pour 245 :
`tools/shell/test/test_shell_esc_edit.py::test_prev_history_multiline[multiline]`. Le commit est le lot nom + persona
d'ED, du Rust de l'hôte, qui ne touche pas au shell. La variante `[singleline]` passe dans le même run, et le run
suivant, 43749446484 sur f4131ed40, est vert, ce test compris. La relance du job (tentative 2) a été annulée par le
groupe de concurrence et n'a pas tranché.

L'attente qui expire est la troisième du test, `test_shell_esc_edit.py:206` :

```python
test.send_control_statement(KEY_ACTION.CTRL_P.value)  # remonter dans l'historique
test.send_control_statement(KEY_ACTION.CTRL_P.value)
test.send_control_statement(KEY_ACTION.CTRL_P.value)  # remonter d'une ligne
test.send_statement("\x1b[B")
test.send_statement("\x1b[B")
test.send_finished_statement(KEY_ACTION.ENTER.value)
assert test.shell_process.expect_exact(["│ databases rule │", pexpect.EOF]) == 0   # ← TIMEOUT
```

`pexpect.spawn` est créé sans délai (`conftest.py:151`), donc avec celui de pexpect, 30 secondes. Trente secondes
sans résultat, ce n'est pas de la lenteur : la requête attendue n'a pas été envoyée.

## La piste, à vérifier

Le test envoie ses touches sans attendre que le shell ait traité la précédente. La flèche vers le bas est une
séquence d'échappement de trois octets (`ESC [ B`). Si le shell lit `ESC` seul dans une lecture, puis `[B` dans la
suivante, il peut la prendre pour une touche Échap suivie de deux caractères, sous charge du runner. Le curseur ne
redescend pas, et Entrée valide autre chose que « databases rule », ou une ligne encore ouverte en mode multiligne.

## La cause, prouvée (16 nov., hotfixs)

La piste de l'échappement coupé en deux est fausse. `linenoiseReadEscapeSequence` lit octet par octet, en `read`
bloquants (`VMIN=1`) : un `ESC` séparé de `[B` est bien relu comme une flèche.

La vraie cause est l'historique. Les fixtures `multiline` et `singleline` lançaient le shell sans `-p`. Il lisait et
écrivait donc `~/.rag3db/history.txt`, le fichier de la personne, d'un test à l'autre. Or, sous Linux, Ctrl-P ne fait
que remonter l'historique : il ne monte d'une ligne que sous Windows (`#ifdef _WIN32`, `linenoise.cpp`).

- **Quand le test passe.** L'historique hérité des tests d'avant contient une entrée plus ancienne que la première
  requête. Le troisième Ctrl-P remonte sur elle, les deux flèches retombent sur « databases rule », et Entrée l'envoie.
- **Quand il échoue.** Sans entrée plus ancienne, Ctrl-P s'arrête sur la première requête. La seconde flèche descend
  alors jusqu'à la ligne vide, et Entrée n'envoie rien : le tampon de l'expiration finit par `rag3db> \r\nrag3db> `.

La variante `[singleline]` passait toujours, parce que son `:singleline` est lui-même une entrée de l'historique.

Les preuves, à luciepc, avec une copie figée du shell (rag3db-colonnes, ec7a0bf30 ; `tools/shell` n'y diffère pas de
master) :

| Situation | Résultat |
| --- | --- |
| Boucle sans charge | 1 échec pour 100, puis 1 pour 184 |
| Historique vide avant chaque partie | rouge 5 fois sur 5 |
| Historique d'une partie avant chaque partie | vert 5 fois sur 5 |

## Le correctif

Branche `shell-test-isolated-history` :

1. **875cf3d31 :** les deux fixtures prennent `-p <tmp_path>`, un historique neuf par test. Le test devient rouge à
   chaque fois (5 sur 5).
2. **c5af421d4 :** la remontée d'une ligne se fait par la flèche haute (`ESC [ A`), qui monte d'une ligne en mode
   multiligne avant de toucher l'historique.

Sur c5af421d4, la suite du shell passe entière (245 sur 245) et `test_prev_history_multiline --count=100` donne 200
passés pour 200.

**Reste à savoir :** d'autres tests lancent encore le shell sans `-p` (`ShellTest()` direct). La suite modifie donc
toujours `~/.rag3db/history.txt`, sans qu'aucun test en dépende aujourd'hui.

## Pour fermer

1. Un rouge reproductible : le test rejoué en boucle sous charge (par exemple `pytest --count=200`, avec un `stress`
   à côté), qui voit l'expiration au moins une fois.
2. Le correctif, dans le test (attendre l'écho de chaque touche avant la suivante) ou dans la lecture des séquences
   d'échappement du shell, puis la même boucle à zéro expiration.

## Fermeture (16 nov.)

- **Sur master** : 9b932226e (l'historique isolé par `-p tmp_path`, rouge 5 fois sur 5) et 08643d923 (la flèche haute à
  la place du troisième Ctrl-P), empilés sur le lot du journal des runs.
- **La suite du shell** passe entière à luciepc sur le lot (245 sur 245), et le test visé 100 fois de suite (200 passés
  sur 200, ses deux variantes comprises).
- **La CI** de master est verte depuis : le « CI Workflow » sur 9f0cbe7ce, et `rag3weaver` sur 457892de98
  (run 43782802364).
- **Reste ouvert, à part** : les ~130 `ShellTest()` sans `-p`, et un test qui crée `history.txt` dans le dossier
  courant (ticket `2026-11-16-la-suite-du-shell-ecrit-dans-l-historique-de-la-personne.md`, lot
  `shell-tests-own-history`, en attente d'empilement).
