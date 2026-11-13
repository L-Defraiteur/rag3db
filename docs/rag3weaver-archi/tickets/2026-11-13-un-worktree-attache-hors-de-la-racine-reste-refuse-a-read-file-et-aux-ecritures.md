# Un worktree attaché hors de la racine reste refusé à read_file et aux écritures

- **État** : ouvert (13 novembre 2026), pour archi.
- **Famille** : produit
- **Gravité** : capacité manquante, et un message faux — le refus dit qu'attacher le worktree suffit, ce qui n'est pas le cas
- **Atteignable en service** : oui, pour tout worktree attaché hors de `workspace.root` (un dépôt sous `~/git_workspaces` quand la racine est `~/rw-code`)
- **Touche rag3weaver** : oui (`extension/rag3weaver/src/daemon_host/atelier_fichiers.rs` : le test de racine 146, le message 148)
- **Pour** : rag3weaver archi (l'atelier qui lit et écrit, son auteur), lot d'après avec les six autres tickets de la relecture
- **Signalé par** : everything declarative, relecture de l'atelier sur origin/master 73c7f15e1 (13 novembre 2026), mis en ticket à la demande d'archi ; lignes de origin/master à cette date

## Ce que c'est

La résolution d'un chemin (`atelier_resoudre`) ne regarde que la racine de l'atelier, pas les
worktrees attachés à la conversation. Un worktree attaché hors de la racine est donc refusé à
`read_file` et aux écritures, alors que `read_scope` sans `path` le lit, et que le message de
refus conseille d'attacher le worktree.

## Le témoin à écrire

Un worktree attaché hors de la racine : `read_file` d'un de ses fichiers le rend, une écriture y
passe par le chemin du worktree (sa réindexation comprise), et un chemin hors de la racine et de
tout worktree attaché reste refusé.

## Ce qui le fermerait

Accepter, à la résolution, un chemin sous un worktree attaché à la conversation (après la
canonisation du correctif e79ecf6c0) ; ou corriger le message s'il est voulu que seul
l'accord ouvre ce chemin ; le témoin vert.
