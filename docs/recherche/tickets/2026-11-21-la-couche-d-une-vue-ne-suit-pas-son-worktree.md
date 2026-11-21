# La couche d'une vue ne suit pas son worktree après open_view

- **État** : ouvert (21 novembre 2026, session recherche, tri du fichier des ratés du MCP).
- **Gravité** : bloquant pour chercher dans son propre worktree (le critère de Lucie, 21 nov. 12 h 28)
- **Atteignable en service** : oui.
- **Touche rag3weaver** : oui (les couches de travail des vues, la montre).
- **Famille** : produit
- **Pour** : arbre principal (lot (a)/(b) de 2b-C)
- **Signalé par** : recherche et arbre, dans `~/.local/state/rag3weaver/mcp-misses.tsv`.

## Le pas exact

Une vue ouverte par `open_view`, son worktree réécrit ensuite sans commit (une fonction ajoutée, une retirée) :
`search_code` avec `view: {layer: <couche de la vue>}` ne trouve pas la fonction ajoutée, après 120 s (« trois_non_commise »,
témoin de recherche ; « quarante_deux_dans_la_vue », témoin d'arbre). La couche de la vue ne s'écrit qu'à l'ouverture.

## Le témoin

`daemon_host::view_search_tests::a_view_on_a_worktree_finds_its_uncommitted_function_and_not_its_deleted_one`, branche
`view-search-witness` (4f965148bf), rouge ; arbre en part pour son lot.

## Le contournement

Aucun par l'outil : chercher par `grep` dans son worktree.
