# Un ForEachNode imbriqué ne rend pas ses données, et son contrôle de champs lit ceux du niveau d'en dessous

- **État** : corrigé le 15 novembre 2026 (session mémoire, 96). Ouvert le 14, contourné alors dans `confirmer_fiche`.
- **Famille** : produit
- **Gravité** : capacité manquante. Un geste en deux étapes dynamiques (lire, puis écrire avec la révision lue) ne s'écrit pas en graphes imbriqués. Les deux défauts se disent par une erreur, pas en silence.
- **Atteignable en service** : non. Tout gabarit qui imbrique un ForEachNode dans un graphe cité par un autre ForEachNode le rencontre.
- **Touche rag3weaver** : oui, `src/dataflow/foreach_node.rs` (`champs_manquants`, `appeler`).
- **Pour** : session mémoire (96), ou la session qui tient le dataflow.
- **Signalé par** : session mémoire (96), en écrivant `confirmer_fiche`, le 14 novembre 2026.

## Ce qu'on voyait

Deux erreurs, dans un graphe cité par un ForEachNode, lui-même cité par un autre :

1. « l'élément 0 n'a pas element.record (nœud « ecrire ») » : `champs_manquants` cherche la marque
   `$element.` dans toutes les chaînes de la configuration des nœuds, y compris le texte du graphe que
   cite le ForEachNode intérieur. Il prend les champs de l'élément intérieur pour ceux de l'élément
   extérieur. Contournement : l'option `element='ligne'` sur le ForEachNode intérieur.
2. « le résultat ecrire.results n'a pas été produit » : `appeler` ne lit qu'une sortie simple du graphe
   cité (`sortie.get(nœud, port)`). Le flux `results` d'un ForEachNode n'y est pas, et un niveau
   imbriqué ne fait pas remonter ce qu'il a écrit. Aucun nœud ne ramasse un flux en une valeur.

## Le correctif

- **`champs_manquants`** ne lit plus la clé `graph` d'un `GraphNode` ou d'un `ForEachNode`, c'est-à-dire
  le texte du graphe cité, dont les `$element.x` sont ceux d'un niveau plus bas. Les autres clés du
  nœud, ses paramètres, restent lues.
- **Le `ForEachNode` reçoit une sortie `list`** : les mêmes résultats que le flux `results`, en une
  liste posée à la fin, l'échec en `{index, error, element}`. Un graphe cité peut la rendre comme
  résultat (`%% result: <nœud>.list`), ce qu'un flux ne permet pas.

## Les témoins

- `foreach_node::le_controle_des_champs_ne_lit_pas_l_element_d_un_niveau_plus_bas` : un parent, un
  milieu et un bas. Le bas lit `$element.v`, et l'élément du milieu n'a pas de `v`.
- `foreach_node::un_niveau_imbrique_rend_la_liste_de_ses_resultats` : le milieu rend `bas.list`.

Les deux sont rouges sur master e8f6f7783, chacun avec l'erreur du ticket mot pour mot, et verts
après. `foreach_node` et `decide_nodes` : 40 sur 40. `e2e_archiviste` : 15 sur 15.
