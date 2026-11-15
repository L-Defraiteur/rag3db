# Un test du shell expire par intermittence : test_prev_history_multiline[multiline]

- **État** : ouvert (15 novembre 2026, hotfixs, relevé par tty)
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

## Pour fermer

1. Un rouge reproductible : le test rejoué en boucle sous charge (par exemple `pytest --count=200`, avec un `stress`
   à côté), qui voit l'expiration au moins une fois.
2. Le correctif, dans le test (attendre l'écho de chaque touche avant la suivante) ou dans la lecture des séquences
   d'échappement du shell, puis la même boucle à zéro expiration.
