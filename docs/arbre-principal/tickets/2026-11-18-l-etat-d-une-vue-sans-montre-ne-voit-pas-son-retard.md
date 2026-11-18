# L'état d'une vue sans montre ne voit pas son retard

- **État** : ouvert (18 novembre 2026), arbre principal ; la part de `ViewState` laissée hors du lot 2 de l'état
  d'index par vue (branche `arbre-principal/view-state`).
- **Famille** : produit
- **Gravité** : une vue dont la couche n'a pas de montre se dit à jour même quand ses fichiers ont changé depuis
  son index. C'est le comportement d'aujourd'hui, par cellule : rien ne régresse, mais rien ne le dit.
- **Atteignable en service** : oui, pour un backend sans montre déclarée.
- **Touche rag3weaver** : oui.

## Ce que c'est

`views::view_state` lit `files_behind` dans `Lag`, pour la couche de la vue. Seule la montre y note, y compris par
sa réconciliation de départ. Sans montre, rien n'y est noté : `files_behind` vaut 0 et `up_to_date` est vrai.

L'accord avec recherche, le 18 novembre, prévoyait deux sources :

- le compte de `Lag` quand la montre de la couche tourne ;
- un diff d'arbres seulement à froid, sans montre : l'arbre noté de la couche (`indexed_tree`, racine, commit et
  état sale) contre l'arbre de sa pointe.

## Ce qu'il faut pour le fermer

- La marque d'arbre indexé **par couche** : aujourd'hui, `indexed_tree:<org>/<project>` est par cellule.
- `Lag` sait si une montre sert la couche.
- `view_state` fait le diff quand aucune montre ne sert la couche, hors du verrou du catalogue (`git` coûte), avec
  un résultat gardé tant que la pointe ne bouge pas.
- Un témoin rouge d'abord : une couche sans montre, un fichier changé depuis son index, la vue ne se dit pas à jour.
