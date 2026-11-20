# Une valeur de `reasoning` que le fournisseur refuse ne se découvre qu'à l'appel

- **État** : ouvert, le 20 novembre 2026.
- **Famille** : produit
- **Gravité** : un réglage déclaré qui ne marche pas n'est dit qu'au premier appel, en pleine passe.
- **Atteignable en service** : oui. Tout graphe ou tout modèle qui déclare `reasoning`.
- **Touche rag3weaver** : oui. Le chargement des outils (`LlmNode`, son paramètre `reasoning`) et la déclaration
  des modèles (`models.llm[].reasoning`).
- **Pour** : session mémoire (96), avec archi pour le modèle de l'équipier.
- **Signalé par** : le banc (docs/banc/20-11-2026/02) et c7, le 20 novembre 2026.

## Ce qu'on voit

`reasoning` admet `minimal`, `low`, `medium` et `high` (`crate::llm::ReasoningEffort`). Sur Vertex, avec Gemini
3.8 Flash, `minimal` est refusé à l'appel : le banc l'a vu en essayant `extract_reasoning=minimal` sur
`archive_slice`. Le gabarit se charge sans rien dire, et l'erreur n'arrive qu'à la première extraction.

## Ce qu'il faut

Une valeur que le fournisseur du modèle refuse est refusée **à la déclaration**, au chargement de l'outil ou du
modèle, avec la liste des valeurs admises **par fournisseur**. La liste est une donnée par fournisseur (et par
modèle si besoin), pas une règle en dur dans le nœud. Le témoin rouge d'abord : un gabarit qui déclare
`reasoning: minimal` sur un modèle Vertex qui le refuse ne se charge pas, et le refus nomme le paramètre, le
fournisseur et les valeurs admises.
