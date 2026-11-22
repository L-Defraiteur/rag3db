# Un fil d'index écrit après la suppression du dossier d'un test

- **État** : ouvert (22 novembre 2026, 20 h 45), rag3weaver archi, à la demande de c7.
- **Famille** : moteur
- **Gravité** : intermittent. La suite e2e qui le subit rend le code 1 (« laisse des résidus dans son TMPDIR »), alors que
  tous ses tests passent. Elle masque donc un vrai rouge, ou fait refuser un lot qui n'y est pour rien.
- **Atteignable en service** : peut-être. Un index qu'on ferme peut garder un fil de fusion qui écrit encore après la
  fermeture. En service, le dossier ne disparaît pas sous lui, mais l'écriture tardive reste à prouver inoffensive.
- **Touche rag3weaver** : oui (la fermeture d'un index plein texte et ses fils de fond ; la dépendance lucivy).
- **Pour** : lucivy, pour la montée en 4.5.0 ; puis archi, pour vérifier que les résidus ne reviennent pas.
- **Signalé par** : la passe de judge-grant-said-2, à luciepc, le 22 novembre à 20 h 36.

## Ce qu'on voit

`run_e2e.sh --test e2e_tty`, sur master 634020301e avec judge-grant-said-2 : 4 tests sur 4 passent, puis le garde des
résidus rougit :

```
▸ RÉSIDUS e2e_tty : 2 entrée(s) dans son TMPDIR (1,2M)
    612K	.tmpc7bSH2
      42 o	data
      114 o	data/code-petit.rag3db.fts
      216 o	data/code-petit.rag3db.fts/Lucivy_ScopeContent
      …
✗ e2e_tty laisse des résidus dans son TMPDIR et n'est pas dans tests/e2e/suites_qui_laissent_des_residus.txt
```

Chaque `.tmpXXXX` est le dossier d'un test, déjà supprimé à la fin du test. Quelque chose y a recréé
`data/code-petit.rag3db.fts/Lucivy_*` après la suppression : un fil d'index qui écrit encore.

La même suite, jouée juste avant sur le même master avec prompt-roots-2, ne laissait aucun résidu : le défaut est
intermittent. Ni l'un ni l'autre lot ne touche à l'index (l'invite de l'équipier pour l'un, une ligne de boîte de la porte
pour l'autre).

Journal : luciepc `~/.cache/rag3weaver-build/judge-grant-said-2-tty/e2e_tty.log`.

## La piste

Le défaut des fusions en attente (`pending_merge_tasks`) est corrigé dans lucivy 4.5.0, alors que rag3weaver dépend
encore de lucivy 4.4.1. Une fusion lancée avant la fermeture écrirait ses segments après la suppression du dossier.
lucivy redépose la montée seule.

## Pour fermer

Après la montée en 4.5.0, jouer e2e_tty plusieurs fois de suite, sans résidu, puis fermer en le disant. Si les
résidus reviennent, chercher quel fil de fond survit à la fermeture du backend dans le test.
