# Un plan de gabarits ignore `search_graphs` et `graph_tool`, qui changent les outils

- **État** : corrigé le 13 novembre 2026, dans le lot qui l'ouvre (session mémoire, 96). Relevé par
  everything declarative en relisant la première tranche de M8.
- **Famille** : produit
- **Gravité** : réponse fausse en silence. Le diff des outils dit « rien d'ajouté » alors que
  l'agent voit des outils de plus.
- **Atteignable en service** : non : la voie fermée de M8 n'a pas encore d'appelant en service.
- **Touche rag3weaver** : oui, `src/template_plan.rs` (`compatibility`).

## Ce qu'on voyait

Le diff des outils ne lisait que `tools`. `search_graphs` et `graph_tool` ajoutent des outils que
voit l'agent (`backend.rs`) : `graph_tool: true` donnait `tools.added == []`, et le plan passait.

## Le correctif

Changer `search_graphs` ou `graph_tool` (absent vaut faux) est refusé, comme compatibilité à
qualifier ; le diff des outils sur le backend chargé reste une piste pour une tranche suivante.
Témoin : `turning_on_graph_tool_or_search_graphs_is_not_a_silent_tool_change`, rouge sur
dd8111736 (plan Ok), vert après.

## Restent des doutes, non corrigés ici

D'autres clés passent sans mention : `declarations` (de `approbation` à `auto` change le pouvoir de
l'agent), `teammate`, `bilans`, `scripts`, `watches`, `routes`, `reactions`, `ui_state`,
`buffer_pool`. Les builtin fournis sont aussi lisibles comme fichiers sous `<racine>/.builtin/…`
(espaces de noms mêlés), et un `builtin:` inconnu perd l'exemple de son message d'erreur.
