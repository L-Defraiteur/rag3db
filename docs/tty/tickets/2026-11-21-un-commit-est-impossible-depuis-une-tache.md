# Un commit est impossible depuis une tâche, et l'équipier tourne en rond sur ses droits

- **État** : ouvert le 21 novembre 2026 par la session tty, dans les essais de la nuit (scénario 2). Pour archi (c7), cette nuit, après l'identité git.
- **Famille** : produit
- **Gravité** : une demande « commite le tout » ne peut pas aboutir, et l'équipier dépense ses pas sur un refus du bac à sable.
- **Atteignable en service** : oui, à chaque tâche à qui l'on demande un commit.
- **Touche rag3weaver** : oui (le bac à sable des commandes de l'équipier, `run_command`).

## Ce que c'est

Dans la tâche `debut` du dépôt `compteur`, l'équipier crée trois fichiers, puis demande l'accord pour `git add README.md compteur.py test_compteur.py`. Accordée, la commande échoue :

    fatal : Impossible de créer '…/compteur/.git/worktrees/code/index.lock' : Permission non accordée

Le gitdir du worktree de la tâche vit dans le `.git` du dépôt, hors de ce que le bac à sable laisse écrire. L'équipier cherche alors à « résoudre les permissions » : il demande l'accord pour `ls -ld …/.git/worktrees/code/`, `whoami`, puis s'arrête sur « 3 pas de suite sans élément réussi ».

## Le pas qui bloque

Le `git add` de la tâche, accordé.

## Contournement

Ne pas demander de commit à l'équipier : la branche de la tâche se fusionne depuis l'IDE (« Fusionner »). Le dire dans la demande.

## Ce qu'il faut

Soit le commit d'une tâche se fait (le bac à sable laisse écrire le gitdir de son propre worktree), soit l'équipier sait qu'il ne commite pas : son outil le lui dit à la première tentative, et il s'arrête au lieu de chercher ses droits.
