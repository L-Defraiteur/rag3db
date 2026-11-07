# `code-tree add --from` suppose un manifeste nommé backend.json

- **État** : ouvert (7 novembre 2026), pour everything declarative — correctif court.
- **Famille** : produit
- **Gravité** : blocage (on ne peut pas inscrire un worktree frère depuis la configuration réelle de Lucie).
- **Atteignable en service** : oui, sur toute configuration dont le manifeste ne s'appelle pas `backend.json` (celui de Lucie s'appelle `rag3db.json` depuis le 11 octobre).
- **Touche rag3weaver** : oui (`src/code_tree.rs`).

## Ce que c'est

c7, le 7 novembre : `rag3weaver-backend code-tree add ~/git_workspaces/rag3db-essais-c7 --from
code-rag3db --as code-essais-c7` échoue : « manifeste
~/.config/rag3weaver/code-rag3db/templates/backends/code/backend.json illisible : No such file ».
`Roots::manifest` (`code_tree.rs:42-43`) pose le nom du manifeste en dur
(`templates/backends/code/backend.json`) ; le manifeste réel est `…/rag3db.json`.

## Ce qu'il faut pour le fermer

`--from` accepte un nom d'arbre **ou** un chemin de manifeste ; par nom, il demande d'abord à
l'hôte le manifeste qu'il a enregistré pour cet arbre (`/ide/etat` rend `manifest` par backend), puis
cherche sous `templates/backends/code/` le seul `.json` qui est un manifeste, et ne retombe sur
`backend.json` qu'en dernier ; l'erreur nomme ce qu'elle a essayé. Le gabarit copié reste
`templates/` (le premier ancêtre de ce nom). Témoin : un arbre dont le manifeste s'appelle
`rag3db.json`, `--from` par son nom et par son chemin, l'arbre neuf s'inscrit.
