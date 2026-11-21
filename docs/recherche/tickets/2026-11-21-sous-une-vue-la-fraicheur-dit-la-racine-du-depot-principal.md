# Sous une vue, la fraîcheur d'une réponse dit la racine du dépôt principal

- **État** : ouvert (21 novembre 2026, session recherche, tri du fichier des ratés du MCP).
- **Gravité** : trompe le lecteur
- **Atteignable en service** : oui.
- **Touche rag3weaver** : oui (la fraîcheur des réponses des outils de lecture).
- **Famille** : produit
- **Pour** : arbre principal
- **Signalé par** : arbre (témoin view-search), dans `~/.local/state/rag3weaver/mcp-misses.tsv`.

## Le pas exact

`search_code` sous `view: {layer: <couche d'une vue>}` rend `freshness.root` = la racine du dépôt principal
(`…/B/rag3db-essai`) et la branche du dépôt, pas le worktree de la vue ni sa branche. Le lecteur croit lire l'état du
dépôt principal.

## Pour le fermer

Sous une vue, `freshness` dit la couche lue, son worktree et sa branche (ou les ajoute à côté de la racine du dépôt).
