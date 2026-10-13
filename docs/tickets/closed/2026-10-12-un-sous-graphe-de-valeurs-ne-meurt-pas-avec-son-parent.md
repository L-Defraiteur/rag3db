# Un sous-graphe de valeurs ne meurt pas avec son parent par niveaux

- **État** : corrigé (12 octobre 2026, session recherche, dans le lot de l'annulation sous tous les exécuteurs, page `extension/rag3weaver/docs/12-octobre-2026-recherche/05-l-annulation-sous-tous-les-executeurs.md`)
- **Gravité** : blocage : un sous-graphe continue après l'arrêt de son parent
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (`src/dataflow`)
- **Ouvert le** : 12 octobre 2026, session recherche, par le relevé de la page 05 ; ouvert et fermé dans le même commit

## Ce que c'était

`graph_node.rs` accroche la poignée d'arrêt du runtime intérieur au jeton du parent (`on_cancel`). Sous un parent par niveaux, l'accroche ne se déclenchait jamais, et le sous-graphe, lui-même par niveaux, ignorait sa poignée.

## La cause

Le jeton d'annulation d'un nœud n'était fait que du hub de sa liaison de flux, absente sous l'exécuteur par niveaux : `on_cancel` n'y faisait rien et `is_cancelled` y restait faux.

## Le témoin

`annulation_tests.rs`, `a_value_subgraph_dies_with_its_levels_parent` : le nœud intérieur attendait ses 5 s, il se réveille désormais à l'annulation du parent.
