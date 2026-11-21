# Relevé, scénario 1 : un dev qui connaît les lacunes, sur une copie d'un vrai projet

Session tty, nuit du 20 au 21 novembre 2026. Ce scénario passe par l'hôte de service de la tablette et par le terminal posés par G. La copie est celle du crate du terminal, en dépôt autonome (`~/.cache/rag3weaver-build/essais-21-11/projet-tty`).

## Ce qui se fait où

- **Ouvrir et fermer une vue** (`open_view`, `close_view`, `views`, `use_view`) : ce sont des outils du client MCP (`rag3weaver-backend mcp`), ceux qu'emploie un agent de code comme Claude Code. **Le terminal n'en a pas.** Au terminal, l'équivalent est la tâche : sa branche, son worktree, sa base copiée.

## L'incident de 01 h 29 : l'hôte posé sans `token-count`

L'hôte posé à 01 h 29 (fed132c964) a été bâti sans la feature `token-count`. Or le gabarit de code de sa propre tête découpe pour granite-278m.
- `rag3weaver-backend code-tree add <copie> --from templates/backends/code/backend.json`, comme le dit le README, répond « l'hôte http://127.0.0.1:7970 n'ouvre pas ce backend : le découpage pour granite-278m compte ses jetons, et ce bâti n'a pas le compteur : il faut la feature token-count ».
- **L'ajout écrit avant d'échouer** : `~/.config/rag3weaver/code-projet-tty/`, sa base, et un backend inscrit à l'hôte (76ad218634aa).
- **Le retrait échoue de même** : `code-tree remove code-projet-tty` veut ouvrir le backend. Le ménage est laissé à G.
- c7 a arrêté le scénario. Selon lui, `search_code` et le chat échouaient aussi depuis 01 h 29 sur les bases sous granite. G remet l'ancien binaire, puis rebâtit avec les features de service et repose l'hôte et le terminal.

Constats pour le produit :
- un ajout qui ne pourra pas ouvrir son backend ne devrait rien écrire ;
- un dépôt inscrit qu'on ne peut pas ouvrir devrait pouvoir se retirer ;
- la pose d'un hôte devrait vérifier que le binaire ouvre le gabarit de sa propre tête.

## La suite

À reprendre sur l'hôte rebâti : la vue par le client MCP (ouvrir, lire, chercher par le nom et par le sens, éditer dans la vue, fermer), puis une tâche de bout en bout au terminal, avec ses accords et ce que montre sa page vivante.
