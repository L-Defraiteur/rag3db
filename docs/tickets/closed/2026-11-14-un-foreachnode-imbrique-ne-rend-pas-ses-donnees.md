# Un ForEachNode imbriqué ne rend pas ses données, et son contrôle de champs lit ceux du niveau d'en dessous

- **État** : ouvert, le 14 novembre 2026. Contourné dans `confirmer_fiche` (un seul niveau), pas corrigé.
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

## Ce qui reste à faire

- Que `champs_manquants` ne descende pas dans le texte d'un graphe cité par un ForEachNode intérieur.
- Que `appeler` ramasse en liste un résultat qui est un flux, ou qu'un nœud le fasse.
- Témoin rouge d'abord, pour chacun : un ForEachNode qui cite un graphe contenant un ForEachNode, dont le
  graphe intérieur lit `$element.x`, et dont le résultat est `intérieur.results`.
