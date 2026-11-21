# Relevé, scénario 1 : un dev qui connaît les lacunes, sur une copie d'un vrai projet

Session tty, nuit du 20 au 21 novembre 2026. Ce scénario passe par l'hôte de service de la tablette et par le terminal posés par G. La copie est celle du crate du terminal, en dépôt autonome (`~/.cache/rag3weaver-build/essais-21-11/projet-tty`).

## Ce qui se fait où

- **Ouvrir et fermer une vue** (`open_view`, `close_view`, `views`, `use_view`) : ce sont des outils du client MCP (`rag3weaver-backend mcp`), ceux qu'emploie un agent de code comme Claude Code. **Le terminal n'en a pas.** Au terminal, l'équivalent est la tâche : sa branche, son worktree, sa base copiée.

## L'incident de 01 h 29 : l'hôte posé sans `token-count`

L'hôte posé à 01 h 29 (fed132c964) a été bâti sans la feature `token-count`. Or le gabarit de code de sa propre tête découpe pour granite-278m.
- `rag3weaver-backend code-tree add <copie> --from templates/backends/code/backend.json`, comme le dit le README, répond « l'hôte http://127.0.0.1:7970 n'ouvre pas ce backend : le découpage pour granite-278m compte ses jetons, et ce bâti n'a pas le compteur : il faut la feature token-count ».
- **L'ajout écrit avant d'échouer** : `~/.config/rag3weaver/code-projet-tty/`, sa base, et un backend inscrit à l'hôte (76ad218634aa).
- **Le retrait échoue de même** : `code-tree remove code-projet-tty` veut ouvrir le backend. Le ménage est laissé à G.
- G a remis l'ancien hôte (83c667e420) à 01 h 33 aux deux postes. Le terminal posé, de fed132c964, reste. Ne pas forcer `--sans-hote` : l'hôte rebâti ouvrira `code-projet-tty`, qui se retirera alors par `code-tree remove`.
- c7 a arrêté le scénario. Selon lui, `search_code` et le chat échouaient aussi depuis 01 h 29 sur les bases sous granite. G remet l'ancien binaire, puis rebâtit avec les features de service et repose l'hôte et le terminal.

Constats pour le produit (les deux premiers en ticket, `2026-11-21-un-ajout-de-depot-ecrit-avant-de-savoir-ouvrir-et-ne-se-retire-plus`) :
- un ajout qui ne pourra pas ouvrir son backend ne devrait rien écrire ;
- un dépôt inscrit qu'on ne peut pas ouvrir devrait pouvoir se retirer ;
- la pose d'un hôte devrait vérifier que le binaire ouvre le gabarit de sa propre tête.

## Sur l'hôte rebâti (posé à 01 h 55)

1. **Retirer la copie restée inscrite** : `code-tree remove code-projet-tty` répond « fermé par l'hôte », puis « manifeste et base mis de côté sous …/code-projet-tty.retire-1795222609 ». **Bon.**
2. **Réinscrire la copie** par le chemin du README : accepté (id 76ad218634aa, nom `projet-tty`).
3. **Les vues, par le client MCP**, comme Claude Code (`rag3weaver-backend mcp --manifest …`), 21 outils :
   - `open_view {lot: "lire", session_dir: "tty-essais"}` crée `~/git_workspaces/tty-essais/21-11-2026-lire` sur `tty-essais/lire`. Le projet n'a pas de remote : la vue part de `HEAD` et le dit (`"from": "HEAD"`). Elle s'indexe et devient la vue active. **Bon.**
   - `use_view` refuse le nom que rend `open_view` (« vue « tty-essais/21-11-2026-lire » inconnue ; vues connues : backend, worktree:/…, fourche:… »). Sans gravité, puisque la vue était déjà active, mais trompeur.
   - `read_file` lit, mais sa réponse JSON porte `"cursor": "worktree:/…/projet-tty"`, l'arbre principal, et non la vue active. À vérifier : lit-il vraiment la vue ?
   - `search_code` sans `options` refuse sur « "options" is a required property ». Avec `options`, il trouve `resoudre_depot` premier, par le nom (`vector+bm25+bm25_exact`) comme par le sens (« où le terminal trouve le dépôt que l'utilisateur a demandé », `vector+bm25`). **Bon.**
   - **`edit_file` refuse d'écrire dans la vue** : « sur ce backend l'agent n'écrit pas l'arbre principal (task_writes) : ouvrez une tâche ». **Une vue ouverte par `open_view` est en lecture seule ; l'écriture passe par une tâche.**
   - `close_view` ferme : le worktree et son target partent. Restent la branche `tty-essais/lire` et une vue `fourche:…` dans `views`.
4. **Une tâche de bout en bout, au terminal posé** (`~/.local/bin/rag3weaver-tty`), tâche `essai-scenario-1` :
   - une demande en lecture seule : réponse juste (`src/client.rs:104-117`, ce que fait la fonction), arbre principal intact. **Bon.**
   - une consigne d'édition et de `cargo check` : la ligne est écrite sur la branche de la tâche (`agent/essai-scenario-1`), l'arbre principal reste intact. Mais **`cargo check` reste bloqué** environ huit minutes. L'équipier finit en « raté », et la tâche en « fait sans plan ». La cause n'est pas tranchée : la charge (15 à la tablette, `rag3db` en réouverture), ou la compilation à froid.
   - **Le message d'attente se trompe de cause** : pour `run_command` et `wait_output`, il dit « an index is probably holding the write ». Ce texte vient de mon lot du 19 novembre ; correction en lot cette nuit.
5. **L'aller-retour avec l'IDE**, joué avec recherche sur la page vivante de la tâche (`/ide/code/76ad218634aa/taches/essai-scenario-1`) :
   - la page montre la boîte, la demande, le report et le diff de la branche, au flux. **Bon.**
   - un tour écrit depuis la page paraît au terminal sous `[ide · demande]`, l'équipier le lit et y répond, et la réponse paraît des deux côtés. **Bon.**
   - une carte provoquée du terminal (`python3 -c "print(42)"`, hors de la liste sûre ; `cargo check`, lui, y était) : l'hôte la rend dans `waiting`, la page la montre, et le « Oui » de la page paraît au terminal sous `› accordé : python3 -c "print(42)"`, puis la commande part. **Bon.**

## En bref, pour la page du parcours

- **Marche** : inscrire un dépôt (sur un hôte qui a `token-count`) et le retirer ; ouvrir, lire, chercher (par le nom et par le sens) et fermer une vue par le client MCP ; une tâche au terminal, suivie et pilotée aussi depuis la page de l'IDE, tours et accords dans les deux sens.
- **Ne marche pas** : écrire dans une vue (l'écriture passe par une tâche) ; `use_view` avec le nom de `open_view` ; `cargo check` lent ou bloqué dans une tâche sous charge.
- **Contourner** : pour écrire, ouvrir une tâche ; pour activer une vue, prendre le nom `worktree:/…` que liste `views` (ou rien, `open_view` l'active).

## La suite

Faite ci-dessus, sur l'hôte rebâti.
