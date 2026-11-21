# Les vues de `open_view` : on n'y écrit pas, `use_view` refuse leur nom, la lecture dit l'arbre principal

- **État** : ouvert le 21 novembre 2026 par la session tty, dans les essais de la nuit (scénario 1). Pour arbre (c7), dans cet ordre : le 3 d'abord, puis le 1, le 2 et le 4.
- **Famille** : produit
- **Gravité** : le 3 peut être une réponse fausse (une lecture de l'arbre principal sous une vue active) ; les autres gênent le parcours d'une vue.
- **Atteignable en service** : oui, pour tout agent qui ouvre une vue par le client MCP.
- **Touche rag3weaver** : oui (`open_view`, `use_view`, `close_view`, `views`, `read_file`, `edit_file`).

## Ce que c'est

Par `rag3weaver-backend mcp`, sur la copie `projet-tty` (id 76ad218634aa) de l'hôte de service de la tablette, le 21 novembre vers 2 h :

3. **La lecture sous une vue active dit l'arbre principal.** Après `open_view {lot: "lire", session_dir: "tty-essais"}`, `views` donne pour active `worktree:/home/lucied/git_workspaces/tty-essais/21-11-2026-lire`. Pourtant, `read_file {path: "README.md", format: "json"}` rend `"cursor": "worktree:/home/lucied/.cache/rag3weaver-build/essais-21-11/projet-tty"`, l'arbre principal. Le contenu des deux était identique : rien ne dit lequel a été lu.
1. **On n'écrit pas dans une vue.** `edit_file` sur README.md, la vue active, répond « « edit_file » écrit les fichiers, et sur ce backend l'agent n'écrit pas l'arbre principal (task_writes) : ouvrez une tâche ». Ni la vue ni l'arbre principal ne changent.
2. **`use_view` refuse le nom que rend `open_view`.** `open_view` rend `"name": "tty-essais/21-11-2026-lire"` ; `use_view {name: "tty-essais/21-11-2026-lire"}` répond « vue « tty-essais/21-11-2026-lire » inconnue de ce backend ; vues connues : backend, worktree:/…/21-11-2026-lire, fourche:a061059… ».
4. **`close_view` laisse des restes.** Le worktree et son target partent, mais la branche `tty-essais/lire` reste dans le dépôt, et `views` liste encore `fourche:a06105957b…`.

## Contournement

- Pour écrire : ouvrir une tâche (sa branche `agent/<nom>`).
- Pour activer une vue : rien à faire, `open_view` l'active ; sinon, le nom `worktree:/…` que liste `views`.

## Ce qu'il faut

- (3) une lecture sous une vue active lit la vue, et sa réponse le dit (`cursor`, `view`) ;
- (1) soit `edit_file` écrit dans la vue ouverte par la session, soit `open_view` dit qu'elle est en lecture seule ;
- (2) `use_view` accepte le nom que rend `open_view` ;
- (4) `close_view` dit ce qu'il laisse (la branche), et ne liste plus la fourche.
