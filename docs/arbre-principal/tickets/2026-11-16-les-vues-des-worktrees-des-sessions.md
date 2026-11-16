# Les vues des worktrees des sessions : le nom court et la vue courante par le contexte

- **État** : ouvert (16 novembre 2026), arbre principal ; après 2a′-3.
- **Famille** : produit
- **Gravité** : Algareth (vision du 16 novembre, point 5) doit chercher « dans le worktree de la session X » sans le
  reproduire. Aujourd'hui une vue se nomme par l'id ou le nom de couche (`view: {layer: "worktree:/chemin"}`), et
  `context.view` est refusé (« vue contextuelle non disponible »).
- **Atteignable en service** : oui, pour tout agent qui appelle un outil de lecture.
- **Touche rag3weaver** : oui (`src/backend.rs` : `call_tool_on`, `CALL_VIEW` ligne ~3569, `load_call_view_stack` ;
  `src/daemon_host/invocation.rs:36`).
- **Pour** : arbre principal (la vue courante) et recherche (le sucre de nom, à la résolution) ; à faire par Codex.
- **Signalé par** : c7 et arbre principal, page `docs/orchestration/visions/16-11-2026-algareth-pair-des-sessions-claude.md`.

## Le correctif attendu

1. **Le nom court** : `view: {"worktree": "<chemin>"}` (et le nom d'une tâche) se résout en `worktree:<chemin réel>`
   dans la résolution de `view` de `call_tool_on`, au même endroit que `layer`. La réponse dit la vue lue par son
   id, sa couche, son rang. Un worktree hors du dépôt se refuse en se nommant.
2. **La vue courante par le contexte** : quand `view` est absent et que l'invocation porte `context.view`,
   `CALL_VIEW` se remplit depuis elle (au lieu du refus de `invocation.rs:36`). Aucune vue implicite sans contexte.

## Ce qu'il faut pour le fermer

Témoins rouges d'abord, dans `src/backend.rs` (module de tests) : une recherche sous `view: {worktree: <test>}` voit
le fichier non commité de ce worktree et pas celui du backend ; une invocation avec `context.view` lit cette vue
sans `view` dans les arguments. Bloqué par 2a′-3 (aucune couche de worktree n'a de génération avant lui).
